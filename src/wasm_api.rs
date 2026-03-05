use crate::infer::{ infer_with_builtins_typed, TypedExpression };
use crate::lsp_native_core as native_core;
use crate::parser::{ self, Expression };
use crate::types::{ Type, TypeEnv };
#[cfg(feature = "compiler")]
use crate::wat;
use serde::Serialize;
use std::cell::RefCell;
use std::collections::{ HashMap, HashSet };
use wasm_bindgen::prelude::wasm_bindgen;

#[derive(Clone, Copy)]
struct Position {
    line: u32,
    character: u32,
}

#[derive(Clone, Copy)]
struct TextRange {
    start: Position,
    end: Position,
}

#[derive(Serialize)]
struct JsonPosition {
    line: u32,
    character: u32,
}

#[derive(Serialize)]
struct JsonRange {
    start: JsonPosition,
    end: JsonPosition,
}

#[derive(Serialize)]
struct JsonDiagnostic {
    message: String,
    severity: String,
    range: JsonRange,
}

#[derive(Serialize)]
struct JsonHover {
    contents: String,
    range: JsonRange,
}

#[derive(Serialize)]
struct JsonCompletionItem {
    label: String,
    detail: Option<String>,
    kind: String,
}

struct DocAnalysis {
    diagnostics: Vec<JsonDiagnostic>,
    symbol_types: HashMap<String, String>,
    user_bound_symbols: HashSet<String>,
}

struct WasmLspCore {
    std_defs: Vec<Expression>,
    base_env: TypeEnv,
    base_next_id: u64,
    global_signatures: HashMap<String, String>,
}

thread_local! {
    static LSP_CORE: RefCell<Option<WasmLspCore>> = const { RefCell::new(None) };
    #[cfg(feature = "compiler")]
    static OUTPUT: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
    #[cfg(feature = "compiler")]
    static STD: RefCell<parser::Expression> = RefCell::new(crate::baked::load_ast());
}

fn with_lsp_core<R>(f: impl FnOnce(&WasmLspCore) -> R) -> R {
    LSP_CORE.with(|cell| {
        if cell.borrow().is_none() {
            *cell.borrow_mut() = Some(build_lsp_core());
        }
        let core = cell.borrow();
        f(core.as_ref().expect("lsp core must be initialized"))
    })
}

fn build_lsp_core() -> WasmLspCore {
    let std_defs = load_std_definitions();
    let (base_env, base_next_id, global_signatures) = build_base_environment(&std_defs);
    WasmLspCore {
        std_defs,
        base_env,
        base_next_id,
        global_signatures,
    }
}

fn load_std_definitions() -> Vec<Expression> {
    native_core::load_std_definitions()
}

fn infer_standalone_std_symbol_signature(core: &WasmLspCore, symbol: &str) -> Option<String> {
    let program = parser::merge_std_and_program(symbol, core.std_defs.clone()).ok()?;
    let (typ, _typed) = infer_with_builtins_typed(&program, (
        core.base_env.clone(),
        core.base_next_id,
    )).ok()?;
    Some(normalize_signature(&typ.to_string()))
}

fn build_base_environment(std_defs: &[Expression]) -> (TypeEnv, u64, HashMap<String, String>) {
    native_core::build_base_environment(std_defs)
}

fn analyze_document_text(text: &str, core: &WasmLspCore) -> DocAnalysis {
    if text.trim().is_empty() {
        return DocAnalysis {
            diagnostics: Vec::new(),
            symbol_types: HashMap::new(),
            user_bound_symbols: HashSet::new(),
        };
    }

    let mut diagnostics = Vec::new();
    let mut symbol_types_raw: HashMap<String, Type> = HashMap::new();
    let mut let_binding_types_raw: HashMap<String, Type> = HashMap::new();
    let mut user_bound_symbols = HashSet::new();
    let analysis_source = strip_comment_bodies_preserve_newlines(text);

    if let Some(exprs) = parse_user_exprs_for_symbol_collection(&analysis_source) {
        collect_user_bound_symbols_from_exprs(&exprs, &mut user_bound_symbols);
    }

    let program = match parser::merge_std_and_program(&analysis_source, core.std_defs.clone()) {
        Ok(expr) => expr,
        Err(primary_err) => {
            let repaired = repair_source_for_analysis(&analysis_source);
            match parser::merge_std_and_program(&repaired, core.std_defs.clone()) {
                Ok(expr) => expr,
                Err(_) => {
                    diagnostics.push(make_error_diagnostic(text, primary_err));
                    return DocAnalysis {
                        diagnostics,
                        symbol_types: HashMap::new(),
                        user_bound_symbols,
                    };
                }
            }
        }
    };

    match infer_with_builtins_typed(&program, (core.base_env.clone(), core.base_next_id)) {
        Ok((_typ, typed)) => {
            collect_symbol_types(&typed, &mut symbol_types_raw);
            collect_let_binding_types(&typed, &mut let_binding_types_raw);
        }
        Err(err) => diagnostics.push(make_error_diagnostic(text, err)),
    }

    for (name, typ) in let_binding_types_raw {
        match symbol_types_raw.get(&name) {
            Some(existing) => {
                if should_replace_type(existing, &typ) {
                    symbol_types_raw.insert(name, typ);
                }
            }
            None => {
                symbol_types_raw.insert(name, typ);
            }
        }
    }

    let symbol_types = symbol_types_raw
        .into_iter()
        .map(|(name, typ)| (name, normalize_signature(&typ.to_string())))
        .collect();

    DocAnalysis {
        diagnostics,
        symbol_types,
        user_bound_symbols,
    }
}

