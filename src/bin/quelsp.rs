use lsp_server::{ Connection, Message, Notification, Request, RequestId, Response };
use lsp_types::notification::{
    DidChangeTextDocument,
    DidCloseTextDocument,
    DidOpenTextDocument,
    PublishDiagnostics,
};
use lsp_types::notification::Notification as LspNotification;
use lsp_types::request::Request as LspRequest;
use lsp_types::request::{ Completion, HoverRequest };
use lsp_types::{
    CompletionItem,
    CompletionItemKind,
    CompletionOptions,
    CompletionParams,
    CompletionResponse,
    Diagnostic,
    DiagnosticSeverity,
    DidChangeTextDocumentParams,
    DidCloseTextDocumentParams,
    DidOpenTextDocumentParams,
    Hover,
    HoverContents,
    HoverParams,
    HoverProviderCapability,
    InitializeParams,
    MarkupContent,
    MarkupKind,
    Position,
    PublishDiagnosticsParams,
    Range,
    ServerCapabilities,
    TextDocumentSyncCapability,
    TextDocumentSyncKind,
    Uri,
};
use que::infer::{ EffectFlags, infer_with_builtins_typed_lsp, InferErrorScope, TypedExpression };
use que::lsp_native_core as native_core;
use que::parser::Expression;
use que::types::{ Type, TypeEnv };
use serde_json::Value;
use std::cell::RefCell;
use std::collections::{ HashMap, HashSet };
use std::panic::{ catch_unwind, AssertUnwindSafe };
use std::time::{ Duration, Instant };
use serde::{ Deserialize, Serialize };

const ANALYSIS_DELAY_MS: u64 = 500;

#[derive(Clone, Debug)]
struct DocAnalysis {
    text: String,
    diagnostics: Vec<Diagnostic>,
    symbol_types: HashMap<String, String>,
    let_binding_types: HashMap<String, String>,
    let_binding_effects: HashMap<String, EffectFlags>,
    user_bound_symbols: HashSet<String>,
    form_scoped_symbols: Vec<FormScopedAnalysis>,
}

#[derive(Clone, Debug)]
struct FormScopedAnalysis {
    range: Range,
    symbol_types: HashMap<String, String>,
    let_binding_types: HashMap<String, String>,
    let_binding_effects: HashMap<String, EffectFlags>,
}

struct LspCore {
    std_defs: Vec<Expression>,
    base_env: TypeEnv,
    base_next_id: u64,
    global_signatures: HashMap<String, String>,
    global_effects: HashMap<String, EffectFlags>,
    std_fallback_names: HashSet<String>,
}

struct PendingChange {
    version: i32,
    text: String,
    due_at: Instant,
}

struct ServerState {
    connection: Connection,
    documents: HashMap<Uri, DocAnalysis>,
    core: RefCell<Option<LspCore>>,
    pending_changes: HashMap<Uri, PendingChange>,
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
            trigger_characters: Some(
                vec![
                    "/".to_string(),
                    "!".to_string(),
                    "?".to_string(),
                    "-".to_string(),
                    "+".to_string(),
                    "*".to_string(),
                    ".".to_string()
                ]
            ),
            ..CompletionOptions::default()
        }),
        ..ServerCapabilities::default()
    };

    let init_value = serde_json
        ::to_value(server_capabilities)
        .map_err(|e| format!("serialize server capabilities: {}", e))?;
    let _init_params: InitializeParams = serde_json
        ::from_value(
            connection.initialize(init_value).map_err(|e| format!("initialize failed: {}", e))?
        )
        .map_err(|e| format!("parse initialize params: {}", e))?;

    let mut state = ServerState {
        connection,
        documents: HashMap::new(),
        core: RefCell::new(None),
        pending_changes: HashMap::new(),
    };

    loop {
        state.flush_due_changes()?;

        if state.has_pending_changes() {
            if let Ok(msg) = state.connection.receiver.try_recv() {
                if state.handle_message(msg)? {
                    break;
                }
                continue;
            }

            let sleep_for = state
                .time_until_next_pending()
                .unwrap_or_else(|| Duration::from_millis(1))
                .min(Duration::from_millis(10));
            std::thread::sleep(sleep_for);
            continue;
        }

        let Ok(msg) = state.connection.receiver.recv() else {
            break;
        };
        if state.handle_message(msg)? {
            break;
        }
    }

    io_threads.join().map_err(|e| format!("failed to join io threads: {}", e))?;
    Ok(())
}

