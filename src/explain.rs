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
    pub optimization_targets: Vec<ExplainOptimizationTarget>,
    pub forms: Vec<ExplainForm>,
    pub bounds_proofs: Vec<ExplainBoundsProof>,
    pub termination: Vec<ExplainTermination>,
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
    pub guarded_fallback_calls: Vec<ExplainCall>,
}

#[derive(Debug, Serialize)]
pub struct ExplainCall {
    pub name: String,
    pub kind: String,
    pub count: usize,
}

#[derive(Debug, Serialize)]
pub struct ExplainOptimizationTarget {
    pub function: String,
    pub kind: String,
    pub count: usize,
    pub note: String,
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
    pub details: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<ExplainLocation>,
}

#[derive(Debug, Serialize)]
pub struct ExplainLocation {
    pub form: usize,
    pub line: u32,
    pub column: u32,
    pub end_line: u32,
    pub end_column: u32,
}

#[derive(Debug, Serialize)]
pub struct ExplainTermination {
    pub subject: String,
    pub status: String,
    pub measure: Option<String>,
    pub reason: String,
    pub proof: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct ExplainBoundsProof {
    pub expression: String,
    pub details: Vec<String>,
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
    explain_program_with_effects_and_source(
        typed_ast,
        wat,
        user_form_count,
        known_effects,
        None,
    )
}

pub fn explain_program_with_effects_and_source(
    typed_ast: &TypedExpression,
    wat: &str,
    user_form_count: usize,
    known_effects: &HashMap<String, EffectFlags>,
    source: Option<&str>,
) -> ExplainReport {
    let metrics = collect_wat_metrics(wat);
    let host_imports = collect_host_imports(wat);
    let optimized_user_calls = collect_prefixed_call_targets(&user_metric_wat(wat), "call $v_");
    let compiled_functions = collect_compiled_functions(wat);
    let optimization_targets = collect_optimization_targets(&compiled_functions);
    let user_nodes = user_form_nodes(typed_ast, user_form_count);
    let mut external_impurity = HashMap::new();
    crate::infer::collect_top_level_function_external_impurity(typed_ast, &mut external_impurity);
    let effect_scope = collect_user_effect_scope(&user_nodes, known_effects, &external_impurity);
    let forms = collect_user_forms(&user_nodes, &effect_scope, &external_impurity);
    let termination = crate::static_analysis::explain_termination(typed_ast, user_form_count)
        .into_iter()
        .map(|finding| ExplainTermination {
            subject: finding.subject,
            status: finding.status,
            measure: finding.measure,
            reason: finding.reason,
            proof: finding.proof,
        })
        .collect();
    let bounds_proofs = crate::static_analysis::explain_bounds_proofs(typed_ast, user_form_count)
        .into_iter()
        .map(|proof| ExplainBoundsProof {
            expression: proof.expression,
            details: proof.details,
        })
        .collect();
    let user_effect = user_nodes.into_iter().fold(EffectFlags::PURE, |acc, form| {
        acc | observable_form_effect(
            form,
            refined_form_effect(form, &effect_scope),
            &external_impurity,
        )
    });
    let result_type = user_form_nodes(typed_ast, user_form_count)
        .last()
        .and_then(|form| form.typ.as_ref())
        .or(typed_ast.typ.as_ref())
        .map(|typ| normalize_signature(&typ.to_string()))
        .unwrap_or_else(|| "_".to_string());
    let mut warnings = Vec::new();

    for finding in crate::static_analysis::analyze_user_program_diagnostics_detailed(
        typed_ast,
        user_form_count,
    ) {
        let location = source.and_then(|source| {
            let ranges = crate::lsp_native_core::static_analysis_diagnostic_ranges(
                source,
                &finding.message,
                finding.user_form_index,
            );
            (ranges.len() == 1).then(|| {
                let range = ranges[0];
                ExplainLocation {
                    form: finding.user_form_index,
                    line: range.start.line + 1,
                    column: range.start.character + 1,
                    end_line: range.end.line + 1,
                    end_column: range.end.character + 1,
                }
            })
        });
        let finding = finding.message;
        let mut lines = finding.lines();
        let message = lines.next().unwrap_or(&finding).to_string();
        let suggestion = lines
            .clone()
            .find_map(|line| line.strip_prefix("help: "))
            .map(str::to_string);
        let details = lines
            .filter_map(|line| line.strip_prefix("detail: "))
            .map(str::to_string)
            .collect();
        let kind = if message.starts_with("static bounds:") {
            "static_bounds"
        } else if message.starts_with("static arithmetic:") {
            "static_arithmetic"
        } else if message.starts_with("termination:") {
            "termination"
        } else {
            "static_analysis"
        };
        warnings.push(ExplainWarning {
            kind: kind.to_string(),
            message,
            suggestion,
            details,
            location,
        });
    }

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
            location: None,
            details: Vec::new(),
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
            location: None,
            details: Vec::new(),
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
            location: None,
            details: Vec::new(),
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
            location: None,
            details: Vec::new(),
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
            location: None,
            details: Vec::new(),
        });
    }

    ExplainReport {
        result_type,
        effect: effect_labels(user_effect),
        host_imports,
        metrics,
        optimized_user_calls,
        compiled_functions,
        optimization_targets,
        forms,
        bounds_proofs,
        termination,
        warnings,
    }
}

