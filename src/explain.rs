use crate::infer::{EffectFlags, TypedExpression};
use crate::lsp_native_core::{normalize_signature, refine_effect_with_known_calls};
use crate::parser::Expression;
use serde::Serialize;
use std::collections::HashMap;

#[derive(Debug, Serialize)]
pub struct ExplainReport {
    pub result_type: String,
    pub effect: Vec<String>,
    pub host_imports: Vec<String>,
    pub metrics: ExplainMetrics,
    pub optimized_user_calls: Vec<String>,
    pub compiled_functions: Vec<ExplainCompiledFunction>,
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
pub struct ExplainCompiledFunction {
    pub name: String,
    pub wat_name: String,
    pub metrics: ExplainMetrics,
    pub calls: Vec<ExplainCall>,
}

#[derive(Debug, Serialize)]
pub struct ExplainCall {
    pub name: String,
    pub kind: String,
    pub count: usize,
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
    explain_program_with_effects(typed_ast, wat, user_form_count, &HashMap::new())
}

pub fn explain_program_with_effects(
    typed_ast: &TypedExpression,
    wat: &str,
    user_form_count: usize,
    known_effects: &HashMap<String, EffectFlags>,
) -> ExplainReport {
    let metrics = collect_wat_metrics(wat);
    let host_imports = collect_host_imports(wat);
    let optimized_user_calls = collect_prefixed_call_targets(&user_metric_wat(wat), "call $v_");
    let compiled_functions = collect_compiled_functions(wat);
    let user_nodes = user_form_nodes(typed_ast, user_form_count);
    let effect_scope = collect_user_effect_scope(&user_nodes, known_effects);
    let forms = collect_user_forms(&user_nodes, &effect_scope);
    let user_effect = user_nodes.into_iter().fold(EffectFlags::PURE, |acc, form| {
        acc | refined_form_effect(form, &effect_scope)
    });
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
        optimized_user_calls,
        compiled_functions,
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

    if !report.optimized_user_calls.is_empty() {
        lines.push(format!(
            "  optimized user calls: {}",
            report.optimized_user_calls.join(", ")
        ));
    }

    let interesting_functions = report
        .compiled_functions
        .iter()
        .filter(|function| function_has_interesting_explain_details(function))
        .collect::<Vec<_>>();
    if !interesting_functions.is_empty() {
        lines.push(String::new());
        lines.push("Compiled function details:".to_string());
        for function in interesting_functions {
            lines.push(format!("  {}:", function.name));
            let mut detail_parts = Vec::new();
            if function.metrics.vector_allocations > 0 {
                detail_parts.push(format!("vector {}", function.metrics.vector_allocations));
            }
            if function.metrics.zeroed_vector_allocations > 0 {
                detail_parts.push(format!(
                    "zeroed-vector {}",
                    function.metrics.zeroed_vector_allocations
                ));
            }
            if function.metrics.uninit_vector_allocations > 0 {
                detail_parts.push(format!(
                    "uninit-vector {}",
                    function.metrics.uninit_vector_allocations
                ));
            }
            if function.metrics.tuple_allocations > 0 {
                detail_parts.push(format!("tuple {}", function.metrics.tuple_allocations));
            }
            if function.metrics.closure_allocations > 0 {
                detail_parts.push(format!("closure {}", function.metrics.closure_allocations));
            }
            if !detail_parts.is_empty() {
                lines.push(format!("    allocations: {}", detail_parts.join(", ")));
            }
            if function.metrics.dynamic_apply_calls > 0 {
                lines.push(format!(
                    "    dynamic apply calls: {}",
                    function.metrics.dynamic_apply_calls
                ));
            }
            if function.metrics.checked_vector_gets > 0 {
                lines.push(format!(
                    "    checked vector gets: {}",
                    function.metrics.checked_vector_gets
                ));
            }
            if function.metrics.direct_user_function_calls > 0 {
                lines.push(format!(
                    "    direct user function calls: {}",
                    function.metrics.direct_user_function_calls
                ));
            }
            if !function.calls.is_empty() {
                let calls = function
                    .calls
                    .iter()
                    .map(|call| format!("{} x{}", call.name, call.count))
                    .collect::<Vec<_>>()
                    .join(", ");
                lines.push(format!("    calls: {}", calls));
            }
        }
    }

    if !report.forms.is_empty() {
        lines.push(String::new());
        lines.push("Source user forms:".to_string());
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

fn collect_user_forms(
    forms: &[&TypedExpression],
    known_effects: &HashMap<String, EffectFlags>,
) -> Vec<ExplainForm> {
    forms
        .iter()
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
                effect: effect_labels(refined_form_effect(form, known_effects)),
                calls,
            }
        })
        .collect()
}

fn collect_user_effect_scope(
    forms: &[&TypedExpression],
    known_effects: &HashMap<String, EffectFlags>,
) -> HashMap<String, EffectFlags> {
    let mut scope = known_effects.clone();
    for form in forms {
        if let Some((keyword, name)) = top_level_binding(form) {
            if keyword == "let" || keyword == "letrec" || keyword == "mut" {
                let effect = refined_form_effect(form, &scope);
                scope.insert(name.to_string(), effect);
            }
        }
    }
    scope
}