fn build_lsp_core() -> LspCore {
    let std_defs = load_std_definitions();
    let (base_env, base_next_id, global_signatures, global_effects) = build_base_environment(
        &std_defs
    );
    let std_fallback_names = collect_std_top_level_let_names(&std_defs)
        .into_iter()
        .filter(|name| !name.starts_with("std/"))
        .collect();
    LspCore {
        std_defs,
        base_env,
        base_next_id,
        global_signatures,
        global_effects,
        std_fallback_names,
    }
}

impl ServerState {
    fn handle_message(&mut self, msg: Message) -> Result<bool, String> {
        match msg {
            Message::Request(req) => {
                if
                    self.connection
                        .handle_shutdown(&req)
                        .map_err(|e| format!("shutdown handling failed: {}", e))?
                {
                    return Ok(true);
                }
                self.handle_request(req)?;
                Ok(false)
            }
            Message::Notification(notif) => {
                self.handle_notification(notif)?;
                Ok(false)
            }
            Message::Response(_) => Ok(false),
        }
    }

    fn has_pending_changes(&self) -> bool {
        !self.pending_changes.is_empty()
    }

    fn time_until_next_pending(&self) -> Option<Duration> {
        let now = Instant::now();
        self.pending_changes
            .values()
            .map(|change| {
                if change.due_at > now {
                    change.due_at.duration_since(now)
                } else {
                    Duration::from_millis(0)
                }
            })
            .min()
    }

    fn flush_due_changes(&mut self) -> Result<(), String> {
        let now = Instant::now();
        let due_uris: Vec<Uri> = self.pending_changes
            .iter()
            .filter_map(|(uri, change)| if change.due_at <= now { Some(uri.clone()) } else { None })
            .collect();

        for uri in due_uris {
            let Some(change) = self.pending_changes.remove(&uri) else {
                continue;
            };
            let analysis = self.with_core(|core| {
                analyze_document_text_safe(
                    &change.text,
                    &core.std_defs,
                    &core.base_env,
                    core.base_next_id,
                    &core.global_signatures,
                    &core.global_effects,
                    &core.std_fallback_names
                )
            });
            self.publish_diagnostics(uri.clone(), Some(change.version), &analysis.diagnostics)?;
            self.documents.insert(uri, analysis);
        }
        Ok(())
    }