#[wasm_bindgen]
pub fn lsp_diagnostics(text: String) -> String {
    with_lsp_core(|core| {
        let analysis = analyze_document_text(&text, core);
        serde_json::to_string(&analysis.diagnostics).unwrap_or_else(|_| "[]".to_string())
    })
}

#[wasm_bindgen]
pub fn lsp_completions(text: String) -> String {
    with_lsp_core(|core| {
        let analysis = analyze_document_text(&text, core);
        let mut merged_signatures: HashMap<String, String> = HashMap::new();

        for (name, signature) in &core.global_signatures {
            merged_signatures.insert(name.clone(), signature.clone());
        }
        for (name, signature) in &analysis.symbol_types {
            if analysis.user_bound_symbols.contains(name) {
                merged_signatures.insert(name.clone(), signature.clone());
                continue;
            }

            if !core.global_signatures.contains_key(name) {
                merged_signatures.insert(name.clone(), signature.clone());
            }
        }

        let mut items = Vec::new();
        for keyword in ["lambda", "if", "let", "let*", "do", "as"] {
            items.push(JsonCompletionItem {
                label: keyword.to_string(),
                detail: None,
                kind: "keyword".to_string(),
            });
        }
        for (label, detail) in merged_signatures {
            let kind = if detail.contains("->") { "function" } else { "constant" };
            items.push(JsonCompletionItem {
                label,
                detail: Some(normalize_signature(&detail)),
                kind: kind.to_string(),
            });
        }
        items.sort_by(|a, b| a.label.cmp(&b.label));

        serde_json::to_string(&items).unwrap_or_else(|_| "[]".to_string())
    })
}

#[wasm_bindgen]
pub fn lsp_hover(text: String, line: u32, character: u32) -> String {
    with_lsp_core(|core| {
        let analysis = analyze_document_text(&text, core);
        let position = Position { line, character };

        if let Some((literal_type, range)) = literal_type_at_position(&text, position) {
            let contents = format_literal_hover(&text, range, &literal_type);
            let hover = JsonHover {
                contents,
                range: to_json_range(range),
            };
            return serde_json::to_string(&Some(hover)).unwrap_or_else(|_| "null".to_string());
        }

        let Some((symbol, range)) = symbol_at_position(&text, position) else {
            return "null".to_string();
        };

        let doc_sig = analysis.symbol_types.get(&symbol).cloned();
        let global_sig = core.global_signatures.get(&symbol).cloned();
        let standalone_std_sig = if analysis.user_bound_symbols.contains(&symbol) {
            None
        } else if is_standalone_symbol_expr_at_range(&text, range, &symbol) {
            infer_standalone_std_symbol_signature(core, &symbol)
        } else {
            None
        };
        let type_info = if analysis.user_bound_symbols.contains(&symbol) {
            doc_sig.or(global_sig)
        } else {
            if is_standalone_symbol_expr_at_range(&text, range, &symbol) {
                standalone_std_sig.or(global_sig).or(doc_sig)
            } else {
                doc_sig.or(global_sig)
            }
        };

        let Some(type_info) = type_info else {
            return "null".to_string();
        };
        let type_info = normalize_signature(&type_info);

        let hover = JsonHover {
            contents: format!("{} : {}", symbol, type_info),
            range: to_json_range(range),
        };
        serde_json::to_string(&Some(hover)).unwrap_or_else(|_| "null".to_string())
    })
}

fn to_json_range(range: TextRange) -> JsonRange {
    JsonRange {
        start: JsonPosition {
            line: range.start.line,
            character: range.start.character,
        },
        end: JsonPosition {
            line: range.end.line,
            character: range.end.character,
        },
    }
}

fn to_core_position(position: Position) -> native_core::CorePosition {
    native_core::CorePosition {
        line: position.line,
        character: position.character,
    }
}

fn from_core_position(position: native_core::CorePosition) -> Position {
    Position {
        line: position.line,
        character: position.character,
    }
}

fn to_core_range(range: TextRange) -> native_core::CoreRange {
    native_core::CoreRange {
        start: to_core_position(range.start),
        end: to_core_position(range.end),
    }
}

fn from_core_range(range: native_core::CoreRange) -> TextRange {
    TextRange {
        start: from_core_position(range.start),
        end: from_core_position(range.end),
    }
}

