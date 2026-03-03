use lsp_server::{Connection, Message, Notification, Request, RequestId, Response};
use lsp_types::notification::{
    DidChangeTextDocument, DidCloseTextDocument, DidOpenTextDocument, PublishDiagnostics,
};
use lsp_types::notification::Notification as LspNotification;
use lsp_types::request::Request as LspRequest;
use lsp_types::request::{Completion, HoverRequest};
use lsp_types::{
    CompletionItem, CompletionItemKind, CompletionOptions, CompletionParams, CompletionResponse,
    Diagnostic, DiagnosticSeverity, DidChangeTextDocumentParams, DidCloseTextDocumentParams,
    DidOpenTextDocumentParams, Hover, HoverContents, HoverParams, HoverProviderCapability,
    InitializeParams, MarkupContent, MarkupKind, Position,
    PublishDiagnosticsParams, Range, ServerCapabilities, TextDocumentSyncCapability,
    TextDocumentSyncKind, Uri,
};
use que::infer::{infer_with_builtins_typed, TypedExpression};
use que::parser::Expression;
use que::types::{create_builtin_environment, Type, TypeEnv};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug)]
struct DocAnalysis {
    text: String,
    diagnostics: Vec<Diagnostic>,
    symbol_types: HashMap<String, String>,
    user_bound_symbols: HashSet<String>,
    form_scoped_symbols: Vec<FormScopedAnalysis>,
}

#[derive(Clone, Debug)]
struct FormScopedAnalysis {
    range: Range,
    symbol_types: HashMap<String, String>,
}

struct ServerState {
    connection: Connection,
    documents: HashMap<Uri, DocAnalysis>,
    std_defs: Vec<Expression>,
    base_env: TypeEnv,
    base_next_id: u64,
    global_signatures: HashMap<String, String>,
}

#[derive(Debug, Deserialize)]
struct GetSignatureParams {
    uri: Uri,
    symbol: String,
    position: Option<Position>,
}

#[derive(Debug, Serialize)]
struct GetSignatureResult {
    signature: Option<String>,
}

fn main() {
    if let Err(err) = run() {
        eprintln!("quelsp error: {}", err);
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let (connection, io_threads) = Connection::stdio();

    let server_capabilities = ServerCapabilities {
        text_document_sync: Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::FULL)),
        hover_provider: Some(HoverProviderCapability::Simple(true)),
        completion_provider: Some(CompletionOptions {
            resolve_provider: Some(false),
            trigger_characters: Some(vec![
                "/".to_string(),
                "!".to_string(),
                "?".to_string(),
                "-".to_string(),
                "+".to_string(),
                "*".to_string(),
                ".".to_string(),
            ]),
            ..CompletionOptions::default()
        }),
        ..ServerCapabilities::default()
    };

    let init_value = serde_json::to_value(server_capabilities)
        .map_err(|e| format!("serialize server capabilities: {}", e))?;
    let _init_params: InitializeParams = serde_json::from_value(
        connection
            .initialize(init_value)
            .map_err(|e| format!("initialize failed: {}", e))?,
    )
    .map_err(|e| format!("parse initialize params: {}", e))?;

    let std_defs = load_std_definitions();
    let (base_env, base_next_id, global_signatures) = build_base_environment(&std_defs);
    let mut state = ServerState {
        connection,
        documents: HashMap::new(),
        std_defs,
        base_env,
        base_next_id,
        global_signatures,
    };

    while let Ok(msg) = state.connection.receiver.recv() {
        match msg {
            Message::Request(req) => {
                if state
                    .connection
                    .handle_shutdown(&req)
                    .map_err(|e| format!("shutdown handling failed: {}", e))?
                {
                    break;
                }
                state.handle_request(req)?;
            }
            Message::Notification(notif) => state.handle_notification(notif)?,
            Message::Response(_) => {}
        }
    }

    io_threads
        .join()
        .map_err(|e| format!("failed to join io threads: {}", e))?;
    Ok(())
}

