use crate::infer::{ infer_with_builtins_typed, TypedExpression };
use crate::parser::{ self, Expression };
use crate::types::{ create_builtin_environment, Type, TypeEnv };
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
    let ast = crate::baked::load_ast();
    if let Expression::Apply(items) = ast {
        return items.into_iter().skip(1).collect();
    }
    Vec::new()
}

fn build_base_environment(std_defs: &[Expression]) -> (TypeEnv, u64, HashMap<String, String>) {
    let (env, next_id) = create_builtin_environment(TypeEnv::new());
    let mut signatures = HashMap::new();

    for scope in &env.scopes {
        for (name, scheme) in scope {
            signatures
                .entry(name.clone())
                .or_insert_with(|| normalize_signature(&scheme.to_string()));
        }
    }

    for (name, signature) in infer_std_signatures(&env, next_id, std_defs) {
        signatures.insert(name, signature);
    }

    (env, next_id, signatures)
}

fn infer_std_signatures(
    base_env: &TypeEnv,
    base_next_id: u64,
    std_defs: &[Expression]
) -> HashMap<String, String> {
    if std_defs.is_empty() {
        return HashMap::new();
    }

    let std_program = Expression::Apply(
        std::iter
            ::once(Expression::Word("do".to_string()))
            .chain(std_defs.iter().cloned())
            .collect()
    );

    let mut raw: HashMap<String, Type> = HashMap::new();
    if
        let Ok((_typ, typed)) = infer_with_builtins_typed(&std_program, (
            base_env.clone(),
            base_next_id,
        ))
    {
        collect_let_binding_types(&typed, &mut raw);
    }

    raw.into_iter()
        .map(|(name, typ)| (name, normalize_signature(&typ.to_string())))
        .collect()
}

