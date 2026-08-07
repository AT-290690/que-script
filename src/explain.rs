use crate::infer::{EffectFlags, TypedExpression};
use crate::lsp_native_core::normalize_signature;
use crate::parser::Expression;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct ExplainReport {
    pub result_type: String,
    pub effect: Vec<String>,
    pub host_imports: Vec<String>,
    pub metrics: ExplainMetrics,
    pub forms: Vec<ExplainForm>,
    pub warnings: Vec<ExplainWarning>,
}

#[derive(Debug, Default, Serialize)]
pub struct ExplainMetrics {
    pub vector_allocations: usize,
    pub zeroed_vector_allocations: usize,
    pub uninit_vector_allocations: usize,
    pub tuple_allocations: usize,
    pub closure_allocations: usize,
    pub dynamic_apply_calls: usize,
    pub checked_vector_gets: usize,
    pub unchecked_vector_gets: usize,
    pub direct_user_function_calls: usize,
    pub wat_bytes: usize,
}

#[derive(Debug, Serialize)]
pub struct ExplainForm {
    pub name: String,
    pub kind: String,
    pub typ: String,
    pub effect: Vec<String>,
    pub calls: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct ExplainWarning {
    pub kind: String,
    pub message: String,
    pub suggestion: Option<String>,
}

pub fn explain_program(
    typed_ast: &TypedExpression,
    wat: &str,
    user_form_count: usize,
) -> ExplainReport {
    let metrics = collect_wat_metrics(wat);
    let host_imports = collect_host_imports(wat);
    let forms = collect_user_forms(typed_ast, user_form_count);
    let user_effect = user_form_nodes(typed_ast, user_form_count)
        .into_iter()
        .fold(EffectFlags::PURE, |acc, form| acc | form.effect);
    let result_type = user_form_nodes(typed_ast, user_form_count)
        .last()
        .and_then(|form| form.typ.as_ref())
        .or(typed_ast.typ.as_ref())
        .map(|typ| normalize_signature(&typ.to_string()))
        .unwrap_or_else(|| "_".to_string());
    let mut warnings = Vec::new();

    if metrics.dynamic_apply_calls > 0 {
        warnings.push(ExplainWarning {
            kind: "dynamic_apply".to_string(),
            message: format!(
                "{} dynamic apply call(s) remain in generated WAT",
                metrics.dynamic_apply_calls
            ),
            suggestion: Some(
                "Prefer direct function calls or let-bound local lambdas when performance matters."
                    .to_string(),
            ),
        });
    }
    if metrics.closure_allocations > 0 {
        warnings.push(ExplainWarning {
            kind: "closure_allocation".to_string(),
            message: format!(
                "{} closure allocation call(s) remain in generated WAT",
                metrics.closure_allocations
            ),
            suggestion: Some(
                "Avoid returning/storing partially applied functions in hot paths.".to_string(),
            ),
        });
    }
    if metrics.tuple_allocations > 0 {
        warnings.push(ExplainWarning {
            kind: "tuple_allocation".to_string(),
            message: format!(
                "{} tuple allocation call(s) remain in generated WAT",
                metrics.tuple_allocations
            ),
            suggestion: Some(
                "Destructure tuple-returning helpers immediately in hot paths where possible."
                    .to_string(),
            ),
        });
    }
    if metrics.checked_vector_gets > 0 {
        warnings.push(ExplainWarning {
            kind: "checked_vector_get".to_string(),
            message: format!(
                "{} checked vector get call(s) remain",
                metrics.checked_vector_gets
            ),
            suggestion: Some(
                "Use simple counted loops over cached lengths to help bounds-check elimination."
                    .to_string(),
            ),
        });
    }
    if !host_imports.is_empty() {
        warnings.push(ExplainWarning {
            kind: "host_imports".to_string(),
            message: format!(
                "program imports host functions: {}",
                host_imports.join(", ")
            ),
            suggestion: Some("Run with the matching --allow permissions.".to_string()),
        });
    }

    ExplainReport {
        result_type,
        effect: effect_labels(user_effect),
        host_imports,
        metrics,
        forms,
        warnings,
    }
}

pub fn render_text(report: &ExplainReport) -> String {
    let mut lines = Vec::new();
    lines.push("Que Explain".to_string());
    lines.push(format!("Result type: {}", report.result_type));
    lines.push(format!("Effect: {}", format_labels(&report.effect)));
    lines.push(format!(
        "Host imports: {}",
        if report.host_imports.is_empty() {
            "none".to_string()
        } else {
            report.host_imports.join(", ")
        }
    ));
    lines.push(String::new());
    lines.push("WAT shape:".to_string());
    lines.push(format!(
        "  vector allocations: {}",
        report.metrics.vector_allocations
    ));
    lines.push(format!(
        "  zeroed vector allocations: {}",
        report.metrics.zeroed_vector_allocations
    ));
    lines.push(format!(
        "  uninit vector allocations: {}",
        report.metrics.uninit_vector_allocations
    ));
    lines.push(format!(
        "  tuple allocations: {}",
        report.metrics.tuple_allocations
    ));
    lines.push(format!(
        "  closure allocations: {}",
        report.metrics.closure_allocations
    ));
    lines.push(format!(
        "  dynamic apply calls: {}",
        report.metrics.dynamic_apply_calls
    ));
    lines.push(format!(
        "  checked vector gets: {}",
        report.metrics.checked_vector_gets
    ));
    lines.push(format!(
        "  unchecked vector gets: {}",
        report.metrics.unchecked_vector_gets
    ));
    lines.push(format!(
        "  direct user function calls: {}",
        report.metrics.direct_user_function_calls
    ));
    lines.push(format!("  wat bytes: {}", report.metrics.wat_bytes));

    if !report.forms.is_empty() {
        lines.push(String::new());
        lines.push("User forms:".to_string());
        for form in &report.forms {
            lines.push(format!(
                "  {} {} : {} [{}]",
                form.kind,
                form.name,
                form.typ,
                format_labels(&form.effect)
            ));
            if !form.calls.is_empty() {
                lines.push(format!("    calls: {}", form.calls.join(", ")));
            }
        }
    }

    if !report.warnings.is_empty() {
        lines.push(String::new());
        lines.push("Warnings:".to_string());
        for warning in &report.warnings {
            lines.push(format!("  {}: {}", warning.kind, warning.message));
            if let Some(suggestion) = &warning.suggestion {
                lines.push(format!("    suggestion: {}", suggestion));
            }
        }
    }

    lines.join("\n")
}

pub fn render_json(report: &ExplainReport) -> Result<String, String> {
    serde_json::to_string_pretty(report)
        .map_err(|e| format!("failed to render explain json: {}", e))
}

fn collect_user_forms(typed: &TypedExpression, user_form_count: usize) -> Vec<ExplainForm> {
    user_form_nodes(typed, user_form_count)
        .into_iter()
        .enumerate()
        .map(|(idx, form)| {
            let (name, kind, typ) = describe_form(idx, form);
            let mut calls = Vec::new();
            collect_calls(&form.expr, &mut calls);
            calls.sort();
            calls.dedup();
            ExplainForm {
                name,
                kind,
                typ,
                effect: effect_labels(form.effect),
                calls,
            }
        })
        .collect()
}

fn user_form_nodes<'a>(
    typed: &'a TypedExpression,
    user_form_count: usize,
) -> Vec<&'a TypedExpression> {
    if let Expression::Apply(_) = &typed.expr {
        if typed.children.len() > 1 {
            let forms = &typed.children[1..];
            let start = forms.len().saturating_sub(user_form_count);
            return forms[start..].iter().collect();
        }
    }
    vec![typed]
}

