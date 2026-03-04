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
use std::panic::{catch_unwind, AssertUnwindSafe};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug)]
struct DocAnalysis {
    text: String,
    diagnostics: Vec<Diagnostic>,
    symbol_types: HashMap<String, String>,
    let_binding_types: HashMap<String, String>,
    user_bound_symbols: HashSet<String>,
    form_scoped_symbols: Vec<FormScopedAnalysis>,
}

#[derive(Clone, Debug)]
struct FormScopedAnalysis {
    range: Range,
    symbol_types: HashMap<String, String>,
    let_binding_types: HashMap<String, String>,
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
                let analysis = analyze_document_text_safe(
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
                let analysis = analyze_document_text_safe(
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

        if let Some((literal_type, literal_range)) = literal_type_at_position(&doc.text, position) {
            let value = format_literal_hover(&doc.text, literal_range, &literal_type);
            return Some(Hover {
                contents: HoverContents::Markup(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value,
                }),
                range: Some(literal_range),
            });
        }

        let (symbol, symbol_range) = symbol_at_position(&doc.text, position)?;
        if is_let_binding_name_at_position(&doc.text, symbol_range, &symbol) {
            let declaration_type = self
                .form_let_signatures_at(doc, position)
                .and_then(|m| m.get(&symbol))
                .or_else(|| doc.let_binding_types.get(&symbol))
                .or_else(|| self.global_signatures.get(&symbol))?;
            let declaration_type = normalize_signature(declaration_type);

            let value = format!("```que\n{} : {}\n```", symbol, declaration_type);
            return Some(Hover {
                contents: HoverContents::Markup(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value,
                }),
                range: Some(symbol_range),
            });
        }

        let type_info = self.resolve_signature_for_doc(doc, &symbol, Some(position))?;
        let type_info = normalize_signature(type_info);

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
                detail: Some(normalize_signature(&signature)),
                kind: Some(kind_for_signature(&signature)),
                ..CompletionItem::default()
            });
        }

        items.sort_by(|a, b| a.label.cmp(&b.label));
        items
    }

    fn signature_for_symbol(&self, uri: &Uri, symbol: &str, position: Option<Position>) -> Option<String> {
        if let Some(doc) = self.documents.get(uri) {
            return self
                .resolve_signature_for_doc(doc, symbol, position)
                .map(|s| normalize_signature(s));
        }
        self.global_signatures
            .get(symbol)
            .map(|s| normalize_signature(s))
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

    fn form_let_signatures_at<'a>(
        &'a self,
        doc: &'a DocAnalysis,
        position: Position,
    ) -> Option<&'a HashMap<String, String>> {
        doc.form_scoped_symbols
            .iter()
            .find(|form| range_contains_position(&form.range, position))
            .map(|form| &form.let_binding_types)
    }
}

fn parse_params<T: serde::de::DeserializeOwned>(params: Value) -> Result<T, String> {
    serde_json::from_value(params).map_err(|e| format!("invalid params: {}", e))
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
    let mut let_binding_types_raw: HashMap<String, Type> = HashMap::new();
    let mut user_bound_symbols = HashSet::new();
    let mut form_scoped_symbols = Vec::new();
    let analysis_source = strip_comment_bodies_preserve_newlines(text);
    let parsed_exprs = parse_user_exprs_for_symbol_collection(&analysis_source);
    if let Some(exprs) = &parsed_exprs {
        collect_user_bound_symbols_from_exprs(exprs, &mut user_bound_symbols);
    }
    let user_form_count = parsed_exprs
        .as_ref()
        .map(|exprs| exprs.len())
        .unwrap_or_else(|| top_level_form_ranges(text).len());

    let program = match que::parser::merge_std_and_program(&analysis_source, std_defs.to_vec()) {
        Ok(expr) => expr,
        Err(primary_err) => {
            let repaired = repair_source_for_analysis(&analysis_source);
            match que::parser::merge_std_and_program(&repaired, std_defs.to_vec()) {
                Ok(expr) => expr,
                Err(_) => {
                    diagnostics.push(make_error_diagnostic(text, primary_err));
                    return DocAnalysis {
                        text: text.to_string(),
                        diagnostics,
                        symbol_types: HashMap::new(),
                        let_binding_types: HashMap::new(),
                        user_bound_symbols,
                        form_scoped_symbols,
                    };
                }
            }
        }
    };

    match infer_with_builtins_typed(&program, (base_env.clone(), base_next_id)) {
        Ok((_typ, typed)) => {
            collect_symbol_types(&typed, &mut symbol_types_raw);
            collect_let_binding_types(&typed, &mut let_binding_types_raw);
            form_scoped_symbols = build_form_scoped_analyses(text, user_form_count, &typed);
        }
        Err(err) => diagnostics.push(make_error_diagnostic(text, err)),
    }

    let symbol_types: HashMap<String, String> = symbol_types_raw
        .into_iter()
        .map(|(name, typ)| (name, normalize_signature(&typ.to_string())))
        .collect();
    let let_binding_types: HashMap<String, String> = let_binding_types_raw
        .into_iter()
        .map(|(name, typ)| (name, normalize_signature(&typ.to_string())))
        .collect();

    DocAnalysis {
        text: text.to_string(),
        diagnostics,
        symbol_types,
        let_binding_types,
        user_bound_symbols,
        form_scoped_symbols,
    }
}