impl ServerState {
    fn handle_request(&mut self, req: Request) -> Result<(), String> {
        match req.method.as_str() {
            HoverRequest::METHOD => {
                let params: HoverParams = parse_params(req.params)?;
                let result = self.hover_for_document(&params);
                self.reply_ok(req.id, result)
            }
            Completion::METHOD => {
                let params: CompletionParams = parse_params(req.params)?;
                let items = self.completion_items_for_document(&params);
                self.reply_ok(req.id, Some(CompletionResponse::Array(items)))
            }
            "que/getSignature" => {
                let params: GetSignatureParams = parse_params(req.params)?;
                let signature = self.signature_for_symbol(&params.uri, &params.symbol, params.position);
                self.reply_ok(req.id, Some(GetSignatureResult { signature }))
            }
            _ => self.reply_ok::<Value>(req.id, None),
        }
    }

    fn handle_notification(&mut self, notif: Notification) -> Result<(), String> {
        match notif.method.as_str() {
            DidOpenTextDocument::METHOD => {
                let params: DidOpenTextDocumentParams = parse_params(notif.params)?;
                let uri = params.text_document.uri;
                let analysis = analyze_document_text(
                    &params.text_document.text,
                    &self.std_defs,
                    &self.base_env,
                    self.base_next_id,
                );
                self.publish_diagnostics(uri.clone(), Some(params.text_document.version), &analysis.diagnostics)?;
                self.documents.insert(uri, analysis);
                Ok(())
            }
            DidChangeTextDocument::METHOD => {
                let params: DidChangeTextDocumentParams = parse_params(notif.params)?;
                let Some(last_change) = params.content_changes.last() else {
                    return Ok(());
                };

                let uri = params.text_document.uri;
                let analysis = analyze_document_text(
                    &last_change.text,
                    &self.std_defs,
                    &self.base_env,
                    self.base_next_id,
                );
                self.publish_diagnostics(uri.clone(), Some(params.text_document.version), &analysis.diagnostics)?;
                self.documents.insert(uri, analysis);
                Ok(())
            }
            DidCloseTextDocument::METHOD => {
                let params: DidCloseTextDocumentParams = parse_params(notif.params)?;
                self.documents.remove(&params.text_document.uri);
                self.publish_diagnostics(params.text_document.uri, None, &[])?;
                Ok(())
            }
            _ => Ok(()),
        }
    }

    fn publish_diagnostics(
        &self,
        uri: Uri,
        version: Option<i32>,
        diagnostics: &[Diagnostic],
    ) -> Result<(), String> {
        let params = PublishDiagnosticsParams {
            uri,
            version,
            diagnostics: diagnostics.to_vec(),
        };
        let notif = Notification::new(PublishDiagnostics::METHOD.to_string(), params);
        self.connection
            .sender
            .send(Message::Notification(notif))
            .map_err(|e| format!("send diagnostics failed: {}", e))
    }

    fn reply_ok<T: serde::Serialize>(&self, id: RequestId, result: Option<T>) -> Result<(), String> {
        let value = serde_json::to_value(result)
            .map_err(|e| format!("serialize response payload failed: {}", e))?;
        let response = Response::new_ok(id, value);
        self.connection
            .sender
            .send(Message::Response(response))
            .map_err(|e| format!("send response failed: {}", e))
    }