fn is_standalone_symbol_expr_at_range(text: &str, range: TextRange, symbol: &str) -> bool {
    native_core::is_standalone_symbol_expr_at_range(text, to_core_range(range), symbol)
}

fn parse_user_exprs_for_symbol_collection(text: &str) -> Option<Vec<Expression>> {
    native_core::parse_user_exprs_for_symbol_collection(text)
}

fn strip_comment_bodies_preserve_newlines(text: &str) -> String {
    native_core::strip_comment_bodies_preserve_newlines(text)
}

fn repair_source_for_analysis(text: &str) -> String {
    native_core::repair_source_for_analysis(text)
}

fn collect_user_bound_symbols_from_exprs(exprs: &[Expression], out: &mut HashSet<String>) {
    native_core::collect_user_bound_symbols_from_exprs(exprs, out)
}

fn collect_let_binding_types(node: &TypedExpression, signatures: &mut HashMap<String, Type>) {
    native_core::collect_let_binding_types(node, signatures)
}

fn collect_symbol_types(node: &TypedExpression, symbols: &mut HashMap<String, Type>) {
    native_core::collect_symbol_types(node, symbols)
}

fn type_specificity_score(typ: &Type) -> i32 {
    match typ {
        Type::Var(_) => 0,
        Type::Int | Type::Float | Type::Bool | Type::Char | Type::Unit => 1,
        Type::List(inner) => 2 + type_specificity_score(inner),
        Type::Tuple(items) => 2 + items.iter().map(type_specificity_score).sum::<i32>(),
        Type::Function(a, b) => 3 + type_specificity_score(a) + type_specificity_score(b),
    }
}

fn should_replace_type(existing: &Type, candidate: &Type) -> bool {
    match (existing, candidate) {
        (Type::Var(_), Type::Var(_)) => false,
        (Type::Var(_), _) => true,
        (_, Type::Var(_)) => false,
        _ => type_specificity_score(candidate) > type_specificity_score(existing),
    }
}

fn normalize_signature(signature: &str) -> String {
    native_core::normalize_signature(signature)
}

fn strip_type_var_numbers(input: &str) -> String {
    native_core::strip_type_var_numbers(input)
}

fn make_error_diagnostic(text: &str, message: String) -> JsonDiagnostic {
    let normalized_message = strip_type_var_numbers(&message);
    let inferred_range = infer_error_range(text, &message);
    let range = inferred_range.unwrap_or_else(|| full_range(text));
    let display_message = if inferred_range.is_some() {
        diagnostic_summary_without_snippet(&normalized_message)
    } else {
        normalized_message
    };
    JsonDiagnostic {
        message: display_message,
        severity: "error".to_string(),
        range: to_json_range(range),
    }
}

fn diagnostic_summary_without_snippet(message: &str) -> String {
    native_core::diagnostic_summary_without_snippet(message)
}
fn infer_error_range(text: &str, message: &str) -> Option<TextRange> {
    native_core::infer_error_range(text, message).map(from_core_range)
}

fn full_range(text: &str) -> TextRange {
    from_core_range(native_core::full_range(text))
}

fn symbol_at_position(text: &str, position: Position) -> Option<(String, TextRange)> {
    native_core
        ::symbol_at_position(text, to_core_position(position))
        .map(|(symbol, range)| (symbol, from_core_range(range)))
}

fn literal_type_at_position(text: &str, position: Position) -> Option<(String, TextRange)> {
    native_core
        ::literal_type_at_position(text, to_core_position(position))
        .map(|(literal_type, range)| (literal_type, from_core_range(range)))
}

fn format_literal_hover(text: &str, range: TextRange, literal_type: &str) -> String {
    native_core::format_literal_hover(text, to_core_range(range), literal_type)
}

#[cfg(feature = "compiler")]
fn write_to_output(s: &str) -> *const u8 {
    OUTPUT.with(|buf| {
        let mut buf = buf.borrow_mut();
        buf.clear();
        buf.extend_from_slice(s.as_bytes());
        buf.as_ptr()
    })
}

#[cfg(feature = "compiler")]
#[wasm_bindgen]
pub fn get_output_ptr() -> *const u8 {
    OUTPUT.with(|buf| buf.borrow().as_ptr())
}

#[cfg(feature = "compiler")]
#[wasm_bindgen]
pub fn get_output_len() -> usize {
    OUTPUT.with(|buf| buf.borrow().len())
}

#[cfg(feature = "compiler")]
#[wasm_bindgen]
pub fn wat(program: String) -> *const u8 {
    let result = STD.with(|std| {
        let std_ast = std.borrow();
        if let parser::Expression::Apply(items) = &*std_ast {
            match parser::merge_std_and_program(&program, items[1..].to_vec()) {
                Ok(wrapped_ast) =>
                    match wat::compile_program_to_wat(&wrapped_ast) {
                        Ok(wat_src) => wat_src,
                        Err(err) => format!("3\n{}", err),
                    }
                Err(err) => format!("2\n{}", err),
            }
        } else {
            "1\nNo expressions...".to_string()
        }
    });
    write_to_output(&result)
}