fn parse_user_exprs_for_symbol_collection(text: &str) -> Option<Vec<Expression>> {
    let masked = mask_literals_for_structural_parse(text);
    if let Ok(exprs) = que::parser::parse(&masked) {
        return Some(exprs);
    }

    let repaired_masked = repair_source_for_analysis(&masked);
    que::parser::parse(&repaired_masked).ok()
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
    let mut string_id = 0usize;
    let mut char_id = 0usize;

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
                out.push_str(&format!("__STR{}__", string_id));
                out.push(' ');
                string_id += 1;
                in_string = true;
            }
            '\'' => {
                out.push(' ');
                out.push_str(&format!("__CHR{}__", char_id));
                out.push(' ');
                char_id += 1;
                in_char = true;
            }
            _ => out.push(ch),
        }
    }

    out
}

fn analyze_document_text_safe(
    text: &str,
    std_defs: &[Expression],
    base_env: &TypeEnv,
    base_next_id: u64,
) -> DocAnalysis {
    match catch_unwind(AssertUnwindSafe(|| {
        analyze_document_text(text, std_defs, base_env, base_next_id)
    })) {
        Ok(analysis) => analysis,
        Err(payload) => {
            let message = format!(
                "Internal parser/inference error: {}",
                panic_payload_to_string(payload)
            );
            DocAnalysis {
                text: text.to_string(),
                diagnostics: vec![make_error_diagnostic(text, message)],
                symbol_types: HashMap::new(),
                let_binding_types: HashMap::new(),
                user_bound_symbols: HashSet::new(),
                form_scoped_symbols: Vec::new(),
            }
        }
    }
}