    fn with_core<R>(&self, f: impl FnOnce(&LspCore) -> R) -> R {
        if self.core.borrow().is_none() {
            *self.core.borrow_mut() = Some(build_lsp_core());
        }
        let core = self.core.borrow();
        f(core.as_ref().expect("lsp core must be initialized"))
    }

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
                let signature = self.signature_for_symbol(
                    &params.uri,
                    &params.symbol,
                    params.position
                );
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
                self.pending_changes.remove(&uri);
                let analysis = self.with_core(|core| {
                    analyze_document_text_safe(
                        &params.text_document.text,
                        &core.std_defs,
                        &core.base_env,
                        core.base_next_id,
                        &core.global_signatures,
                        &core.global_effects,
                        &core.std_fallback_names
                    )
                });
                self.publish_diagnostics(
                    uri.clone(),
                    Some(params.text_document.version),
                    &analysis.diagnostics
                )?;
                self.documents.insert(uri, analysis);
                Ok(())
            }
            DidChangeTextDocument::METHOD => {
                let params: DidChangeTextDocumentParams = parse_params(notif.params)?;
                let Some(last_change) = params.content_changes.last() else {
                    return Ok(());
                };

                let uri = params.text_document.uri;
                if let Some(doc) = self.documents.get_mut(&uri) {
                    doc.text = last_change.text.clone();
                } else {
                    self.documents.insert(uri.clone(), DocAnalysis {
                        text: last_change.text.clone(),
                        diagnostics: Vec::new(),
                        symbol_types: HashMap::new(),
                        let_binding_types: HashMap::new(),
                        let_binding_effects: HashMap::new(),
                        user_bound_symbols: HashSet::new(),
                        form_scoped_symbols: Vec::new(),
                    });
                }
                self.pending_changes.insert(uri, PendingChange {
                    version: params.text_document.version,
                    text: last_change.text.clone(),
                    due_at: Instant::now() + Duration::from_millis(ANALYSIS_DELAY_MS),
                });
                Ok(())
            }
            DidCloseTextDocument::METHOD => {
                let params: DidCloseTextDocumentParams = parse_params(notif.params)?;
                self.documents.remove(&params.text_document.uri);
                self.pending_changes.remove(&params.text_document.uri);
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
        diagnostics: &[Diagnostic]
    ) -> Result<(), String> {
        let params = PublishDiagnosticsParams {
            uri,
            version,
            diagnostics: diagnostics.to_vec(),
        };
        let notif = Notification::new(PublishDiagnostics::METHOD.to_string(), params);
        self.connection.sender
            .send(Message::Notification(notif))
            .map_err(|e| format!("send diagnostics failed: {}", e))
    }

    fn reply_ok<T: serde::Serialize>(
        &self,
        id: RequestId,
        result: Option<T>
    ) -> Result<(), String> {
        let value = serde_json
            ::to_value(result)
            .map_err(|e| format!("serialize response payload failed: {}", e))?;
        let response = Response::new_ok(id, value);
        self.connection.sender
            .send(Message::Response(response))
            .map_err(|e| format!("send response failed: {}", e))
    }

    fn hover_for_document(&self, params: &HoverParams) -> Option<Hover> {
        let uri = &params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;
        let doc = self.documents.get(uri)?;

        if let Some((literal_type, literal_range)) = literal_type_at_position(&doc.text, position) {
            let value = format!(
                "```que\n{}\n```",
                format_literal_hover(&doc.text, literal_range, &literal_type)
            );
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
                .and_then(|m| m.get(&symbol).cloned())
                .or_else(|| doc.let_binding_types.get(&symbol).cloned())
                .or_else(|| self.global_signature(&symbol))?;
            let declaration_type = normalize_signature(&declaration_type);
            let declaration_effect = self
                .form_let_effects_at(doc, position)
                .and_then(|m| m.get(&symbol).copied())
                .or_else(|| doc.let_binding_effects.get(&symbol).copied())
                .or_else(|| self.with_core(|core| core.global_effects.get(&symbol).copied()))
                .or_else(|| native_core::known_symbol_effect(&symbol));
            let effect_text = if declaration_type.contains("->") {
                declaration_effect.and_then(native_core::format_effect_flags)
            } else {
                None
            };
            let value = if let Some(effect) = effect_text {
                format!("```que\n{} : {}\n```\n`effects: {}`", symbol, declaration_type, effect)
            } else {
                format!("```que\n{} : {}\n```", symbol, declaration_type)
            };

            return Some(Hover {
                contents: HoverContents::Markup(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value,
                }),
                range: Some(symbol_range),
            });
        }

        let scoped_sig = self
            .form_signatures_at(doc, position)
            .and_then(|symbols| symbols.get(&symbol))
            .cloned();
        let doc_sig = doc.symbol_types.get(&symbol).cloned();
        let global_sig = self.global_signature(&symbol);
        let type_info = (if doc.user_bound_symbols.contains(&symbol) {
            scoped_sig.or(doc_sig).or(global_sig)
        } else if is_standalone_symbol_expr_at_range(&doc.text, symbol_range, &symbol) {
            global_sig.or(scoped_sig).or(doc_sig)
        } else {
            scoped_sig.or(doc_sig).or(global_sig)
        })?;
        let type_info = normalize_signature(&type_info);
        let symbol_effect = self
            .form_let_effects_at(doc, position)
            .and_then(|m| m.get(&symbol).copied())
            .or_else(|| doc.let_binding_effects.get(&symbol).copied())
            .or_else(|| self.with_core(|core| core.global_effects.get(&symbol).copied()))
            .or_else(|| native_core::known_symbol_effect(&symbol));
        let effect_text = if type_info.contains("->") {
            symbol_effect.and_then(native_core::format_effect_flags)
        } else {
            None
        };
        let value = if let Some(effect) = effect_text {
            format!("```que\n{} : {}\n```\n`effects: {}`", symbol, type_info, effect)
        } else {
            format!("```que\n{} : {}\n```", symbol, type_info)
        };

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
        let mut inferred_signatures: HashMap<String, String> = HashMap::new();
        let mut items = Vec::new();

        for keyword in [
            "lambda",
            "if",
            "let",
            "let*",
            "mut",
            "do",
            "as",
            "alter!",
            "while",
            "loop",
            "vector",
            "string",
            "tuple",
        ] {
            let insert_text = match keyword {
                "mut" | "alter!" => Some(format!("{} ", keyword)),
                _ => None,
            };
            items.push(CompletionItem {
                label: keyword.to_string(),
                kind: Some(CompletionItemKind::KEYWORD),
                insert_text,
                ..CompletionItem::default()
            });
        }

        if let Some(doc) = self.documents.get(uri) {
            for (name, signature) in &doc.symbol_types {
                if should_hide_completion_symbol(name) {
                    continue;
                }
                let should_override =
                    doc.user_bound_symbols.contains(name) || self.global_signature(name).is_none();
                if should_override {
                    inferred_signatures.insert(name.clone(), signature.clone());
                }
            }

            if let Some(form_signatures) = self.form_signatures_at(doc, position) {
                for (name, signature) in form_signatures {
                    if should_hide_completion_symbol(name) {
                        continue;
                    }
                    inferred_signatures.insert(name.clone(), signature.clone());
                }
            }
        }

        for (name, signature) in &inferred_signatures {
            items.push(CompletionItem {
                label: name.clone(),
                detail: Some(normalize_signature(signature)),
                kind: Some(kind_for_signature(signature)),
                ..CompletionItem::default()
            });
        }

        if inferred_signatures.is_empty() {
            self.with_core(|core| {
                for name in &core.std_fallback_names {
                    if should_hide_completion_symbol(name) {
                        continue;
                    }
                    let detail = core.global_signatures.get(name).cloned();
                    let kind = detail
                        .as_ref()
                        .map(|sig| kind_for_signature(sig))
                        .unwrap_or(CompletionItemKind::FUNCTION);
                    items.push(CompletionItem {
                        label: name.clone(),
                        detail: detail.as_ref().map(|sig| normalize_signature(sig)),
                        kind: Some(kind),
                        ..CompletionItem::default()
                    });
                }
            });
        }

        items.sort_by(|a, b| a.label.cmp(&b.label));
        items.dedup_by(|a, b| a.label == b.label);
        items
    }

    fn signature_for_symbol(
        &self,
        uri: &Uri,
        symbol: &str,
        position: Option<Position>
    ) -> Option<String> {
        if let Some(doc) = self.documents.get(uri) {
            return self
                .resolve_signature_for_doc(doc, symbol, position)
                .map(|s| normalize_signature(&s));
        }
        self.global_signature(symbol).map(|s| normalize_signature(&s))
    }

    fn resolve_signature_for_doc(
        &self,
        doc: &DocAnalysis,
        symbol: &str,
        position: Option<Position>
    ) -> Option<String> {
        let scoped_sig = position
            .and_then(|pos| self.form_signatures_at(doc, pos))
            .and_then(|symbols| symbols.get(symbol))
            .cloned();
        let doc_sig = doc.symbol_types.get(symbol).cloned();
        let global_sig = self.global_signature(symbol);

        if doc.user_bound_symbols.contains(symbol) {
            scoped_sig.or(doc_sig).or(global_sig)
        } else {
            global_sig.or(scoped_sig).or(doc_sig)
        }
    }

    fn global_signature(&self, symbol: &str) -> Option<String> {
        self.with_core(|core| core.global_signatures.get(symbol).cloned())
    }

    fn form_signatures_at<'a>(
        &'a self,
        doc: &'a DocAnalysis,
        position: Position
    ) -> Option<&'a HashMap<String, String>> {
        doc.form_scoped_symbols
            .iter()
            .find(|form| range_contains_position(&form.range, position))
            .map(|form| &form.symbol_types)
    }

    fn form_let_signatures_at<'a>(
        &'a self,
        doc: &'a DocAnalysis,
        position: Position
    ) -> Option<&'a HashMap<String, String>> {
        doc.form_scoped_symbols
            .iter()
            .find(|form| range_contains_position(&form.range, position))
            .map(|form| &form.let_binding_types)
    }

    fn form_let_effects_at<'a>(
        &'a self,
        doc: &'a DocAnalysis,
        position: Position
    ) -> Option<&'a HashMap<String, EffectFlags>> {
        doc.form_scoped_symbols
            .iter()
            .find(|form| range_contains_position(&form.range, position))
            .map(|form| &form.let_binding_effects)
    }
}