fn describe_form(idx: usize, form: &TypedExpression) -> (String, String, String) {
    if let Expression::Apply(items) = &form.expr {
        if items.len() >= 3 {
            if let (Some(Expression::Word(kw)), Some(Expression::Word(name))) =
                (items.first(), items.get(1))
            {
                if kw == "let" || kw == "letrec" || kw == "mut" {
                    let typ = form
                        .children
                        .get(2)
                        .and_then(|child| child.typ.as_ref())
                        .or(form.typ.as_ref())
                        .map(|typ| normalize_signature(&typ.to_string()))
                        .unwrap_or_else(|| "_".to_string());
                    return (name.clone(), kw.clone(), typ);
                }
            }
        }
    }
    let typ = form
        .typ
        .as_ref()
        .map(|typ| normalize_signature(&typ.to_string()))
        .unwrap_or_else(|| "_".to_string());
    (format!("form[{}]", idx), "expr".to_string(), typ)
}

fn collect_calls(expr: &Expression, calls: &mut Vec<String>) {
    match expr {
        Expression::Apply(items) => {
            if let Some(Expression::Word(head)) = items.first() {
                if !is_special_form_or_literal_constructor(head) {
                    calls.push(head.clone());
                }
            }
            for item in items {
                collect_calls(item, calls);
            }
        }
        Expression::Int(_) | Expression::Dec(_) | Expression::Word(_) => {}
    }
}