fn panic_payload_to_string(payload: Box<dyn std::any::Any + Send>) -> String {
    if let Some(msg) = payload.downcast_ref::<&str>() {
        return (*msg).to_string();
    }
    if let Some(msg) = payload.downcast_ref::<String>() {
        return msg.clone();
    }
    "unknown panic".to_string()
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
        let mut raw_let_bindings: HashMap<String, Type> = HashMap::new();
        collect_symbol_types(typed_user_forms[idx], &mut raw_symbols);
        collect_let_binding_types(typed_user_forms[idx], &mut raw_let_bindings);
        let symbol_types = raw_symbols
            .into_iter()
            .map(|(name, typ)| (name, normalize_signature(&typ.to_string())))
            .collect();
        let let_binding_types = raw_let_bindings
            .into_iter()
            .map(|(name, typ)| (name, normalize_signature(&typ.to_string())))
            .collect();
        out.push(FormScopedAnalysis {
            range: form_ranges[idx],
            symbol_types,
            let_binding_types,
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

fn make_error_diagnostic(text: &str, message: String) -> Diagnostic {
    let normalized_message = strip_type_var_numbers(&message);
    let inferred_range = infer_error_range(text, &message);
    let range = inferred_range.unwrap_or_else(|| full_range(text));
    let display_message = if inferred_range.is_some() {
        first_error_line(&normalized_message).to_string()
    } else {
        normalized_message
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
            return Some(Range::new(
                byte_offset_to_position(text, start),
                byte_offset_to_position(text, end),
            ));
        }
        i += 1;
    }
    None
}

fn find_call_prefix_range(text: &str, snippet: &str) -> Option<Range> {
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
                    return Some(Range::new(
                        byte_offset_to_position(text, i),
                        byte_offset_to_position(text, close + 1),
                    ));
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

fn find_first_call_range(text: &str) -> Option<Range> {
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

fn range_contains_position(range: &Range, position: Position) -> bool {
    position_at_or_after(position, range.start) && position_before(position, range.end)
}

fn position_at_or_after(a: Position, b: Position) -> bool {
    a.line > b.line || (a.line == b.line && a.character >= b.character)
}

fn position_before(a: Position, b: Position) -> bool {
    a.line < b.line || (a.line == b.line && a.character < b.character)
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

fn is_let_binding_name_at_position(text: &str, symbol_range: Range, symbol: &str) -> bool {
    let Some(symbol_start) = position_to_byte_offset(text, symbol_range.start) else {
        return false;
    };
    let Some(symbol_end) = position_to_byte_offset(text, symbol_range.end) else {
        return false;
    };
    let Some(open) = find_enclosing_open_paren_before(text, symbol_start) else {
        return false;
    };
    let Some(close) = find_matching_paren_byte(text, open) else {
        return false;
    };
    if symbol_end > close + 1 {
        return false;
    }

    let Some((head, _head_start, _head_end, after_head)) =
        read_top_level_atom_token_in_list(text, open + 1, close)
    else {
        return false;
    };
    if head != "let" && head != "let*" {
        return false;
    }

    let Some((name, name_start, name_end, _)) =
        read_top_level_atom_token_in_list(text, after_head, close)
    else {
        return false;
    };

    name == symbol && name_start == symbol_start && name_end == symbol_end
}

fn find_enclosing_open_paren_before(text: &str, target: usize) -> Option<usize> {
    let bytes = text.as_bytes();
    let end = target.min(bytes.len());
    let mut stack: Vec<(u8, usize)> = Vec::new();
    let mut i = 0usize;
    let mut in_string = false;
    let mut in_comment = false;

    while i < end {
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
            b'(' | b'[' => stack.push((b, i)),
            b')' | b']' => {
                let _ = stack.pop();
            }
            _ => {}
        }
        i += 1;
    }

    for (ch, idx) in stack.iter().rev() {
        if *ch == b'(' {
            return Some(*idx);
        }
    }
    None
}

fn read_top_level_atom_token_in_list(
    text: &str,
    mut i: usize,
    list_close: usize,
) -> Option<(String, usize, usize, usize)> {
    let bytes = text.as_bytes();
    i = skip_ws_and_comments(text, i, list_close);
    if i >= list_close || i >= bytes.len() {
        return None;
    }

    if matches!(bytes[i], b'(' | b')' | b'[' | b']' | b'"') {
        return None;
    }

    let start = i;
    while i < list_close && i < bytes.len() {
        let b = bytes[i];
        if b.is_ascii_whitespace() || matches!(b, b';' | b'(' | b')' | b'[' | b']' | b'"') {
            break;
        }
        i += 1;
    }
    if i <= start {
        return None;
    }

    let token = text.get(start..i)?.to_string();
    let next = skip_ws_and_comments(text, i, list_close);
    Some((token, start, i, next))
}

fn skip_ws_and_comments(text: &str, mut i: usize, end: usize) -> usize {
    let bytes = text.as_bytes();
    while i < end && i < bytes.len() {
        if bytes[i].is_ascii_whitespace() {
            i += 1;
            continue;
        }
        if bytes[i] == b';' {
            while i < end && i < bytes.len() && bytes[i] != b'\n' {
                i += 1;
            }
            continue;
        }
        break;
    }
    i
}

fn literal_type_at_position(text: &str, position: Position) -> Option<(String, Range)> {
    let offset = position_to_byte_offset(text, position)?;

    if let Some((start, end)) = find_enclosing_string_literal(text, offset) {
        return Some((
            "[Char]".to_string(),
            Range::new(
                byte_offset_to_position(text, start),
                byte_offset_to_position(text, end),
            ),
        ));
    }

    if let Some((start, end)) = find_enclosing_char_literal(text, offset) {
        return Some((
            "Char".to_string(),
            Range::new(
                byte_offset_to_position(text, start),
                byte_offset_to_position(text, end),
            ),
        ));
    }

    if let Some((token, start, end)) = numeric_token_at_offset(text, offset) {
        let typ = if is_float_token(&token) {
            Some("Float")
        } else if is_int_token(&token) {
            Some("Int")
        } else {
            None
        }?;
        return Some((
            typ.to_string(),
            Range::new(
                byte_offset_to_position(text, start),
                byte_offset_to_position(text, end),
            ),
        ));
    }

    None
}

fn format_literal_hover(text: &str, range: Range, literal_type: &str) -> String {
    if literal_type == "[Char]" {
        if let Some((preview, len, truncated)) = preview_string_literal(text, range, 100) {
            let suffix = if truncated { "..." } else { "" };
            return format!(
                "```que\n\"{}{}\" : [Char]\nlen: {}\n```",
                preview,
                suffix,
                len
            );
        }
    }

    let literal_text = text_for_range(text, range).unwrap_or_default();
    format!("```que\n{} : {}\n```", literal_text, literal_type)
}

fn preview_string_literal(
    text: &str,
    range: Range,
    max_chars: usize,
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

        if b == b'"' && (i == 0 || bytes[i - 1] != b'\\') {
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

        if b == b'"' && (i == 0 || bytes[i - 1] != b'\\') {
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
    let dot_count = slice.iter().filter(|&&b| b == b'.').count();
    if dot_count != 1 {
        return false;
    }
    if !slice.iter().all(|b| b.is_ascii_digit() || *b == b'.') {
        return false;
    }
    let dot_idx = slice.iter().position(|&b| b == b'.').unwrap_or(0);
    let left = &slice[..dot_idx];
    let right = &slice[dot_idx + 1..];
    (!left.is_empty() || !right.is_empty())
        && left.iter().all(|b| b.is_ascii_digit())
        && right.iter().all(|b| b.is_ascii_digit())
}

fn text_for_range(text: &str, range: Range) -> Option<String> {
    let start = position_to_byte_offset(text, range.start)?;
    let end = position_to_byte_offset(text, range.end)?;
    text.get(start..end).map(|s| s.to_string())
}