fn parse_params<T: serde::de::DeserializeOwned>(params: Value) -> Result<T, String> {
    serde_json::from_value(params).map_err(|e| format!("invalid params: {}", e))
}

fn to_core_position(position: Position) -> native_core::CorePosition {
    native_core::CorePosition {
        line: position.line,
        character: position.character,
    }
}

fn from_core_position(position: native_core::CorePosition) -> Position {
    Position::new(position.line, position.character)
}

fn to_core_range(range: Range) -> native_core::CoreRange {
    native_core::CoreRange {
        start: to_core_position(range.start),
        end: to_core_position(range.end),
    }
}

fn from_core_range(range: native_core::CoreRange) -> Range {
    Range::new(from_core_position(range.start), from_core_position(range.end))
}

fn normalize_signature(signature: &str) -> String {
    native_core::normalize_signature(signature)
}

fn strip_type_var_numbers(input: &str) -> String {
    native_core::strip_type_var_numbers(input)
}

fn build_base_environment(
    std_defs: &[Expression]
) -> (TypeEnv, u64, HashMap<String, String>, HashMap<String, EffectFlags>) {
    native_core::build_base_environment(std_defs)
}

fn load_std_definitions() -> Vec<Expression> {
    native_core::load_std_definitions()
}

fn collect_std_top_level_let_names(std_defs: &[Expression]) -> HashSet<String> {
    native_core::collect_std_top_level_let_names(std_defs)
}