fn is_special_form_or_literal_constructor(name: &str) -> bool {
    matches!(
        name,
        "do" | "block"
            | "let"
            | "letrec"
            | "lambda"
            | "if"
            | "cond"
            | "while"
            | "mut"
            | "alter!"
            | "vector"
            | "tuple"
            | "string"
    )
}

fn collect_wat_metrics(wat: &str) -> ExplainMetrics {
    let metric_wat = user_metric_wat(wat);
    let wat = metric_wat.as_str();
    ExplainMetrics {
        vector_allocations: count_occurrences(wat, "call $vec_new_i32"),
        zeroed_vector_allocations: count_occurrences(wat, "call $vec_new_zeroed_i32"),
        uninit_vector_allocations: count_occurrences(wat, "call $vec_new_uninit_i32"),
        tuple_allocations: count_occurrences(wat, "call $tuple_new"),
        closure_allocations: count_occurrences(wat, "call $closure_new"),
        dynamic_apply_calls: count_occurrences(wat, "call $apply0_i32")
            + count_occurrences(wat, "call $apply1_i32")
            + count_occurrences(wat, "call $apply2_i32")
            + count_occurrences(wat, "call $apply3_i32"),
        checked_vector_gets: count_occurrences(wat, "call $vec_get_i32"),
        unchecked_vector_gets: count_occurrences(wat, "i32.load"),
        direct_user_function_calls: count_prefixed_calls(wat, "call $v_"),
        wat_bytes: wat.len(),
    }
}

fn user_metric_wat(wat: &str) -> String {
    let lines = wat.lines().collect::<Vec<_>>();
    let mut out = Vec::new();
    let mut i = 0usize;
    while i < lines.len() {
        let trimmed = lines[i].trim_start();
        if trimmed.starts_with("(func (export \"main\")") || trimmed.starts_with("(func $v_") {
            let mut depth = 0i32;
            while i < lines.len() {
                let line = lines[i];
                depth += line.matches('(').count() as i32;
                depth -= line.matches(')').count() as i32;
                out.push(line);
                i += 1;
                if depth <= 0 {
                    break;
                }
            }
            continue;
        }
        i += 1;
    }
    out.join("\n")
}

fn collect_host_imports(wat: &str) -> Vec<String> {
    let mut imports = Vec::new();
    for line in wat.lines() {
        let line = line.trim();
        if !line.starts_with("(import \"host\"") {
            continue;
        }
        let parts = line.split('"').collect::<Vec<_>>();
        if parts.len() >= 4 {
            imports.push(parts[3].to_string());
        }
    }
    imports.sort();
    imports.dedup();
    imports
}

fn count_occurrences(haystack: &str, needle: &str) -> usize {
    haystack.match_indices(needle).count()
}

fn count_prefixed_calls(wat: &str, prefix: &str) -> usize {
    wat.lines()
        .filter(|line| line.trim_start().starts_with(prefix))
        .count()
}

fn effect_labels(effect: EffectFlags) -> Vec<String> {
    let mut labels = Vec::new();
    if effect.is_pure() {
        labels.push("pure".to_string());
        return labels;
    }
    if effect.contains(EffectFlags::MUTATE) {
        labels.push("mutate".to_string());
    }
    if effect.contains(EffectFlags::IO) {
        labels.push("io".to_string());
    }
    if effect.contains(EffectFlags::UNKNOWN_CALL) {
        labels.push("unknown-call".to_string());
    }
    labels
}

fn format_labels(labels: &[String]) -> String {
    if labels.is_empty() {
        "none".to_string()
    } else {
        labels.join(", ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn explain_source(src: &str) -> ExplainReport {
        let expr = crate::parser::build(src).expect("source should build");
        let wrapped = crate::externals::prepend_builtin_host_externs(&expr)
            .expect("host externs should prepend");
        let (_typ, typed) = crate::infer::infer_with_builtins_typed(
            &wrapped,
            crate::types::create_builtin_environment(crate::types::TypeEnv::new()),
        )
        .expect("source should infer");
        let split =
            crate::wat::compile_program_to_split_wat_typed(&typed).expect("source should compile");
        explain_program(&typed, &split.user_wat, 1)
    }

    #[test]
    fn explain_reports_basic_shape_as_text() {
        let report = explain_source("(+ 1 2)");
        assert_eq!(report.result_type, "Int");
        let text = render_text(&report);
        assert!(text.contains("Que Explain"));
        assert!(text.contains("Result type: Int"));
        assert!(text.contains("WAT shape:"));
    }

    #[test]
    fn explain_reports_json_shape() {
        let report = explain_source("(+ 1 2)");
        let json = render_json(&report).expect("json should render");
        assert!(json.contains("\"result_type\": \"Int\""));
        assert!(json.contains("\"metrics\""));
    }
}