pub fn render_text(report: &ExplainReport) -> String {
    let mut lines = Vec::new();
    lines.push("Que Explain".to_string());
    lines.push(format!("Result type: {}", report.result_type));

    lines.push(String::new());
    lines.push("Correctness:".to_string());
    let correctness_warnings = report
        .warnings
        .iter()
        .filter(|warning| matches!(warning.kind.as_str(), "static_bounds" | "static_arithmetic" | "termination"))
        .collect::<Vec<_>>();
    if correctness_warnings.is_empty() {
        lines.push("  no warnings".to_string());
    }
    for warning in correctness_warnings {
        let location = warning
            .location
            .as_ref()
            .map(|location| format!(" at {}:{}", location.line, location.column))
            .unwrap_or_default();
        lines.push(format!("  {}{}: {}", warning.kind, location, warning.message));
        for detail in &warning.details {
            lines.push(format!("    {detail}"));
        }
        if let Some(suggestion) = &warning.suggestion {
            lines.push(format!("    suggestion: {suggestion}"));
        }
    }

    if !report.bounds_proofs.is_empty() {
        lines.push("  Proven bounds:".to_string());
        for proof in &report.bounds_proofs {
            lines.push(format!("    {} proven safe", proof.expression));
            for detail in &proof.details {
                lines.push(format!("      {detail}"));
            }
        }
    }

    lines.push(String::new());
    lines.push("Termination:".to_string());
    if report.termination.is_empty() {
        lines.push("  no loops or recursive functions analyzed".to_string());
    }
    for finding in &report.termination {
        lines.push(format!("  {} terminating: {}", finding.status, finding.subject));
        lines.push(format!("    {}", finding.reason));
        for fact in &finding.proof {
            lines.push(format!("    {fact}"));
        }
    }

    lines.push(String::new());
    lines.push("Effects and permissions:".to_string());
    lines.push(format!("  effect: {}", format_labels(&report.effect)));
    lines.push(format!(
        "  host imports: {}",
        if report.host_imports.is_empty() {
            "none".to_string()
        } else {
            report.host_imports.join(", ")
        }
    ));
    lines.push(String::new());
    lines.push("Performance:".to_string());
    for warning in report.warnings.iter().filter(|warning| {
        !matches!(warning.kind.as_str(), "static_bounds" | "static_arithmetic" | "termination" | "host_imports")
    }) {
        lines.push(format!("  {}: {}", warning.kind, warning.message));
        if let Some(suggestion) = &warning.suggestion {
            lines.push(format!("    suggestion: {suggestion}"));
        }
    }
    lines.push("  WAT shape:".to_string());
    lines.push(format!(
        "    vector allocations: {}",
        report.metrics.vector_allocations
    ));
    lines.push(format!(
        "    zeroed vector allocations: {}",
        report.metrics.zeroed_vector_allocations
    ));
    lines.push(format!(
        "    uninit vector allocations: {}",
        report.metrics.uninit_vector_allocations
    ));
    lines.push(format!(
        "    tuple allocations: {}",
        report.metrics.tuple_allocations
    ));
    lines.push(format!(
        "    closure allocations: {}",
        report.metrics.closure_allocations
    ));
    lines.push(format!(
        "    dynamic apply calls: {}",
        report.metrics.dynamic_apply_calls
    ));
    lines.push(format!(
        "    checked vector gets: {}",
        report.metrics.checked_vector_gets
    ));
    lines.push(format!(
        "    unchecked vector gets: {}",
        report.metrics.unchecked_vector_gets
    ));
    lines.push(format!(
        "    direct user function calls: {}",
        report.metrics.direct_user_function_calls
    ));
    lines.push(format!("    wat bytes: {}", report.metrics.wat_bytes));

    if !report.optimized_user_calls.is_empty() {
        lines.push(format!(
            "  optimized user calls: {}",
            report.optimized_user_calls.join(", ")
        ));
    }

    if !report.optimization_targets.is_empty() {
        lines.push(String::new());
        lines.push("  Optimization targets (static WAT shape, not runtime profile):".to_string());
        for target in &report.optimization_targets {
            lines.push(format!(
                "    {}: {} x{}",
                target.function, target.kind, target.count
            ));
            lines.push(format!("      note: {}", target.note));
        }
    }

    let interesting_functions = report
        .compiled_functions
        .iter()
        .filter(|function| function_has_interesting_explain_details(function))
        .collect::<Vec<_>>();
    if !interesting_functions.is_empty() || !report.forms.is_empty() {
        lines.push(String::new());
        lines.push("Generated code:".to_string());
    }
    if !interesting_functions.is_empty() {
        lines.push("  Compiled function details:".to_string());
        for function in interesting_functions {
            lines.push(format!("    {}:", function.name));
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
            if !function.guarded_fallback_calls.is_empty() {
                let calls = function
                    .guarded_fallback_calls
                    .iter()
                    .map(|call| format!("{} x{}", call.name, call.count))
                    .collect::<Vec<_>>()
                    .join(", ");
                lines.push(format!("    guarded fallback calls: {}", calls));
            }
        }
    }

    if !report.forms.is_empty() {
        lines.push("  Source user forms:".to_string());
        for form in &report.forms {
            lines.push(format!(
                "    {} {} : {} [{}]",
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

    lines.join("\n")
}

pub fn render_json(report: &ExplainReport) -> Result<String, String> {
    serde_json::to_string_pretty(report)
        .map_err(|e| format!("failed to render explain json: {}", e))
}

fn collect_user_forms(
    forms: &[&TypedExpression],
    known_effects: &HashMap<String, EffectFlags>,
    external_impurity: &HashMap<String, bool>,
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
            let effect = observable_form_effect(
                form,
                refined_form_effect(form, known_effects),
                external_impurity,
            );
            ExplainForm {
                name,
                kind,
                typ,
                effect: effect_labels(effect),
                calls,
            }
        })
        .collect()
}

fn observable_form_effect(
    form: &TypedExpression,
    mut effect: EffectFlags,
    external_impurity: &HashMap<String, bool>,
) -> EffectFlags {
    if let Some((_keyword, name)) = top_level_binding(form) {
        if external_impurity.get(name) == Some(&false) {
            effect = EffectFlags(effect.0 & !EffectFlags::MUTATE.0);
        }
    }
    effect
}

fn collect_user_effect_scope(
    forms: &[&TypedExpression],
    known_effects: &HashMap<String, EffectFlags>,
    external_impurity: &HashMap<String, bool>,
) -> HashMap<String, EffectFlags> {
    let mut scope = known_effects.clone();
    for form in forms {
        if let Some((keyword, name)) = top_level_binding(form) {
            if keyword == "let" || keyword == "letrec" || keyword == "mut" {
                let effect = observable_form_effect(
                    form,
                    refined_form_effect(form, &scope),
                    external_impurity,
                );
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
        vector_allocations: count_occurrences(wat, "call $vec_new_i32")
            + count_occurrences(wat, "call $vec_new_filled_i32"),
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
            guarded_fallback_calls: extract_guarded_fallback_region(&function.body)
                .map(|fallback| collect_call_counts(&fallback))
                .unwrap_or_default(),
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

fn extract_guarded_fallback_region(wat: &str) -> Option<String> {
    let lines = wat.lines().collect::<Vec<_>>();
    let mut guard_candidates = Vec::new();
    for window in lines.windows(2) {
        if window[0].trim() == "i32.const 0" {
            let trimmed = window[1].trim();
            if let Some(local) = trimmed.strip_prefix("local.set ") {
                guard_candidates.push(local);
            }
        }
    }
    for guard_local in guard_candidates {
        let local_get = format!("local.get {guard_local}");
        let mut if_idx = None;
        for (i, window) in lines.windows(2).enumerate() {
            if window[0].trim() == local_get && window[1].trim_start().starts_with("if (result") {
                if_idx = Some(i + 1);
                break;
            }
        }
        let Some(if_idx) = if_idx else {
            continue;
        };
        let mut depth = 1i32;
        let mut fallback = Vec::new();
        for line in lines.iter().skip(if_idx + 1) {
            let trimmed = line.trim();
            if trimmed == "else" && depth == 1 {
                return Some(fallback.join("\n"));
            }
            if trimmed == "if"
                || trimmed.starts_with("if ")
                || trimmed == "block"
                || trimmed.starts_with("block ")
                || trimmed == "loop"
                || trimmed.starts_with("loop ")
            {
                depth += 1;
            } else if trimmed == "end" {
                depth -= 1;
                if depth <= 0 {
                    break;
                }
            }
            fallback.push(*line);
        }
    }
    None
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
        "vec_new_i32"
            | "vec_new_filled_i32"
            | "vec_new_zeroed_i32"
            | "vec_new_uninit_i32"
            | "tuple_new"
            | "closure_new"
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
        || !function.guarded_fallback_calls.is_empty()
}

fn collect_optimization_targets(
    functions: &[ExplainCompiledFunction],
) -> Vec<ExplainOptimizationTarget> {
    let mut targets = Vec::new();
    for function in functions {
        push_optimization_target(
            &mut targets,
            function,
            "dynamic apply",
            function.metrics.dynamic_apply_calls,
            "Runtime function dispatch remains here; prefer direct calls or inlinable let-bound lambdas in hot code.",
        );
        push_optimization_target(
            &mut targets,
            function,
            "closure allocation",
            function.metrics.closure_allocations,
            "Function values or partial application allocate closures here.",
        );
        push_optimization_target(
            &mut targets,
            function,
            "tuple allocation",
            function.metrics.tuple_allocations,
            "Tuple values materialize here; destructure immediately or avoid tuple results in hot paths.",
        );
        push_optimization_target(
            &mut targets,
            function,
            "checked vector get",
            function.metrics.checked_vector_gets,
            "Bounds checks remain here; counted loops or known indexes may help.",
        );

        for call in &function.calls {
            let fallback_count = call_count_by_name(&function.guarded_fallback_calls, &call.name);
            let hot_count = call.count.saturating_sub(fallback_count);
            match call.name.as_str() {
                "vec_set_scalar_materialized_i32" => push_optimization_target(
                    &mut targets,
                    function,
                    "scalar vector set helper",
                    hot_count,
                    "Replacement writes still use a runtime set helper on the main path here; a raw-store proof could help.",
                ),
                "vec_materialize_i32" => push_optimization_target(
                    &mut targets,
                    function,
                    "vector materialization",
                    hot_count,
                    "Vector representation is being normalized before access; proving stable materialized vectors could remove this.",
                ),
                "vec_get_i32" => push_optimization_target(
                    &mut targets,
                    function,
                    "checked vector get helper",
                    hot_count,
                    "Runtime vector reads remain here; prove bounds or use simpler counted access patterns.",
                ),
                "vec_push_i32" | "vec_push_scalar_i32" => push_optimization_target(
                    &mut targets,
                    function,
                    "vector push helper",
                    hot_count,
                    "Append growth remains here; builder/fill lowering may help when final length is predictable.",
                ),
                "vec_new_i32" => push_optimization_target(
                    &mut targets,
                    function,
                    "generic vector allocation",
                    call.count,
                    "Generic vector allocation remains here; zeroed, uninit, or fixed-builder lowering may help.",
                ),
                "rc_retain" | "rc_release" if call.count >= 10 => push_optimization_target(
                    &mut targets,
                    function,
                    "reference counting",
                    hot_count,
                    "Reference retain/release traffic is visible here; ownership transfer or scalarization may help.",
                ),
                _ => {}
            }
        }
    }

    targets.sort_by(|a, b| {
        optimization_target_score(b)
            .cmp(&optimization_target_score(a))
            .then_with(|| a.function.cmp(&b.function))
            .then_with(|| a.kind.cmp(&b.kind))
    });
    targets.truncate(8);
    targets
}

fn push_optimization_target(
    targets: &mut Vec<ExplainOptimizationTarget>,
    function: &ExplainCompiledFunction,
    kind: &str,
    count: usize,
    note: &str,
) {
    if count == 0 {
        return;
    }
    targets.push(ExplainOptimizationTarget {
        function: function.name.clone(),
        kind: kind.to_string(),
        count,
        note: note.to_string(),
    });
}

fn call_count_by_name(calls: &[ExplainCall], name: &str) -> usize {
    calls
        .iter()
        .filter(|call| call.name == name)
        .map(|call| call.count)
        .sum()
}

fn optimization_target_score(target: &ExplainOptimizationTarget) -> usize {
    let weight = match target.kind.as_str() {
        "dynamic apply" => 1000,
        "scalar vector set helper" => 800,
        "vector materialization" => 600,
        "checked vector get" | "checked vector get helper" => 500,
        "closure allocation" => 450,
        "tuple allocation" => 350,
        "vector push helper" => 250,
        "generic vector allocation" => 200,
        "reference counting" => 50,
        _ => 10,
    };
    target.count.saturating_mul(weight)
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
        explain_program_with_effects_and_source(
            &typed,
            &split.user_wat,
            1,
            &HashMap::new(),
            Some(src),
        )
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
    fn explain_reports_proven_warning_and_unknown_termination() {
        let proven = explain_source(
            "(letrec down (lambda (n) (if (<= n 0) 0 (down (- n 1)))))",
        );
        assert!(proven.termination.iter().any(|finding| {
            finding.subject == "down"
                && finding.status == "proven"
                && finding.measure.as_deref() == Some("n")
        }));
        let text = render_text(&proven);
        assert!(text.contains("Termination:"));
        assert!(text.contains("proven terminating: down"));
        assert!(text.contains("base case:"));
        assert!(text.contains("recursive call:"));

        let warning = explain_source(
            "(letrec stuck (lambda (n) (if (= n 0) 0 (stuck n))))",
        );
        assert!(warning
            .termination
            .iter()
            .any(|finding| finding.subject == "stuck" && finding.status == "warning"));

        let unknown = explain_source(
            "(letrec mystery (lambda (n) (if (= n 0) 0 (mystery (* n 2)))))",
        );
        assert!(unknown
            .termination
            .iter()
            .any(|finding| finding.subject == "mystery" && finding.status == "unknown"));

        let symbolic_step = explain_source(
            "(let count (lambda (step) (mut i 0) (while (and (> step 0) (< i 10)) (alter! i (+ i step))) i))",
        );
        assert!(symbolic_step.termination.iter().any(|finding| {
            finding.status == "proven"
                && finding.measure.as_deref() == Some("i")
                && finding.reason.contains("i increases")
        }));
    }

    #[test]
    fn explain_reports_json_shape() {
        let report = explain_source("(+ 1 2)");
        let json = render_json(&report).expect("json should render");
        assert!(json.contains("\"result_type\": \"Int\""));
        assert!(json.contains("\"metrics\""));
    }

    #[test]
    fn explain_json_includes_static_analysis_warnings() {
        let report = explain_source("(* 50000 50000)");
        assert!(report
            .warnings
            .iter()
            .any(|warning| warning.kind == "static_arithmetic"));
        assert!(report.warnings.iter().any(|warning| {
            warning.kind == "static_arithmetic"
                && warning.location.as_ref().is_some_and(|location| {
                    location.line == 1 && location.column == 1
                })
        }));
        let json = render_json(&report).expect("report should serialize");
        let value: serde_json::Value =
            serde_json::from_str(&json).expect("report should be valid JSON");
        assert!(value["warnings"]
            .as_array()
            .is_some_and(|warnings| warnings.iter().any(|warning| {
                warning["kind"] == "static_arithmetic"
                    && warning["message"]
                        .as_str()
                        .is_some_and(|message| message.contains("overflow"))
            })));
    }

    #[test]
    fn explain_text_groups_sections_and_shows_analysis_evidence() {
        let report = explain_source(
            "(let n 10) (let xs []) (mut build 0) (while (<= build n) (push! xs true) (alter! build (+ build 1))) (mut i 0) (while (<= i n) (get xs i) (* i i) (alter! i (+ i 1)))",
        );
        let text = render_text(&report);
        let correctness = text.find("Correctness:").expect("correctness section");
        let performance = text.find("Performance:").expect("performance section");
        assert!(correctness < performance);
        assert!(text.contains("Proven bounds:"));
        assert!(text.contains("proven safe"));
        assert!(text.contains("initial:"));
        assert!(text.contains("condition:"));
        assert!(text.contains("update:"));
        assert!(text.contains("bound:"));

        let overflow = explain_source("(mut i 5000000) (* i i)");
        let warning = overflow
            .warnings
            .iter()
            .find(|warning| warning.kind == "static_arithmetic")
            .expect("overflow warning");
        assert!(warning
            .details
            .iter()
            .any(|detail| detail.contains("inferred range")));
        assert!(warning
            .details
            .iter()
            .any(|detail| detail.contains("safe square range")));
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
    fn explain_reports_ranked_optimization_targets() {
        let report = explain_source(
            r#"(let make (lambda (a b) [a b]))
(make 40 2)"#,
        );
        let text = render_text(&report);

        assert!(
            text.contains("Optimization targets (static WAT shape, not runtime profile):"),
            "expected optimization target section, got:\n{}",
            text
        );
        assert!(
            report.optimization_targets.iter().any(|target| {
                target.function == "make" && target.kind == "generic vector allocation"
            }),
            "expected generic vector allocation target, got: {:?}",
            report.optimization_targets
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

    #[test]
    fn explain_reports_transitive_io_without_local_mutation() {
        let source = r#"(let log! (lambda (x)
  (mut scratch 0)
  (alter! scratch x)
  (print! "log")))
(let search? (lambda (x)
  (letrec bs (lambda (n)
    (if (= n 0) true (do (log! n) (bs (- n 1))))))
  (bs x)))
(search? 1)"#;
        let std_defs = crate::lsp_native_core::load_std_definitions();
        let (base_env, base_next_id, _signatures, effects) =
            crate::lsp_native_core::build_base_environment(&std_defs);
        let program =
            crate::parser::merge_std_and_program(source, std_defs).expect("source should merge");
        let (_typ, typed) =
            crate::infer::infer_with_builtins_typed(&program, (base_env, base_next_id))
                .expect("source should infer");
        let split =
            crate::wat::compile_program_to_split_wat_typed(&typed).expect("source should compile");
        let report = explain_program_with_effects(&typed, &split.user_wat, 3, &effects);

        for name in ["log!", "search?"] {
            let form = report
                .forms
                .iter()
                .find(|form| form.name == name)
                .expect("function should be reported");
            assert!(form.effect.iter().any(|effect| effect == "io"));
            assert!(!form.effect.iter().any(|effect| effect == "mutate"));
        }
        assert!(report.effect.iter().any(|effect| effect == "io"));
        assert!(!report.effect.iter().any(|effect| effect == "mutate"));
    }
}