fn collect_let_binding_types(node: &TypedExpression, signatures: &mut HashMap<String, Type>) {
    native_core::collect_let_binding_types(node, signatures)
}

fn collect_let_binding_effects(
    node: &TypedExpression,
    effects: &mut HashMap<String, EffectFlags>,
    fallback_effects: &HashMap<String, EffectFlags>
) {
    native_core::collect_let_binding_effects(node, effects, fallback_effects)
}

fn analyze_document_text(
    text: &str,
    std_defs: &[Expression],
    base_env: &TypeEnv,
    base_next_id: u64,
    global_signatures: &HashMap<String, String>,
    global_effects: &HashMap<String, EffectFlags>,
    std_fallback_names: &HashSet<String>
) -> DocAnalysis {
    if text.trim().is_empty() {
        return DocAnalysis {
            text: text.to_string(),
            diagnostics: Vec::new(),
            symbol_types: HashMap::new(),
            let_binding_types: HashMap::new(),
            let_binding_effects: HashMap::new(),
            user_bound_symbols: HashSet::new(),
            form_scoped_symbols: Vec::new(),
        };
    }

    let mut diagnostics = Vec::new();
    let mut symbol_types_raw: HashMap<String, Type> = HashMap::new();
    let mut let_binding_types_raw: HashMap<String, Type> = HashMap::new();
    let mut let_binding_effects: HashMap<String, EffectFlags> = HashMap::new();
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
                    diagnostics.extend(make_error_diagnostic(text, primary_err, None));
                    return DocAnalysis {
                        text: text.to_string(),
                        diagnostics,
                        symbol_types: HashMap::new(),
                        let_binding_types: HashMap::new(),
                        let_binding_effects: HashMap::new(),
                        user_bound_symbols,
                        form_scoped_symbols,
                    };
                }
            }
        }
    };

    match
        infer_with_builtins_typed_lsp(&program, (base_env.clone(), base_next_id), user_form_count)
    {
        Ok((_typ, typed)) => {
            for form in extract_user_top_level_typed_forms(&typed, user_form_count) {
                collect_symbol_types(form, &mut symbol_types_raw);
                collect_let_binding_types(form, &mut let_binding_types_raw);
                collect_let_binding_effects(form, &mut let_binding_effects, global_effects);
            }
            form_scoped_symbols = build_form_scoped_analyses(
                text,
                user_form_count,
                &typed,
                global_effects
            );
        }
        Err(err) => {
            let mut candidate_symbols = user_bound_symbols.clone();
            candidate_symbols.extend(global_signatures.keys().cloned());
            candidate_symbols.extend(std_fallback_names.iter().cloned());
            candidate_symbols.extend(symbol_types_raw.keys().cloned());
            candidate_symbols.extend(let_binding_types_raw.keys().cloned());
            let message_with_suggestions = native_core::append_undefined_variable_suggestions(
                &err.message,
                candidate_symbols.iter().map(|s| s.as_str()),
                3
            );
            diagnostics.extend(
                make_error_diagnostic(text, message_with_suggestions, err.scope.as_ref())
            );
        }
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
        let_binding_effects,
        user_bound_symbols,
        form_scoped_symbols,
    }
}