fn analyze_document_text(text: &str, core: &WasmLspCore) -> DocAnalysis {
    let mut diagnostics = Vec::new();
    let mut symbol_types_raw: HashMap<String, Type> = HashMap::new();
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
        Ok((_typ, typed)) => collect_symbol_types(&typed, &mut symbol_types_raw),
        Err(err) => diagnostics.push(make_error_diagnostic(text, err)),
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
            let should_override =
                analysis.user_bound_symbols.contains(name) ||
                !core.global_signatures.contains_key(name);
            if should_override {
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
                detail: Some(detail),
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

        let doc_sig = analysis.symbol_types.get(&symbol);
        let global_sig = core.global_signatures.get(&symbol);
        let type_info = if analysis.user_bound_symbols.contains(&symbol) {
            doc_sig.or(global_sig)
        } else {
            global_sig.or(doc_sig)
        };

        let Some(type_info) = type_info else {
            return "null".to_string();
        };

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

fn parse_user_exprs_for_symbol_collection(text: &str) -> Option<Vec<Expression>> {
    let masked = mask_literals_for_structural_parse(text);
    if let Ok(exprs) = parser::parse(&masked) {
        return Some(exprs);
    }
    let repaired = repair_source_for_analysis(&masked);
    parser::parse(&repaired).ok()
}

fn strip_comment_bodies_preserve_newlines(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut in_comment = false;
    let mut in_string = false;
    let mut in_char = false;

    for ch in text.chars() {
        if in_comment {
            if ch == '\n' {
                in_comment = false;
                out.push('\n');
            }
            continue;
        }

        if in_string {
            out.push(ch);
            if ch == '"' {
                in_string = false;
            }
            continue;
        }

        if in_char {
            out.push(ch);
            if ch == '\'' {
                in_char = false;
            }
            continue;
        }

        match ch {
            ';' => {
                in_comment = true;
                out.push(' ');
            }
            '"' => {
                in_string = true;
                out.push(ch);
            }
            '\'' => {
                in_char = true;
                out.push(ch);
            }
            _ => out.push(ch),
        }
    }

    out
}

fn mask_literals_for_structural_parse(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut in_comment = false;
    let mut in_string = false;
    let mut in_char = false;
    let mut str_idx = 0usize;
    let mut char_idx = 0usize;

    for ch in text.chars() {
        if in_comment {
            if ch == '\n' {
                in_comment = false;
                out.push('\n');
            }
            continue;
        }
        if in_string {
            if ch == '"' {
                in_string = false;
            }
            continue;
        }
        if in_char {
            if ch == '\'' {
                in_char = false;
            }
            continue;
        }

        match ch {
            ';' => {
                in_comment = true;
                out.push(' ');
            }
            '"' => {
                out.push(' ');
                out.push_str(&format!("__STR{}__", str_idx));
                out.push(' ');
                str_idx += 1;
                in_string = true;
            }
            '\'' => {
                out.push(' ');
                out.push_str(&format!("__CHR{}__", char_idx));
                out.push(' ');
                char_idx += 1;
                in_char = true;
            }
            _ => out.push(ch),
        }
    }

    out
}

fn repair_source_for_analysis(text: &str) -> String {
    let mut repaired = text.to_string();
    let mut stack: Vec<char> = Vec::new();
    let mut in_string = false;
    let mut in_comment = false;

    for ch in text.chars() {
        if in_comment {
            if ch == '\n' {
                in_comment = false;
            }
            continue;
        }
        if !in_string && ch == ';' {
            in_comment = true;
            continue;
        }
        if ch == '"' {
            in_string = !in_string;
            continue;
        }
        if in_string {
            continue;
        }
        match ch {
            '(' => stack.push(')'),
            '[' => stack.push(']'),
            ')' | ']' => {
                if let Some(expected) = stack.last().copied() {
                    if ch == expected {
                        let _ = stack.pop();
                    }
                }
            }
            _ => {}
        }
    }

    if in_string {
        repaired.push('"');
    }
    while let Some(close) = stack.pop() {
        repaired.push(close);
    }
    repaired
}

fn collect_user_bound_symbols_from_exprs(exprs: &[Expression], out: &mut HashSet<String>) {
    for expr in exprs {
        collect_user_bound_symbols(expr, out);
    }
}

fn collect_user_bound_symbols(expr: &Expression, out: &mut HashSet<String>) {
    if let Expression::Apply(items) = expr {
        if let Some(Expression::Word(head)) = items.first() {
            if head == "let" || head == "let*" {
                if let Some(Expression::Word(name)) = items.get(1) {
                    out.insert(name.clone());
                }
            } else if head == "lambda" && items.len() >= 2 {
                for param in items.iter().skip(1).take(items.len().saturating_sub(2)) {
                    collect_pattern_words(param, out);
                }
            }
        }
        for item in items {
            collect_user_bound_symbols(item, out);
        }
    }
}

fn collect_pattern_words(expr: &Expression, out: &mut HashSet<String>) {
    match expr {
        Expression::Word(word) => {
            if word != "." {
                out.insert(word.clone());
            }
        }
        Expression::Apply(items) => {
            for item in items {
                collect_pattern_words(item, out);
            }
        }
        _ => {}
    }
}

fn collect_let_binding_types(node: &TypedExpression, signatures: &mut HashMap<String, Type>) {
    if let Expression::Apply(items) = &node.expr {
        if let [Expression::Word(keyword), Expression::Word(name), _rhs, ..] = &items[..] {
            if keyword == "let" || keyword == "let*" {
                if let Some(rhs_type) = node.children.get(2).and_then(|child| child.typ.as_ref()) {
                    match signatures.get(name) {
                        Some(existing) => {
                            if should_replace_type(existing, rhs_type) {
                                signatures.insert(name.clone(), rhs_type.clone());
                            }
                        }
                        None => {
                            signatures.insert(name.clone(), rhs_type.clone());
                        }
                    }
                }
            }
        }
    }

    for child in &node.children {
        collect_let_binding_types(child, signatures);
    }
}

fn collect_symbol_types(node: &TypedExpression, symbols: &mut HashMap<String, Type>) {
    if let Expression::Word(name) = &node.expr {
        if let Some(typ) = &node.typ {
            match symbols.get(name) {
                Some(existing) => {
                    if should_replace_type(existing, typ) {
                        symbols.insert(name.clone(), typ.clone());
                    }
                }
                None => {
                    symbols.insert(name.clone(), typ.clone());
                }
            }
        }
    }
    for child in &node.children {
        collect_symbol_types(child, symbols);
    }
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
    let trimmed = signature.trim();
    let without_quantifier = if let Some(dot_idx) = trimmed.find('.') {
        let prefix = trimmed[..dot_idx].trim();
        let looks_like_quantifier = is_quantifier_prefix(prefix);
        if looks_like_quantifier {
            trimmed[dot_idx + 1..].trim()
        } else {
            trimmed
        }
    } else {
        trimmed
    };
    strip_type_var_numbers(without_quantifier)
}

fn is_quantifier_prefix(prefix: &str) -> bool {
    if prefix.is_empty() {
        return false;
    }

    let chars: Vec<char> = prefix.chars().collect();
    let mut i = 0usize;
    let mut saw_any = false;

    while i < chars.len() {
        while i < chars.len() && (chars[i].is_whitespace() || chars[i] == ',') {
            i += 1;
        }
        if i >= chars.len() {
            break;
        }

        if chars[i] != 'T' {
            return false;
        }
        saw_any = true;
        i += 1;

        while i < chars.len() && chars[i].is_whitespace() {
            i += 1;
        }

        let digit_start = i;
        while i < chars.len() && chars[i].is_ascii_digit() {
            i += 1;
        }
        if i == digit_start {
            return false;
        }
    }

    saw_any
}

fn strip_type_var_numbers(input: &str) -> String {
    let chars: Vec<char> = input.chars().collect();
    let mut out = String::with_capacity(input.len());
    let mut i = 0usize;
    while i < chars.len() {
        if chars[i] == 'T' {
            let mut j = i + 1;
            while j < chars.len() && chars[j].is_whitespace() {
                j += 1;
            }
            let digit_start = j;
            while j < chars.len() && chars[j].is_ascii_digit() {
                j += 1;
            }
            if j > digit_start {
                out.push('T');
                i = j;
                continue;
            }
        }
        out.push(chars[i]);
        i += 1;
    }
    out
}

fn make_error_diagnostic(text: &str, message: String) -> JsonDiagnostic {
    let normalized_message = strip_type_var_numbers(&message);
    let inferred_range = infer_error_range(text, &message);
    let range = inferred_range.unwrap_or_else(|| full_range(text));
    let display_message = if inferred_range.is_some() {
        first_error_line(&normalized_message).to_string()
    } else {
        normalized_message
    };
    JsonDiagnostic {
        message: display_message,
        severity: "error".to_string(),
        range: to_json_range(range),
    }
}

fn first_error_line(message: &str) -> &str {
    message
        .lines()
        .next()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .unwrap_or(message)
}

fn infer_error_range(text: &str, message: &str) -> Option<TextRange> {
    if message.contains("Char should be of length 1") {
        if let Some(range) = find_invalid_char_literal_range(text) {
            return Some(range);
        }
    }

    if let Some(snippet) = extract_error_snippet(message) {
        if let Some(range) = find_snippet_range(text, &snippet) {
            return Some(range);
        }
        if let Some(range) = find_call_prefix_range(text, &snippet) {
            return Some(range);
        }
    }

    if let Some(symbol) = extract_symbol_from_error(message) {
        if let Some(range) = find_symbol_range(text, &symbol) {
            return Some(range);
        }
    }

    find_first_call_range(text)
}

fn extract_error_snippet(message: &str) -> Option<String> {
    for line in message.lines().map(str::trim) {
        if line.starts_with('(') && line.ends_with(')') && line.len() >= 2 {
            return Some(line.to_string());
        }
    }

    let bytes = message.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        if bytes[i] == b'(' {
            let start = i;
            let mut depth = 1usize;
            i += 1;
            while i < bytes.len() {
                match bytes[i] {
                    b'(' => depth += 1,
                    b')' => {
                        depth = depth.saturating_sub(1);
                        if depth == 0 {
                            if let Some(slice) = message.get(start..=i) {
                                return Some(slice.trim().to_string());
                            }
                            return None;
                        }
                    }
                    _ => {}
                }
                i += 1;
            }
            break;
        }
        i += 1;
    }
    None
}

fn find_invalid_char_literal_range(text: &str) -> Option<TextRange> {
    let bytes = text.as_bytes();
    let mut i = 0usize;
    let mut in_string = false;
    let mut in_comment = false;

    while i < text.len() {
        let ch = text[i..].chars().next()?;
        let ch_len = ch.len_utf8();

        if in_comment {
            if ch == '\n' {
                in_comment = false;
            }
            i += ch_len;
            continue;
        }

        if !in_string && ch == ';' {
            in_comment = true;
            i += ch_len;
            continue;
        }

        if ch == '"' {
            let escaped = i > 0 && bytes[i - 1] == b'\\';
            if !escaped {
                in_string = !in_string;
            }
            i += ch_len;
            continue;
        }

        if !in_string && ch == '\'' {
            let start = i;
            i += ch_len;
            let mut char_count = 0usize;
            let mut closed = false;

            while i < text.len() {
                let inner = text[i..].chars().next()?;
                let inner_len = inner.len_utf8();
                if inner == '\'' {
                    closed = true;
                    i += inner_len;
                    break;
                }
                char_count += 1;
                i += inner_len;
            }

            if !closed {
                return Some(TextRange {
                    start: byte_offset_to_position(text, start),
                    end: byte_offset_to_position(text, (start + 1).min(text.len())),
                });
            }

            if char_count != 1 {
                return Some(TextRange {
                    start: byte_offset_to_position(text, start),
                    end: byte_offset_to_position(text, i),
                });
            }

            continue;
        }

        i += ch_len;
    }

    None
}

fn find_snippet_range(text: &str, snippet: &str) -> Option<TextRange> {
    if snippet.is_empty() {
        return None;
    }
    let bytes = text.as_bytes();
    let needle = snippet.as_bytes();
    if needle.len() > bytes.len() {
        return None;
    }

    let mut i = 0usize;
    let mut in_string = false;
    let mut in_comment = false;
    while i + needle.len() <= bytes.len() {
        let b = bytes[i];
        if in_comment {
            if b == b'\n' {
                in_comment = false;
            }
            i += 1;
            continue;
        }
        if !in_string && b == b';' {
            in_comment = true;
            i += 1;
            continue;
        }
        if b == b'"' && (i == 0 || bytes[i - 1] != b'\\') {
            in_string = !in_string;
            i += 1;
            continue;
        }
        if in_string {
            i += 1;
            continue;
        }

        if &bytes[i..i + needle.len()] == needle {
            let start = i;
            let end = i + needle.len();
            return Some(TextRange {
                start: byte_offset_to_position(text, start),
                end: byte_offset_to_position(text, end),
            });
        }
        i += 1;
    }
    None
}

fn find_call_prefix_range(text: &str, snippet: &str) -> Option<TextRange> {
    let all_tokens = extract_call_prefix_tokens(snippet, 3);
    if all_tokens.is_empty() {
        return None;
    }
    for token_count in (1..=all_tokens.len()).rev() {
        let prefix_tokens = &all_tokens[..token_count];
        let bytes = text.as_bytes();
        let mut i = 0usize;
        let mut in_string = false;
        let mut in_comment = false;
        while i < bytes.len() {
            if in_comment {
                if bytes[i] == b'\n' {
                    in_comment = false;
                }
                i += 1;
                continue;
            }
            if !in_string && bytes[i] == b';' {
                in_comment = true;
                i += 1;
                continue;
            }
            if bytes[i] == b'"' && (i == 0 || bytes[i - 1] != b'\\') {
                in_string = !in_string;
                i += 1;
                continue;
            }
            if in_string {
                i += 1;
                continue;
            }

            if bytes[i] == b'(' && match_call_prefix_at(text, i, prefix_tokens) {
                if let Some(close) = find_matching_paren_byte(text, i) {
                    return Some(TextRange {
                        start: byte_offset_to_position(text, i),
                        end: byte_offset_to_position(text, close + 1),
                    });
                }
            }
            i += 1;
        }
    }
    None
}

fn extract_call_prefix_tokens(snippet: &str, max_tokens: usize) -> Vec<String> {
    let bytes = snippet.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() && bytes[i] != b'(' {
        i += 1;
    }
    if i >= bytes.len() {
        return Vec::new();
    }
    i += 1;

    let mut tokens = Vec::new();
    while i < bytes.len() && tokens.len() < max_tokens {
        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        if i >= bytes.len() || bytes[i] == b')' || bytes[i] == b'(' {
            break;
        }
        let start = i;
        while i < bytes.len() {
            let b = bytes[i];
            if b.is_ascii_whitespace() || matches!(b, b'(' | b')' | b'[' | b']' | b'"' | b';') {
                break;
            }
            i += 1;
        }
        if start < i {
            if let Ok(tok) = std::str::from_utf8(&bytes[start..i]) {
                tokens.push(tok.to_string());
            }
        } else {
            break;
        }
    }
    tokens
}

fn match_call_prefix_at(text: &str, open_idx: usize, tokens: &[String]) -> bool {
    if tokens.is_empty() {
        return false;
    }
    let bytes = text.as_bytes();
    if bytes.get(open_idx).copied() != Some(b'(') {
        return false;
    }
    let mut i = open_idx + 1;
    for (pos, tok) in tokens.iter().enumerate() {
        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        if i >= bytes.len() {
            return false;
        }
        let tok_bytes = tok.as_bytes();
        if i + tok_bytes.len() > bytes.len() {
            return false;
        }
        if &bytes[i..i + tok_bytes.len()] != tok_bytes {
            return false;
        }
        let left_ok = i == 0 || !is_ident_char(bytes[i - 1]);
        let right_idx = i + tok_bytes.len();
        let right_ok = right_idx >= bytes.len() || !is_ident_char(bytes[right_idx]);
        if !left_ok || !right_ok {
            return false;
        }
        i = right_idx;
        if pos + 1 < tokens.len() {
            while i < bytes.len() && bytes[i].is_ascii_whitespace() {
                i += 1;
            }
            if i >= bytes.len() || bytes[i] == b'(' || bytes[i] == b')' {
                return false;
            }
        }
    }
    true
}

fn find_first_call_range(text: &str) -> Option<TextRange> {
    let bytes = text.as_bytes();
    let mut i = 0usize;
    let mut in_string = false;
    let mut in_comment = false;

    while i < bytes.len() {
        if in_comment {
            if bytes[i] == b'\n' {
                in_comment = false;
            }
            i += 1;
            continue;
        }
        if !in_string && bytes[i] == b';' {
            in_comment = true;
            i += 1;
            continue;
        }
        if bytes[i] == b'"' && (i == 0 || bytes[i - 1] != b'\\') {
            in_string = !in_string;
            i += 1;
            continue;
        }
        if in_string {
            i += 1;
            continue;
        }

        if bytes[i] == b'(' {
            let open = i;
            i += 1;

            while i < bytes.len() && bytes[i].is_ascii_whitespace() {
                i += 1;
            }
            if i >= bytes.len() {
                break;
            }

            let name_start = i;
            while i < bytes.len() {
                let b = bytes[i];
                if b.is_ascii_whitespace() || matches!(b, b'(' | b')' | b'[' | b']' | b'"' | b';') {
                    break;
                }
                i += 1;
            }

            if name_start == i {
                continue;
            }
            let name = std::str::from_utf8(&bytes[name_start..i]).ok()?;
            if name.starts_with('_')
                || matches!(name, "do" | "let" | "let*" | "if" | "lambda" | "as")
            {
                continue;
            }

            if let Some(close) = find_matching_paren_byte(text, open) {
                return Some(TextRange {
                    start: byte_offset_to_position(text, open),
                    end: byte_offset_to_position(text, close + 1),
                });
            }
        } else {
            i += 1;
        }
    }
    None
}

fn extract_symbol_from_error(message: &str) -> Option<String> {
    for prefix in ["Undefined variable: ", "Unknown function: "] {
        if let Some(rest) = message.strip_prefix(prefix) {
            let symbol = rest.lines().next().unwrap_or("").trim();
            if !symbol.is_empty() {
                return Some(symbol.to_string());
            }
        }
    }
    None
}

fn is_ident_char(ch: u8) -> bool {
    ch.is_ascii_alphanumeric() || matches!(ch, b'_' | b'/' | b'-' | b'?' | b'!' | b'*' | b'+' | b'.')
}

fn find_symbol_range(text: &str, symbol: &str) -> Option<TextRange> {
    if symbol.is_empty() {
        return None;
    }
    let bytes = text.as_bytes();
    let needle = symbol.as_bytes();
    if needle.len() > bytes.len() {
        return None;
    }

    let mut i = 0usize;
    let mut in_string = false;
    let mut in_comment = false;
    while i + needle.len() <= bytes.len() {
        let b = bytes[i];
        if in_comment {
            if b == b'\n' {
                in_comment = false;
            }
            i += 1;
            continue;
        }
        if !in_string && b == b';' {
            in_comment = true;
            i += 1;
            continue;
        }
        if b == b'"' && (i == 0 || bytes[i - 1] != b'\\') {
            in_string = !in_string;
            i += 1;
            continue;
        }
        if in_string {
            i += 1;
            continue;
        }

        if &bytes[i..i + needle.len()] == needle {
            let left_ok = i == 0 || !is_ident_char(bytes[i - 1]);
            let right_ok = i + needle.len() == bytes.len() || !is_ident_char(bytes[i + needle.len()]);
            if left_ok && right_ok {
                return Some(TextRange {
                    start: byte_offset_to_position(text, i),
                    end: byte_offset_to_position(text, i + needle.len()),
                });
            }
        }
        i += 1;
    }
    None
}

fn find_matching_paren_byte(text: &str, open_idx: usize) -> Option<usize> {
    let bytes = text.as_bytes();
    if bytes.get(open_idx).copied() != Some(b'(') {
        return None;
    }

    let mut depth = 0usize;
    let mut in_string = false;
    let mut in_comment = false;

    for i in open_idx..bytes.len() {
        let b = bytes[i];

        if in_comment {
            if b == b'\n' {
                in_comment = false;
            }
            continue;
        }

        if !in_string && b == b';' {
            in_comment = true;
            continue;
        }

        if b == b'"' && (i == 0 || bytes[i - 1] != b'\\') {
            in_string = !in_string;
            continue;
        }
        if in_string {
            continue;
        }

        match b {
            b'(' => depth += 1,
            b')' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => {}
        }
    }
    None
}

fn full_range(text: &str) -> TextRange {
    TextRange {
        start: Position {
            line: 0,
            character: 0,
        },
        end: end_position(text),
    }
}

fn end_position(text: &str) -> Position {
    let mut line = 0u32;
    let mut character = 0u32;
    for ch in text.chars() {
        if ch == '\n' {
            line += 1;
            character = 0;
        } else {
            character += 1;
        }
    }
    Position { line, character }
}

fn symbol_at_position(text: &str, position: Position) -> Option<(String, TextRange)> {
    let line_text = text.lines().nth(position.line as usize)?;
    let chars: Vec<char> = line_text.chars().collect();
    if chars.is_empty() {
        return None;
    }

    let mut idx = (position.character as usize).min(chars.len().saturating_sub(1));
    if !is_symbol_char(chars[idx]) {
        if idx > 0 && is_symbol_char(chars[idx - 1]) {
            idx -= 1;
        } else {
            return None;
        }
    }

    let mut left = idx;
    while left > 0 && is_symbol_char(chars[left - 1]) {
        left -= 1;
    }
    let mut right = idx;
    while right + 1 < chars.len() && is_symbol_char(chars[right + 1]) {
        right += 1;
    }

    let symbol: String = chars[left..=right].iter().collect();
    let range = TextRange {
        start: Position {
            line: position.line,
            character: left as u32,
        },
        end: Position {
            line: position.line,
            character: (right + 1) as u32,
        },
    };
    Some((symbol, range))
}

fn is_symbol_char(ch: char) -> bool {
    !ch.is_whitespace() && !matches!(ch, '(' | ')' | '[' | ']' | '{' | '}' | '"' | ';' | ',')
}

fn literal_type_at_position(text: &str, position: Position) -> Option<(String, TextRange)> {
    let offset = position_to_byte_offset(text, position)?;

    if let Some((start, end)) = find_enclosing_string_literal(text, offset) {
        return Some((
            "[Char]".to_string(),
            TextRange {
                start: byte_offset_to_position(text, start),
                end: byte_offset_to_position(text, end),
            },
        ));
    }
    if let Some((start, end)) = find_enclosing_char_literal(text, offset) {
        return Some((
            "Char".to_string(),
            TextRange {
                start: byte_offset_to_position(text, start),
                end: byte_offset_to_position(text, end),
            },
        ));
    }
    if let Some((token, start, end)) = numeric_token_at_offset(text, offset) {
        let typ = (if is_float_token(&token) {
            Some("Float")
        } else if is_int_token(&token) {
            Some("Int")
        } else {
            None
        })?;
        return Some((
            typ.to_string(),
            TextRange {
                start: byte_offset_to_position(text, start),
                end: byte_offset_to_position(text, end),
            },
        ));
    }
    None
}

fn format_literal_hover(text: &str, range: TextRange, literal_type: &str) -> String {
    if literal_type == "[Char]" {
        if let Some((preview, len, truncated)) = preview_string_literal(text, range, 100) {
            let suffix = if truncated { "..." } else { "" };
            return format!("\"{}{}\" : [Char]\nlen: {}", preview, suffix, len);
        }
    }
    let literal_text = text_for_range(text, range).unwrap_or_default();
    format!("{} : {}", literal_text, literal_type)
}

fn preview_string_literal(
    text: &str,
    range: TextRange,
    max_chars: usize
) -> Option<(String, usize, bool)> {
    let start = position_to_byte_offset(text, range.start)?;
    let end = position_to_byte_offset(text, range.end)?;
    let raw = text.get(start..end)?;
    let content = raw
        .strip_prefix('"')
        .and_then(|s| s.strip_suffix('"'))
        .unwrap_or(raw);

    let mut preview = String::new();
    let mut len = 0usize;
    let mut truncated = false;
    for ch in content.chars() {
        len += 1;
        if len <= max_chars {
            append_escaped_preview_char(&mut preview, ch);
        } else {
            truncated = true;
        }
    }
    Some((preview, len, truncated))
}

fn append_escaped_preview_char(out: &mut String, ch: char) {
    match ch {
        '\n' => out.push_str("\\n"),
        '\r' => out.push_str("\\r"),
        '\t' => out.push_str("\\t"),
        '"' => out.push_str("\\\""),
        '\\' => out.push_str("\\\\"),
        _ => out.push(ch),
    }
}

fn position_to_byte_offset(text: &str, position: Position) -> Option<usize> {
    let mut line = 0u32;
    let mut col = 0u32;
    for (idx, ch) in text.char_indices() {
        if line == position.line && col == position.character {
            return Some(idx);
        }
        if ch == '\n' {
            line += 1;
            col = 0;
        } else {
            col += 1;
        }
    }
    if line == position.line && col == position.character {
        Some(text.len())
    } else {
        None
    }
}

fn byte_offset_to_position(text: &str, target: usize) -> Position {
    let mut line = 0u32;
    let mut character = 0u32;
    for (idx, ch) in text.char_indices() {
        if idx >= target {
            break;
        }
        if ch == '\n' {
            line += 1;
            character = 0;
        } else {
            character += 1;
        }
    }
    Position { line, character }
}

fn text_for_range(text: &str, range: TextRange) -> Option<String> {
    let start = position_to_byte_offset(text, range.start)?;
    let end = position_to_byte_offset(text, range.end)?;
    text.get(start..end).map(|s| s.to_string())
}

fn find_enclosing_string_literal(text: &str, offset: usize) -> Option<(usize, usize)> {
    let bytes = text.as_bytes();
    let mut i = 0usize;
    let mut in_comment = false;
    let mut in_string = false;
    let mut start = 0usize;

    while i < bytes.len() {
        let b = bytes[i];
        if in_comment {
            if b == b'\n' {
                in_comment = false;
            }
            i += 1;
            continue;
        }
        if !in_string && b == b';' {
            in_comment = true;
            i += 1;
            continue;
        }
        if b == b'"' {
            if !in_string {
                in_string = true;
                start = i;
            } else {
                let end = i + 1;
                if offset >= start && offset < end {
                    return Some((start, end));
                }
                in_string = false;
            }
        }
        i += 1;
    }

    if in_string && offset >= start {
        return Some((start, bytes.len()));
    }
    None
}

fn find_enclosing_char_literal(text: &str, offset: usize) -> Option<(usize, usize)> {
    let bytes = text.as_bytes();
    let mut i = 0usize;
    let mut in_comment = false;
    let mut in_string = false;

    while i < bytes.len() {
        let b = bytes[i];
        if in_comment {
            if b == b'\n' {
                in_comment = false;
            }
            i += 1;
            continue;
        }
        if !in_string && b == b';' {
            in_comment = true;
            i += 1;
            continue;
        }
        if b == b'"' {
            in_string = !in_string;
            i += 1;
            continue;
        }
        if !in_string && b == b'\'' {
            let start = i;
            i += 1;
            while i < bytes.len() {
                if bytes[i] == b'\'' {
                    let end = i + 1;
                    if offset >= start && offset < end {
                        return Some((start, end));
                    }
                    i = end;
                    break;
                }
                if bytes[i] == b'\n' {
                    break;
                }
                i += 1;
            }
            continue;
        }
        i += 1;
    }
    None
}

fn numeric_token_at_offset(text: &str, offset: usize) -> Option<(String, usize, usize)> {
    if text.is_empty() {
        return None;
    }
    let bytes = text.as_bytes();
    let mut idx = offset.min(bytes.len().saturating_sub(1));
    if idx < bytes.len() && !is_numeric_token_byte(bytes[idx]) {
        if idx > 0 && is_numeric_token_byte(bytes[idx - 1]) {
            idx -= 1;
        } else {
            return None;
        }
    }
    let mut start = idx;
    while start > 0 && is_numeric_token_byte(bytes[start - 1]) {
        start -= 1;
    }
    let mut end = idx + 1;
    while end < bytes.len() && is_numeric_token_byte(bytes[end]) {
        end += 1;
    }
    let token = text.get(start..end)?.to_string();
    Some((token, start, end))
}

fn is_numeric_token_byte(b: u8) -> bool {
    b.is_ascii_digit() || b == b'.' || b == b'-'
}

fn is_int_token(token: &str) -> bool {
    let bytes = token.as_bytes();
    if bytes.is_empty() {
        return false;
    }
    let start = if bytes[0] == b'-' { 1 } else { 0 };
    start < bytes.len() && bytes[start..].iter().all(|b| b.is_ascii_digit())
}

fn is_float_token(token: &str) -> bool {
    let bytes = token.as_bytes();
    if bytes.is_empty() {
        return false;
    }
    let start = if bytes[0] == b'-' { 1 } else { 0 };
    if start >= bytes.len() {
        return false;
    }
    let slice = &bytes[start..];
    let dot_count = slice
        .iter()
        .filter(|&&b| b == b'.')
        .count();
    if dot_count != 1 {
        return false;
    }
    if !slice.iter().all(|b| b.is_ascii_digit() || *b == b'.') {
        return false;
    }
    let dot_idx = slice
        .iter()
        .position(|&b| b == b'.')
        .unwrap_or(0);
    let left = &slice[..dot_idx];
    let right = &slice[dot_idx + 1..];
    (!left.is_empty() || !right.is_empty()) &&
        left.iter().all(|b| b.is_ascii_digit()) &&
        right.iter().all(|b| b.is_ascii_digit())
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