    fn hover_for_document(&self, params: &HoverParams) -> Option<Hover> {
        let uri = &params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;
        let doc = self.documents.get(uri)?;
        let (symbol, symbol_range) = symbol_at_position(&doc.text, position)?;
        let type_info = self.resolve_signature_for_doc(doc, &symbol, Some(position))?;

        let value = format!("```que\n{} : {}\n```", symbol, type_info);
        Some(Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value,
            }),
            range: Some(symbol_range),
        })
    }

    fn completion_items_for_document(&self, params: &CompletionParams) -> Vec<CompletionItem> {
        let uri = &params.text_document_position.text_document.uri;
        let position = params.text_document_position.position;
        let mut merged_signatures: HashMap<String, String> = HashMap::new();
        let mut items = Vec::new();

        for keyword in ["lambda", "if", "let", "let*", "do", "as"] {
            items.push(CompletionItem {
                label: keyword.to_string(),
                kind: Some(CompletionItemKind::KEYWORD),
                ..CompletionItem::default()
            });
        }

        for (name, signature) in &self.global_signatures {
            merged_signatures.insert(name.clone(), signature.clone());
        }

        if let Some(doc) = self.documents.get(uri) {
            for (name, signature) in &doc.symbol_types {
                let should_override =
                    doc.user_bound_symbols.contains(name) || !self.global_signatures.contains_key(name);
                if should_override {
                    merged_signatures.insert(name.clone(), signature.clone());
                }
            }

            if let Some(form_signatures) = self.form_signatures_at(doc, position) {
                for (name, signature) in form_signatures {
                    merged_signatures.insert(name.clone(), signature.clone());
                }
            }
        }

        for (name, signature) in merged_signatures {
            items.push(CompletionItem {
                label: name,
                detail: Some(signature.clone()),
                kind: Some(kind_for_signature(&signature)),
                ..CompletionItem::default()
            });
        }

        items.sort_by(|a, b| a.label.cmp(&b.label));
        items
    }

    fn signature_for_symbol(&self, uri: &Uri, symbol: &str, position: Option<Position>) -> Option<String> {
        if let Some(doc) = self.documents.get(uri) {
            return self.resolve_signature_for_doc(doc, symbol, position).cloned();
        }
        self.global_signatures.get(symbol).cloned()
    }

    fn resolve_signature_for_doc<'a>(
        &'a self,
        doc: &'a DocAnalysis,
        symbol: &str,
        position: Option<Position>,
    ) -> Option<&'a String> {
        let scoped_sig = position
            .and_then(|pos| self.form_signatures_at(doc, pos))
            .and_then(|symbols| symbols.get(symbol));
        let doc_sig = doc.symbol_types.get(symbol);
        let global_sig = self.global_signatures.get(symbol);

        if doc.user_bound_symbols.contains(symbol) {
            scoped_sig.or(doc_sig).or(global_sig)
        } else {
            global_sig.or(scoped_sig).or(doc_sig)
        }
    }

    fn form_signatures_at<'a>(
        &'a self,
        doc: &'a DocAnalysis,
        position: Position,
    ) -> Option<&'a HashMap<String, String>> {
        doc.form_scoped_symbols
            .iter()
            .find(|form| range_contains_position(&form.range, position))
            .map(|form| &form.symbol_types)
    }
}

fn parse_params<T: serde::de::DeserializeOwned>(params: Value) -> Result<T, String> {
    serde_json::from_value(params).map_err(|e| format!("invalid params: {}", e))
}