fn refined_form_effect(
    form: &TypedExpression,
    known_effects: &HashMap<String, EffectFlags>,
) -> EffectFlags {
    let self_name =
        top_level_binding(form).and_then(|(keyword, name)| (keyword == "letrec").then_some(name));

    refine_effect_with_known_calls(&form.expr, form.effect, known_effects, self_name)
}

fn top_level_binding(form: &TypedExpression) -> Option<(&str, &str)> {
    match &form.expr {
        Expression::Apply(items) => match &items[..] {
            [Expression::Word(keyword), Expression::Word(name), ..] => {
                Some((keyword.as_str(), name.as_str()))
            }
            _ => None,
        },
        _ => None,
    }
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
    collect_wat_metrics_from_slice(metric_wat.as_str())
}

fn collect_wat_metrics_from_slice(wat: &str) -> ExplainMetrics {
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

fn collect_compiled_functions(wat: &str) -> Vec<ExplainCompiledFunction> {
    split_user_metric_functions(wat)
        .into_iter()
        .map(|function| ExplainCompiledFunction {
            name: function.display_name,
            wat_name: function.wat_name,
            metrics: collect_wat_metrics_from_slice(&function.body),
            calls: collect_call_counts(&function.body),
        })
        .collect()
}

struct WatFunctionSlice {
    display_name: String,
    wat_name: String,
    body: String,
}

fn split_user_metric_functions(wat: &str) -> Vec<WatFunctionSlice> {
    let lines = wat.lines().collect::<Vec<_>>();
    let mut out = Vec::new();
    let mut i = 0usize;
    while i < lines.len() {
        let trimmed = lines[i].trim_start();
        let names = if trimmed.starts_with("(func (export \"main\")") {
            Some(("main".to_string(), "main".to_string()))
        } else if trimmed.starts_with("(func $v_")
            && !trimmed.starts_with("(func $v___partial_dyn_")
        {
            let wat_name = trimmed
                .split_whitespace()
                .nth(1)
                .unwrap_or("$v_<unknown>")
                .trim_start_matches('$')
                .to_string();
            let display_name = wat_name
                .strip_prefix("v_")
                .map(demangle_wat_user_symbol)
                .unwrap_or_else(|| wat_name.clone());
            Some((display_name, wat_name))
        } else {
            None
        };

        if let Some((display_name, wat_name)) = names {
            let mut depth = 0i32;
            let mut body = Vec::new();
            while i < lines.len() {
                let line = lines[i];
                depth += line.matches('(').count() as i32;
                depth -= line.matches(')').count() as i32;
                body.push(line);
                i += 1;
                if depth <= 0 {
                    break;
                }
            }
            out.push(WatFunctionSlice {
                display_name,
                wat_name,
                body: body.join("\n"),
            });
            continue;
        }
        i += 1;
    }
    out
}

fn collect_call_counts(wat: &str) -> Vec<ExplainCall> {
    let mut counts: HashMap<(String, String), usize> = HashMap::new();
    for line in wat.lines() {
        let trimmed = line.trim_start();
        let target = trimmed
            .strip_prefix("call $")
            .or_else(|| trimmed.strip_prefix("return_call $"));
        let Some(target) = target else {
            continue;
        };
        let target = target.split_whitespace().next().unwrap_or("");
        if target.is_empty() {
            continue;
        }
        let (name, kind) = explain_call_name_and_kind(target);
        *counts.entry((name, kind)).or_insert(0) += 1;
    }

    let mut calls = counts
        .into_iter()
        .map(|((name, kind), count)| ExplainCall { name, kind, count })
        .collect::<Vec<_>>();
    calls.sort_by(|a, b| {
        call_kind_rank(&a.kind)
            .cmp(&call_kind_rank(&b.kind))
            .then_with(|| a.name.cmp(&b.name))
    });
    calls
}

fn explain_call_name_and_kind(target: &str) -> (String, String) {
    if let Some(user_name) = target.strip_prefix("v_") {
        return (
            demangle_wat_user_symbol(user_name),
            "user-function".to_string(),
        );
    }
    if target.starts_with("apply") {
        return (target.to_string(), "dynamic-apply".to_string());
    }
    if target.starts_with("host_") {
        return (
            target.trim_start_matches("host_").to_string(),
            "host-import".to_string(),
        );
    }
    if matches!(
        target,
        "vec_new_i32" | "vec_new_zeroed_i32" | "vec_new_uninit_i32" | "tuple_new" | "closure_new"
    ) {
        return (target.to_string(), "allocation".to_string());
    }
    if target.starts_with("vec_") || target.starts_with("tuple_") || target.starts_with("closure_")
    {
        return (target.to_string(), "runtime".to_string());
    }
    (target.to_string(), "runtime".to_string())
}

fn call_kind_rank(kind: &str) -> usize {
    match kind {
        "user-function" => 0,
        "dynamic-apply" => 1,
        "allocation" => 2,
        "host-import" => 3,
        _ => 4,
    }
}

fn function_has_interesting_explain_details(function: &ExplainCompiledFunction) -> bool {
    function.metrics.vector_allocations > 0
        || function.metrics.zeroed_vector_allocations > 0
        || function.metrics.uninit_vector_allocations > 0
        || function.metrics.tuple_allocations > 0
        || function.metrics.closure_allocations > 0
        || function.metrics.dynamic_apply_calls > 0
        || function.metrics.checked_vector_gets > 0
        || function.metrics.direct_user_function_calls > 0
}

fn user_metric_wat(wat: &str) -> String {
    let lines = wat.lines().collect::<Vec<_>>();
    let mut out = Vec::new();
    let mut i = 0usize;
    while i < lines.len() {
        let trimmed = lines[i].trim_start();
        if trimmed.starts_with("(func (export \"main\")")
            || (trimmed.starts_with("(func $v_") && !trimmed.starts_with("(func $v___partial_dyn_"))
        {
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

fn collect_prefixed_call_targets(wat: &str, prefix: &str) -> Vec<String> {
    let mut calls = wat
        .lines()
        .filter_map(|line| line.trim_start().strip_prefix(prefix))
        .filter_map(|rest| rest.split_whitespace().next())
        .map(demangle_wat_user_symbol)
        .collect::<Vec<_>>();
    calls.sort();
    calls.dedup();
    calls
}

fn demangle_wat_user_symbol(symbol: &str) -> String {
    symbol
        .replace("_dash__gt_", "->")
        .replace("_slash_", "/")
        .replace("_dash_", "-")
        .replace("_gt_", ">")
        .replace("_bang_", "!")
        .replace("_question_", "?")
        .replace("_dot_", ".")
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

    #[test]
    fn explain_does_not_count_internal_partial_dynamic_helpers_as_user_apply() {
        let report = explain_source("(let add (lambda (a b) (+ a b)))\n(add 1 2)");
        assert_eq!(report.metrics.dynamic_apply_calls, 0);
        assert!(report
            .warnings
            .iter()
            .all(|warning| warning.kind != "dynamic_apply"));
    }

    #[test]
    fn explain_reports_compiled_function_details_for_allocations_and_calls() {
        let report = explain_source(
            r#"(let make (lambda (a b) [a b]))
(make 40 2)"#,
        );
        let text = render_text(&report);

        assert!(
            text.contains("Compiled function details:"),
            "expected function detail section, got:\n{}",
            text
        );
        assert!(
            text.contains("make:"),
            "expected make function detail, got:\n{}",
            text
        );
        assert!(
            text.contains("allocations: vector"),
            "expected vector allocation attribution, got:\n{}",
            text
        );
        assert!(
            text.contains("calls: make x1"),
            "expected direct user call attribution, got:\n{}",
            text
        );
    }

    #[test]
    fn explain_refines_known_std_alias_effects() {
        let source = r#"(letrec pure/sum (lambda (xs s) (if (empty? xs) s (pure/sum (cdr xs) (+ (car xs) s)))))
(pure/sum [ 1 2 3 ] 0)"#;
        let std_defs = crate::lsp_native_core::load_std_definitions();
        let (base_env, base_next_id, _signatures, effects) =
            crate::lsp_native_core::build_base_environment(&std_defs);
        let wrapped_with_program =
            crate::parser::merge_std_and_program(source, std_defs).expect("source should merge");
        let (_typ, typed) = crate::infer::infer_with_builtins_typed(
            &wrapped_with_program,
            (base_env, base_next_id),
        )
        .expect("source should infer");
        let split =
            crate::wat::compile_program_to_split_wat_typed(&typed).expect("source should compile");
        let report = explain_program_with_effects(&typed, &split.user_wat, 2, &effects);

        assert!(
            !report.effect.iter().any(|label| label == "unknown-call"),
            "expected explain effect not to contain unknown-call, got: {:?}",
            report.effect
        );
    }

    #[test]
    fn explain_keeps_unknown_call_for_function_parameter() {
        let source = r#"(letrec pure/sum (lambda (xs s f) (if (f xs) s (pure/sum (cdr xs) (+ (car xs) s) f))))
(pure/sum [ 1 2 3 ] 0 empty?)"#;
        let std_defs = crate::lsp_native_core::load_std_definitions();
        let (base_env, base_next_id, _signatures, effects) =
            crate::lsp_native_core::build_base_environment(&std_defs);
        let wrapped_with_program =
            crate::parser::merge_std_and_program(source, std_defs).expect("source should merge");
        let (_typ, typed) = crate::infer::infer_with_builtins_typed(
            &wrapped_with_program,
            (base_env, base_next_id),
        )
        .expect("source should infer");
        let split =
            crate::wat::compile_program_to_split_wat_typed(&typed).expect("source should compile");
        let report = explain_program_with_effects(&typed, &split.user_wat, 2, &effects);

        assert!(
            report.effect.iter().any(|label| label == "unknown-call"),
            "expected explain effect to keep unknown-call, got: {:?}",
            report.effect
        );
    }
}