fn parse_user_exprs_for_symbol_collection(text: &str) -> Option<Vec<Expression>> {
    native_core::parse_user_exprs_for_symbol_collection(text)
}

fn strip_comment_bodies_preserve_newlines(text: &str) -> String {
    native_core::strip_comment_bodies_preserve_newlines(text)
}

fn analyze_document_text_safe(
    text: &str,
    std_defs: &[Expression],
    base_env: &TypeEnv,
    base_next_id: u64,
    global_signatures: &HashMap<String, String>,
    global_effects: &HashMap<String, EffectFlags>,
    std_fallback_names: &HashSet<String>
) -> DocAnalysis {
    match
        catch_unwind(
            AssertUnwindSafe(|| {
                analyze_document_text(
                    text,
                    std_defs,
                    base_env,
                    base_next_id,
                    global_signatures,
                    global_effects,
                    std_fallback_names
                )
            })
        )
    {
        Ok(analysis) => analysis,
        Err(payload) => {
            let message = format!(
                "Internal parser/inference error: {}",
                panic_payload_to_string(payload)
            );
            DocAnalysis {
                text: text.to_string(),
                diagnostics: make_error_diagnostic(text, message, None),
                symbol_types: HashMap::new(),
                let_binding_types: HashMap::new(),
                let_binding_effects: HashMap::new(),
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
    global_effects: &HashMap<String, EffectFlags>
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
        let mut let_binding_effects = HashMap::new();
        collect_symbol_types(typed_user_forms[idx], &mut raw_symbols);
        collect_let_binding_types(typed_user_forms[idx], &mut raw_let_bindings);
        collect_let_binding_effects(typed_user_forms[idx], &mut let_binding_effects, global_effects);
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
            let_binding_effects,
        });
    }

    out
}