fn normalize_signature(signature: &str) -> String {
    let trimmed = signature.trim();
    let without_quantifier = if let Some(dot_idx) = trimmed.find('.') {
        let prefix = trimmed[..dot_idx].trim();
        let looks_like_quantifier = !prefix.is_empty() &&
            prefix
                .split_whitespace()
                .all(|tok| tok.starts_with('T') && tok[1..].chars().all(|c| c.is_ascii_digit()));
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

fn strip_type_var_numbers(input: &str) -> String {
    let chars: Vec<char> = input.chars().collect();
    let mut out = String::with_capacity(input.len());
    let mut i = 0usize;
    while i < chars.len() {
        if chars[i] == 'T' {
            let mut j = i + 1;
            while j < chars.len() && chars[j].is_ascii_digit() {
                j += 1;
            }
            if j > i + 1 {
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
        signatures.insert(name, normalize_signature(&signature));
    }

    (env, next_id, signatures)
}

fn load_std_definitions() -> Vec<Expression> {
    let ast = que::baked::load_ast();
    if let Expression::Apply(items) = ast {
        return items.into_iter().skip(1).collect();
    }
    Vec::new()
}

fn infer_std_signatures(
    base_env: &TypeEnv,
    base_next_id: u64,
    std_defs: &[Expression],
) -> HashMap<String, String> {
    let mut raw_signatures: HashMap<String, Type> = HashMap::new();
    if std_defs.is_empty() {
        return HashMap::new();
    }

    let std_program = Expression::Apply(
        std::iter::once(Expression::Word("do".to_string()))
            .chain(std_defs.iter().cloned())
            .collect(),
    );

    if let Ok((_typ, typed)) = infer_with_builtins_typed(&std_program, (base_env.clone(), base_next_id)) {
        collect_let_binding_types(&typed, &mut raw_signatures);
    }

    raw_signatures
        .into_iter()
        .map(|(name, typ)| (name, normalize_signature(&typ.to_string())))
        .collect()
}

fn collect_let_binding_types(node: &TypedExpression, signatures: &mut HashMap<String, Type>) {
    if let Expression::Apply(items) = &node.expr {
        if let [Expression::Word(keyword), Expression::Word(name), _rhs, ..] = &items[..] {
            if keyword == "let" || keyword == "let*" {
                if let Some(rhs_type) = node
                    .children
                    .get(2)
                    .and_then(|child| child.typ.as_ref())
                {
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

fn analyze_document_text(
    text: &str,
    std_defs: &[Expression],
    base_env: &TypeEnv,
    base_next_id: u64,
) -> DocAnalysis {
    let mut diagnostics = Vec::new();
    let mut symbol_types_raw: HashMap<String, Type> = HashMap::new();
    let mut user_bound_symbols = HashSet::new();
    let mut form_scoped_symbols = Vec::new();

    let mut analysis_source = text.to_string();
    let parsed_exprs = match que::parser::parse(text) {
        Ok(exprs) => exprs,
        Err(parse_err) => {
            diagnostics.push(parse_error_to_diagnostic(text, &parse_err));
            let repaired = repair_source_for_analysis(text);
            match que::parser::parse(&repaired) {
                Ok(exprs) => {
                    analysis_source = repaired;
                    exprs
                }
                Err(_) => {
                    return DocAnalysis {
                        text: text.to_string(),
                        diagnostics,
                        symbol_types: HashMap::new(),
                        user_bound_symbols,
                        form_scoped_symbols,
                    };
                }
            }
        }
    };
    collect_user_bound_symbols_from_exprs(&parsed_exprs, &mut user_bound_symbols);

    let program = match que::parser::merge_std_and_program(&analysis_source, std_defs.to_vec()) {
        Ok(expr) => expr,
        Err(err) => {
            diagnostics.push(make_error_diagnostic(text, err));
            return DocAnalysis {
                text: text.to_string(),
                diagnostics,
                symbol_types: HashMap::new(),
                user_bound_symbols,
                form_scoped_symbols,
            };
        }
    };

    match infer_with_builtins_typed(&program, (base_env.clone(), base_next_id)) {
        Ok((_typ, typed)) => {
            collect_symbol_types(&typed, &mut symbol_types_raw);
            form_scoped_symbols = build_form_scoped_analyses(text, parsed_exprs.len(), &typed);
        }
        Err(err) => diagnostics.push(make_error_diagnostic(text, err)),
    }

    let symbol_types: HashMap<String, String> = symbol_types_raw
        .into_iter()
        .map(|(name, typ)| (name, normalize_signature(&typ.to_string())))
        .collect();

    DocAnalysis {
        text: text.to_string(),
        diagnostics,
        symbol_types,
        user_bound_symbols,
        form_scoped_symbols,
    }
}

fn build_form_scoped_analyses(
    text: &str,
    user_form_count: usize,
    typed_program: &TypedExpression,
) -> Vec<FormScopedAnalysis> {
    if user_form_count == 0 {
        return Vec::new();
    }

    let form_ranges = top_level_form_ranges(text);
    let typed_user_forms = extract_user_top_level_typed_forms(typed_program, user_form_count);
    let count = form_ranges.len().min(typed_user_forms.len());
    if count == 0 {
        return Vec::new();
    }

    let mut out = Vec::with_capacity(count);
    for idx in 0..count {
        let mut raw_symbols: HashMap<String, Type> = HashMap::new();
        collect_symbol_types(typed_user_forms[idx], &mut raw_symbols);
        let symbol_types = raw_symbols
            .into_iter()
            .map(|(name, typ)| (name, normalize_signature(&typ.to_string())))
            .collect();
        out.push(FormScopedAnalysis {
            range: form_ranges[idx],
            symbol_types,
        });
    }

    out
}

fn extract_user_top_level_typed_forms<'a>(
    typed_program: &'a TypedExpression,
    user_form_count: usize,
) -> Vec<&'a TypedExpression> {
    if let Expression::Apply(_) = &typed_program.expr {
        if typed_program.children.len() <= 1 {
            return Vec::new();
        }

        let forms = &typed_program.children[1..];
        let start = forms.len().saturating_sub(user_form_count);
        return forms[start..].iter().collect();
    }

    Vec::new()
}

fn top_level_form_ranges(text: &str) -> Vec<Range> {
    let bytes = text.as_bytes();
    let mut ranges = Vec::new();
    let mut i = 0usize;

    while i < bytes.len() {
        while i < bytes.len() {
            if bytes[i].is_ascii_whitespace() {
                i += 1;
                continue;
            }
            if bytes[i] == b';' {
                while i < bytes.len() && bytes[i] != b'\n' {
                    i += 1;
                }
                continue;
            }
            break;
        }

        if i >= bytes.len() {
            break;
        }

        let start = i;
        match bytes[i] {
            b'(' | b'[' => {
                let end = find_matching_list_end_byte(text, i)
                    .map(|idx| idx + 1)
                    .unwrap_or(text.len());
                ranges.push(Range::new(
                    byte_offset_to_position(text, start),
                    byte_offset_to_position(text, end),
                ));
                i = end;
            }
            b'"' => {
                i += 1;
                while i < bytes.len() {
                    if bytes[i] == b'"' && (i == 0 || bytes[i - 1] != b'\\') {
                        i += 1;
                        break;
                    }
                    i += 1;
                }
                ranges.push(Range::new(
                    byte_offset_to_position(text, start),
                    byte_offset_to_position(text, i),
                ));
            }
            _ => {
                i += 1;
                while i < bytes.len() && !bytes[i].is_ascii_whitespace() && bytes[i] != b';' {
                    i += 1;
                }
                ranges.push(Range::new(
                    byte_offset_to_position(text, start),
                    byte_offset_to_position(text, i),
                ));
            }
        }
    }

    ranges
}

fn find_matching_list_end_byte(text: &str, open_idx: usize) -> Option<usize> {
    let bytes = text.as_bytes();
    let open = *bytes.get(open_idx)?;
    if open != b'(' && open != b'[' {
        return None;
    }

    let mut stack = vec![if open == b'(' { b')' } else { b']' }];
    let mut in_string = false;
    let mut in_comment = false;
    let mut i = open_idx + 1;

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

        if b == b'"' && (i == 0 || bytes[i - 1] != b'\\') {
            in_string = !in_string;
            i += 1;
            continue;
        }

        if in_string {
            i += 1;
            continue;
        }

        match b {
            b'(' => stack.push(b')'),
            b'[' => stack.push(b']'),
            b')' | b']' => {
                if let Some(expected) = stack.pop() {
                    if b != expected {
                        return None;
                    }
                    if stack.is_empty() {
                        return Some(i);
                    }
                } else {
                    return None;
                }
            }
            _ => {}
        }

        i += 1;
    }

    None
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
        _ => {
            let existing_score = type_specificity_score(existing);
            let candidate_score = type_specificity_score(candidate);
            candidate_score > existing_score
        }
    }
}

fn parse_error_to_diagnostic(text: &str, message: &str) -> Diagnostic {
    if let Some(token_index) = extract_parse_error_token_index(message) {
        let spans = tokenize_with_ranges(text);
        if let Some((start, end)) = spans.get(token_index) {
            return Diagnostic {
                range: Range::new(*start, *end),
                severity: Some(DiagnosticSeverity::ERROR),
                message: message.to_string(),
                source: Some("que-parse".to_string()),
                ..Diagnostic::default()
            };
        }
    }
    make_error_diagnostic(text, message.to_string())
}

fn make_error_diagnostic(text: &str, message: String) -> Diagnostic {
    let inferred_range = infer_error_range(text, &message);
    let range = inferred_range.unwrap_or_else(|| full_range(text));
    let display_message = if inferred_range.is_some() {
        first_error_line(&message).to_string()
    } else {
        message
    };

    Diagnostic {
        range,
        severity: Some(DiagnosticSeverity::ERROR),
        message: display_message,
        source: Some("que".to_string()),
        ..Diagnostic::default()
    }
}

fn first_error_line(message: &str) -> &str {
    message.lines().next().map(str::trim).filter(|line| !line.is_empty()).unwrap_or(message)
}

fn infer_error_range(text: &str, message: &str) -> Option<Range> {
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

fn find_invalid_char_literal_range(text: &str) -> Option<Range> {
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
                return Some(Range::new(
                    byte_offset_to_position(text, start),
                    byte_offset_to_position(text, (start + 1).min(text.len())),
                ));
            }

            if char_count != 1 {
                return Some(Range::new(
                    byte_offset_to_position(text, start),
                    byte_offset_to_position(text, i),
                ));
            }

            continue;
        }

        i += ch_len;
    }

    None
}

fn find_snippet_range(text: &str, snippet: &str) -> Option<Range> {
    let start = text.find(snippet)?;
    let end = start + snippet.len();
    Some(Range::new(
        byte_offset_to_position(text, start),
        byte_offset_to_position(text, end),
    ))
}

fn find_call_prefix_range(text: &str, snippet: &str) -> Option<Range> {
    let prefix_tokens = extract_call_prefix_tokens(snippet, 3);
    if prefix_tokens.is_empty() {
        return None;
    }
    let bytes = text.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        if bytes[i] == b'(' && match_call_prefix_at(text, i, &prefix_tokens) {
            if let Some(close) = find_matching_paren_byte(text, i) {
                return Some(Range::new(
                    byte_offset_to_position(text, i),
                    byte_offset_to_position(text, close + 1),
                ));
            }
        }
        i += 1;
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

fn find_first_call_range(text: &str) -> Option<Range> {
    let bytes = text.as_bytes();
    let mut i = 0usize;

    while i < bytes.len() {
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
                if b.is_ascii_whitespace() || matches!(b, b'(' | b')' | b'[' | b']' | b'"' | b';')
                {
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
                return Some(Range::new(
                    byte_offset_to_position(text, open),
                    byte_offset_to_position(text, close + 1),
                ));
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

fn find_symbol_range(text: &str, symbol: &str) -> Option<Range> {
    if symbol.is_empty() {
        return None;
    }
    let bytes = text.as_bytes();
    let needle = symbol.as_bytes();
    if needle.len() > bytes.len() {
        return None;
    }

    let mut i = 0usize;
    while i + needle.len() <= bytes.len() {
        if &bytes[i..i + needle.len()] == needle {
            let left_ok = i == 0 || !is_ident_char(bytes[i - 1]);
            let right_ok = i + needle.len() == bytes.len() || !is_ident_char(bytes[i + needle.len()]);
            if left_ok && right_ok {
                return Some(Range::new(
                    byte_offset_to_position(text, i),
                    byte_offset_to_position(text, i + needle.len()),
                ));
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

fn byte_offset_to_position(text: &str, target: usize) -> Position {
    let mut line = 0u32;
    let mut col = 0u32;
    let mut prev_idx = 0usize;

    for (idx, ch) in text.char_indices() {
        if idx >= target {
            break;
        }
        prev_idx = idx;
        if ch == '\n' {
            line += 1;
            col = 0;
        } else {
            col += 1;
        }
    }

    if target > text.len() {
        return end_position(text);
    }
    if target == text.len() && !text.is_empty() {
        let last_ch = text[prev_idx..].chars().next().unwrap_or('\0');
        if last_ch != '\n' {
            // current (line, col) already points at end-of-text
        }
    }
    Position::new(line, col)
}

fn full_range(text: &str) -> Range {
    Range::new(Position::new(0, 0), end_position(text))
}

fn end_position(text: &str) -> Position {
    let mut line = 0_u32;
    let mut character = 0_u32;
    for ch in text.chars() {
        if ch == '\n' {
            line += 1;
            character = 0;
        } else {
            character += 1;
        }
    }
    Position::new(line, character)
}

fn range_contains_position(range: &Range, position: Position) -> bool {
    position_at_or_after(position, range.start) && position_before(position, range.end)
}

fn position_at_or_after(a: Position, b: Position) -> bool {
    a.line > b.line || (a.line == b.line && a.character >= b.character)
}

fn position_before(a: Position, b: Position) -> bool {
    a.line < b.line || (a.line == b.line && a.character < b.character)
}

fn extract_parse_error_token_index(message: &str) -> Option<usize> {
    let prefix = "Error parsing expression at token index ";
    let rest = message.strip_prefix(prefix)?;
    let number = rest.split(':').next()?.trim();
    number.parse::<usize>().ok()
}

fn tokenize_with_ranges(text: &str) -> Vec<(Position, Position)> {
    let mut tokens = Vec::new();
    let mut line = 0_u32;
    let mut col = 0_u32;

    let mut buf_start: Option<Position> = None;
    let mut buf_len: u32 = 0;

    let flush_buffer = |tokens: &mut Vec<(Position, Position)>,
                        buf_start: &mut Option<Position>,
                        buf_len: &mut u32| {
        if let Some(start) = *buf_start {
            let end = Position::new(start.line, start.character + *buf_len);
            tokens.push((start, end));
            *buf_start = None;
            *buf_len = 0;
        }
    };

    let chars: Vec<char> = text.chars().collect();
    let mut i = 0_usize;
    while i < chars.len() {
        let ch = chars[i];
        match ch {
            ';' => {
                flush_buffer(&mut tokens, &mut buf_start, &mut buf_len);
                while i < chars.len() && chars[i] != '\n' {
                    i += 1;
                    col += 1;
                }
                continue;
            }
            '(' | ')' => {
                flush_buffer(&mut tokens, &mut buf_start, &mut buf_len);
                let start = Position::new(line, col);
                let end = Position::new(line, col + 1);
                tokens.push((start, end));
                i += 1;
                col += 1;
                continue;
            }
            '\n' => {
                flush_buffer(&mut tokens, &mut buf_start, &mut buf_len);
                i += 1;
                line += 1;
                col = 0;
                continue;
            }
            c if c.is_whitespace() => {
                flush_buffer(&mut tokens, &mut buf_start, &mut buf_len);
                i += 1;
                col += 1;
                continue;
            }
            _ => {
                if buf_start.is_none() {
                    buf_start = Some(Position::new(line, col));
                }
                buf_len += 1;
                i += 1;
                col += 1;
            }
        }
    }
    flush_buffer(&mut tokens, &mut buf_start, &mut buf_len);
    tokens
}

fn kind_for_signature(signature: &str) -> CompletionItemKind {
    if signature.contains("->") {
        CompletionItemKind::FUNCTION
    } else {
        CompletionItemKind::CONSTANT
    }
}

fn symbol_at_position(text: &str, position: Position) -> Option<(String, Range)> {
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
    if symbol.is_empty() {
        return None;
    }

    let range = Range::new(
        Position::new(position.line, left as u32),
        Position::new(position.line, (right + 1) as u32),
    );
    Some((symbol, range))
}

fn is_symbol_char(ch: char) -> bool {
    !ch.is_whitespace() && !matches!(ch, '(' | ')' | '[' | ']' | '{' | '}' | '"' | ';' | ',')
}