fn extract_user_top_level_typed_forms<'a>(
    typed_program: &'a TypedExpression,
    user_form_count: usize
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
                ranges.push(
                    Range::new(
                        byte_offset_to_position(text, start),
                        byte_offset_to_position(text, end)
                    )
                );
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
                ranges.push(
                    Range::new(
                        byte_offset_to_position(text, start),
                        byte_offset_to_position(text, i)
                    )
                );
            }
            _ => {
                i += 1;
                while i < bytes.len() && !bytes[i].is_ascii_whitespace() && bytes[i] != b';' {
                    i += 1;
                }
                ranges.push(
                    Range::new(
                        byte_offset_to_position(text, start),
                        byte_offset_to_position(text, i)
                    )
                );
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
    native_core::repair_source_for_analysis(text)
}

fn collect_user_bound_symbols_from_exprs(exprs: &[Expression], out: &mut HashSet<String>) {
    native_core::collect_user_bound_symbols_from_exprs(exprs, out)
}

fn collect_symbol_types(node: &TypedExpression, symbols: &mut HashMap<String, Type>) {
    native_core::collect_symbol_types(node, symbols)
}

fn make_error_diagnostic(
    text: &str,
    message: String,
    scope: Option<&InferErrorScope>
) -> Vec<Diagnostic> {
    let normalized_message = strip_type_var_numbers(&message);
    let inferred_ranges = infer_error_ranges(text, &message, scope);
    let display_message = if !inferred_ranges.is_empty() {
        diagnostic_summary_without_snippet(&normalized_message)
    } else {
        normalized_message
    };

    let ranges = if inferred_ranges.is_empty() { vec![full_range(text)] } else { inferred_ranges };

    ranges
        .into_iter()
        .map(|range| Diagnostic {
            range,
            severity: Some(DiagnosticSeverity::ERROR),
            message: display_message.clone(),
            source: Some("que".to_string()),
            ..Diagnostic::default()
        })
        .collect()
}

fn diagnostic_summary_without_snippet(message: &str) -> String {
    native_core::diagnostic_summary_without_snippet(message)
}

fn infer_error_ranges(text: &str, message: &str, scope: Option<&InferErrorScope>) -> Vec<Range> {
    native_core::infer_error_ranges(text, message, scope).into_iter().map(from_core_range).collect()
}

fn find_matching_paren_byte(text: &str, open_idx: usize) -> Option<usize> {
    native_core::find_matching_paren_byte(text, open_idx)
}

fn byte_offset_to_position(text: &str, target: usize) -> Position {
    from_core_position(native_core::byte_offset_to_position(text, target))
}

fn full_range(text: &str) -> Range {
    from_core_range(native_core::full_range(text))
}

fn position_to_byte_offset(text: &str, position: Position) -> Option<usize> {
    native_core::position_to_byte_offset(text, to_core_position(position))
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

fn should_hide_completion_symbol(symbol: &str) -> bool {
    symbol.starts_with('_')
}

fn symbol_at_position(text: &str, position: Position) -> Option<(String, Range)> {
    native_core
        ::symbol_at_position(text, to_core_position(position))
        .map(|(symbol, range)| (symbol, from_core_range(range)))
}

fn is_standalone_symbol_expr_at_range(text: &str, range: Range, symbol: &str) -> bool {
    native_core::is_standalone_symbol_expr_at_range(text, to_core_range(range), symbol)
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

    let Some((head, _head_start, _head_end, after_head)) = read_top_level_atom_token_in_list(
        text,
        open + 1,
        close
    ) else {
        return false;
    };
    if head != "let" && head != "let*" && head != "mut" {
        return false;
    }

    let Some((name, name_start, name_end, _)) = read_top_level_atom_token_in_list(
        text,
        after_head,
        close
    ) else {
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
    list_close: usize
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
    native_core
        ::literal_type_at_position(text, to_core_position(position))
        .map(|(literal_type, range)| (literal_type, from_core_range(range)))
}

fn format_literal_hover(text: &str, range: Range, literal_type: &str) -> String {
    native_core::format_literal_hover(text, to_core_range(range), literal_type)
}
