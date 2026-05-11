use crate::infer::{ infer_with_builtins_typed, InferErrorInfo, InferErrorScope, TypedExpression };
use crate::lsp_native_core::{
    diagnostic_summary_without_snippet,
    extract_error_snippet,
    infer_error_ranges,
    normalize_signature,
};
use crate::parser::Expression;
use reqwest::blocking::{ Client, Response };
use reqwest::header::{ CONTENT_TYPE, HeaderMap, HeaderValue };
use reqwest::Method;
use serde_json::{ json, Value };
use std::collections::{ BTreeMap, HashSet };
use std::env;
use std::fs;
use std::io;
use std::io::Read as _;
use std::io::Write as _;
use std::path::{ Path, PathBuf };
use std::thread;
use std::time::Duration;
use wasmtime::Linker;
use wasmtime::{ Caller, Extern, Memory, TypedFunc };
use wasmtime_wasi::{ ResourceTable, WasiCtx, WasiCtxBuilder, WasiCtxView, WasiView };

const VEC_LEN_OFFSET: i32 = 0;
const VEC_CAP_OFFSET: i32 = 4;
const VEC_RC_OFFSET: i32 = 8;
const VEC_ELEM_REF_OFFSET: i32 = 12;
const VEC_DATA_PTR_OFFSET: i32 = 16;
const VEC_MAGIC_OFFSET: i32 = 20;
const VEC_HEADER_SIZE: i32 = 24;
const VEC_MAGIC: i32 = 1_447_380_017;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ShellPermission {
    Read,
    Write,
    Delete,
}

impl ShellPermission {
    fn as_str(&self) -> &'static str {
        match self {
            ShellPermission::Read => "read",
            ShellPermission::Write => "write",
            ShellPermission::Delete => "delete",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShellPolicy {
    shell_enabled: bool,
    permissions: HashSet<ShellPermission>,
}

impl ShellPolicy {
    pub fn disabled() -> Self {
        Self {
            shell_enabled: false,
            permissions: HashSet::new(),
        }
    }

    fn enabled(permissions: HashSet<ShellPermission>) -> Self {
        Self {
            shell_enabled: true,
            permissions,
        }
    }

    fn allows(&self, permission: ShellPermission) -> bool {
        self.permissions.contains(&permission)
    }

    pub fn require(
        &self,
        permission: ShellPermission,
        operation: &str,
        target: &str
    ) -> Result<(), String> {
        if !self.shell_enabled {
            return Err(
                format!(
                    "host io is disabled. pass --allow <read|write|delete> [...]. denied operation '{}' for '{}'",
                    operation,
                    target
                )
            );
        }

        if !self.allows(permission) {
            return Err(
                format!(
                    "permission '{}' is required for operation '{}'. denied target: {}",
                    permission.as_str(),
                    operation,
                    target
                )
            );
        }

        Ok(())
    }
}

fn parse_shell_policy_permissions(parts: &[String]) -> Result<ShellPolicy, String> {
    let mut permissions = HashSet::new();
    let mut grant_all = false;

    for part in parts {
        for fragment in part.split(',') {
            let token = fragment.trim().trim_matches('"').trim_matches('\'').to_ascii_lowercase();
            if token.is_empty() {
                continue;
            }
            match token.as_str() {
                "read" => {
                    permissions.insert(ShellPermission::Read);
                }
                "write" => {
                    permissions.insert(ShellPermission::Write);
                }
                "delete" => {
                    permissions.insert(ShellPermission::Delete);
                }
                "all" | "*" => {
                    grant_all = true;
                }
                _ => {
                    return Err(
                        format!("unknown shell permission '{}'. expected one of: read, write, delete", token)
                    );
                }
            }
        }
    }

    if grant_all {
        permissions.insert(ShellPermission::Read);
        permissions.insert(ShellPermission::Write);
        permissions.insert(ShellPermission::Delete);
    }

    Ok(ShellPolicy::enabled(permissions))
}

pub fn take_shell_policy_from_argv(argv: &mut Vec<String>) -> Result<ShellPolicy, String> {
    if let Some(pos) = argv.iter().position(|arg| arg == "--allow") {
        let permissions = argv[pos + 1..].to_vec();
        argv.truncate(pos);
        return parse_shell_policy_permissions(&permissions);
    }

    Ok(ShellPolicy::disabled())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DebugMode {
    Off,
    Basic,
    Code,
    Types,
    All,
}

fn merge_debug_mode(current: DebugMode, next: DebugMode) -> DebugMode {
    let enabled = current != DebugMode::Off || next != DebugMode::Off;
    let code =
        matches!(current, DebugMode::Code | DebugMode::All) ||
        matches!(next, DebugMode::Code | DebugMode::All);
    let types =
        matches!(current, DebugMode::Types | DebugMode::All) ||
        matches!(next, DebugMode::Types | DebugMode::All);

    if code && types {
        DebugMode::All
    } else if code {
        DebugMode::Code
    } else if types {
        DebugMode::Types
    } else if enabled {
        DebugMode::Basic
    } else {
        DebugMode::Off
    }
}

fn parse_debug_mode_token(token: &str) -> Option<DebugMode> {
    match token.trim().to_ascii_lowercase().as_str() {
        "all" => Some(DebugMode::All),
        "basic" | "loc" | "location" => Some(DebugMode::Basic),
        "code" => Some(DebugMode::Code),
        "types" => Some(DebugMode::Types),
        _ => None,
    }
}

impl DebugMode {
    fn is_enabled(self) -> bool {
        self != DebugMode::Off
    }

    fn includes_code(self) -> bool {
        matches!(self, DebugMode::Code | DebugMode::All)
    }

    fn includes_types(self) -> bool {
        matches!(self, DebugMode::Types | DebugMode::All)
    }
}

pub fn take_debug_mode_from_argv(argv: &mut Vec<String>) -> DebugMode {
    let mut mode = DebugMode::Off;
    let mut i = 0usize;
    let mut out = Vec::with_capacity(argv.len());

    while i < argv.len() {
        let token = &argv[i];
        if token == "--debug" {
            let mut next_mode = DebugMode::Basic;
            if let Some(next_token) = argv.get(i + 1) {
                if let Some(parsed) = parse_debug_mode_token(next_token) {
                    next_mode = parsed;
                    i += 1;
                }
            }
            mode = merge_debug_mode(mode, next_mode);
            i += 1;
            continue;
        }

        out.push(token.clone());
        i += 1;
    }

    *argv = out;
    mode
}

fn take_help_flag_from_argv(argv: &mut Vec<String>) -> bool {
    let mut found = false;
    let mut out = Vec::with_capacity(argv.len());
    for token in argv.iter() {
        if token == "--help" || token == "-h" {
            found = true;
            continue;
        }
        out.push(token.clone());
    }
    *argv = out;
    found
}

fn take_no_result_flag_from_argv(argv: &mut Vec<String>) -> bool {
    let mut found = false;
    let mut out = Vec::with_capacity(argv.len());
    for token in argv.iter() {
        if token == "--no-result" {
            found = true;
            continue;
        }
        out.push(token.clone());
    }
    *argv = out;
    found
}

fn enable_debug_runtime_guards() {
    env::set_var("QUE_INT_OVERFLOW_CHECK", "1");
    env::set_var("QUE_FLOAT_OVERFLOW_CHECK", "1");
    env::set_var("QUE_DIV_ZERO_CHECK", "1");
    env::set_var("QUE_BOUNDS_CHECK", "1");
}

fn load_project_library_definitions(start_dir: &Path) -> Result<Vec<Expression>, String> {
    let Some(project) = crate::project::discover_project_config(start_dir)? else {
        return Ok(Vec::new());
    };
    crate::project::load_bundle_definitions(&project.root_dir, &project.config.deps)
}

fn apply_project_env_vars(start_dir: &Path) -> Result<(), String> {
    let Some(project) = crate::project::discover_project_config(start_dir)? else {
        return Ok(());
    };
    for (key, value) in project.config.env.iter() {
        env::set_var(key, value);
    }
    Ok(())
}

fn resolve_project_entry_path(
    cwd: &Path,
    explicit_path: Option<&str>,
) -> Result<(PathBuf, String, PathBuf), String> {
    if let Some(file_path) = explicit_path {
        let program = fs
            ::read_to_string(file_path)
            .map_err(|e| format!("failed to read '{}': {}", file_path, e))?;
        let script_path = fs::canonicalize(file_path).unwrap_or_else(|_| PathBuf::from(file_path));
        let script_cwd = script_path
            .parent()
            .map(Path::to_path_buf)
            .filter(|path| !path.as_os_str().is_empty())
            .unwrap_or_else(|| PathBuf::from("."));
        return Ok((script_path, program, script_cwd));
    }

    let Some(project) = crate::project::discover_project_config(cwd)? else {
        return Err("missing file_path".to_string());
    };
    let Some(entry) = project.config.entry.as_deref() else {
        return Err(
            format!(
                "missing file_path and '{}' has no `entry`",
                project.path.display()
            )
        );
    };
    let entry_path = project.root_dir.join(entry);
    let program = fs
        ::read_to_string(&entry_path)
        .map_err(|e| format!("failed to read '{}': {}", entry_path.display(), e))?;
    let script_path = fs::canonicalize(&entry_path).unwrap_or(entry_path);
    Ok((script_path, program, project.root_dir))
}

fn init_project_config_file(dir: &Path) -> Result<PathBuf, String> {
    let path = dir.join(crate::project::PROJECT_CONFIG_FILE);
    if path.exists() {
        return Err(format!("project config already exists at '{}'", path.display()));
    }
    fs::write(&path, crate::project::default_project_config_text())
        .map_err(|e| format!("failed to write '{}': {}", path.display(), e))?;
    Ok(path)
}

fn take_install_output_path_from_argv(argv: &mut Vec<String>) -> Result<Option<String>, String> {
    let mut out_path: Option<String> = None;
    let mut out = Vec::with_capacity(argv.len());
    let mut i = 0usize;
    while i < argv.len() {
        if argv[i] == "--out" {
            i += 1;
            if i >= argv.len() || argv[i].starts_with("--") {
                return Err("--out requires a path".to_string());
            }
            out_path = Some(argv[i].clone());
            i += 1;
            continue;
        }
        out.push(argv[i].clone());
        i += 1;
    }
    *argv = out;
    Ok(out_path)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EmitKind {
    Source,
    Wat,
    Wasm,
    Types,
}

#[derive(Debug)]
struct EmitRequest {
    kind: EmitKind,
    out_path: Option<String>,
}

fn take_emit_request_from_argv(argv: &mut Vec<String>) -> Result<Option<EmitRequest>, String> {
    let mut kind = None;
    if let Some(pos) = argv.iter().position(|arg| arg == "--emit") {
        if pos + 1 >= argv.len() || argv[pos + 1].starts_with("--") {
            return Err("--emit requires one of: source | wat | wasm | types".to_string());
        }
        let value = argv[pos + 1].clone();
        kind = Some(match value.as_str() {
            "source" => EmitKind::Source,
            "wat" => EmitKind::Wat,
            "wasm" => EmitKind::Wasm,
            "types" => EmitKind::Types,
            _ => {
                return Err(format!("unknown --emit kind '{}'", value));
            }
        });
        argv.drain(pos..=pos + 1);
    } else if let Some(pos) = argv.iter().position(|arg| arg == "--emit-source") {
        argv.remove(pos);
        kind = Some(EmitKind::Source);
    }

    let Some(kind) = kind else {
        return Ok(None);
    };

    let mut out_path: Option<String> = None;
    let mut out = Vec::with_capacity(argv.len());
    let mut i = 0usize;
    while i < argv.len() {
        if argv[i] == "--out" {
            i += 1;
            if i >= argv.len() || argv[i].starts_with("--") {
                return Err("--out requires a path".to_string());
            }
            out_path = Some(argv[i].clone());
            i += 1;
            continue;
        }
        out.push(argv[i].clone());
        i += 1;
    }
    *argv = out;
    Ok(Some(EmitRequest { kind, out_path }))
}

fn native_shell_help(bin_name: &str) -> String {
    format!(
        "Usage: {bin} <script.que> [arg ...] [--debug [basic|code|types|all]] [--allow <read|write|delete|all> [...]]\n\
         or:    {bin} --eval <source> [arg ...] [--debug [basic|code|types|all]] [--allow <read|write|delete|all> [...]]\n\
         or:    {bin} [<script.que>] [arg ...] --emit <source|wat|wasm|types> [--out <file>]\n\
         or:    {bin} --eval <source> [arg ...] --emit <source|wat|wasm|types> [--out <file>]\n\
         or:    {bin} [<script.que>] [arg ...] --emit-source [--out <expanded.lisp>]\n\
         or:    {bin} --eval <source> [arg ...] --emit-source [--out <expanded.lisp>]\n\
         or:    {bin} init\n\
         or:    {bin} --install [helpers.que ...] [--out <que-lib.lisp>]\n\
         or:    {bin} lambda <command> [...]\n\
         or:    {bin} --lib <names|types|source> [pattern|name]\n\
         or:    {bin} --learn\n\
         or:    {bin} --env\n\
         or:    {bin} --uninstall [--out <que-lib.lisp>]\n\
         \n\
         Flags:\n\
           --help, -h     Show this help and exit.\n\
          --learn        Print Que language quick reference.\n\
          --env          Print environment flags and tuning examples.\n\
           --eval, -e     Execute inline Que source without a script file.\n\
           --emit         Output source, wat, wasm, or top-level types and exit.\n\
           --emit-source  Print merged/tree-shaken/desugared Lisp source and exit.\n\
                         Use with --out <file> to write it instead of printing.\n\
          init           Write a default `{config}` in the current directory.\n\
           --debug        Enable compiler/runtime debug report on errors (default: basic locations).\n\
                         Also forces QUE_INT_OVERFLOW_CHECK, QUE_FLOAT_OVERFLOW_CHECK,\n\
                         QUE_DIV_ZERO_CHECK, and QUE_BOUNDS_CHECK to ON for this run.\n\
           --no-result    Do not print/decode the final evaluated program value.\n\
           --allow        Enable host io permissions (read, write, delete, all).\n\
         \n\
         Notes:\n\
           - Recommended: run with `--debug` for stronger safety checks and richer diagnostics.\n\
          - If a nearby `{config}` exists, native CLI and native LSP load its `deps`.\n\
          - If no `{config}` exists, no project config is used.\n\
          - Omitting `<script.que>` uses `entry` from `{config}` when present.\n\
          - Script arguments come before flags like --allow.\n\
          - `--install` accepts helper .que files as positional arguments.\n\
           - `lambda` manages https://lambda.quest auth, uploads, deletes, and execution.\n\
           - `--lib names [pattern]` lists available library names.\n\
           - `--lib types [pattern]` prints name and inferred type.\n\
           - `--lib source <name>` prints the exact symbol source.\n\
            - Inline eval example: `{bin} --eval '(+ 1 2)'`.\n\
            - Wildcards in pattern: `*` any sequence, `?` single char.\n\
           - `--emit source` prints merged/tree-shaken/desugared Lisp.\n\
           - `--emit wat` prints WAT text.\n\
           - `--emit wasm` prints raw wasm bytes unless you pass `--out`.\n\
           - `--emit types` prints inferred top-level user-form types and final result type.\n\
           - --debug, --no-result, --emit, --emit-source and --help can appear after the script path.\n\
           - `--install` writes/extends an external library file (used by all binaries).\n\
           - `--uninstall` removes the active external library file.\n\
           - Default output path: /usr/local/share/que/que-lib.lisp.\n\
           - In installed setups, without an external library file only language builtins are available.\n\
           - After install/uninstall, restart editor/LSP to reload library state.\n\
           - Once installed, helper bundle source files can be removed.\n\
         ",
        bin = bin_name,
        config = crate::project::PROJECT_CONFIG_FILE
    )
}

const LAMBDA_API_BASE_DEFAULT: &str = "https://lambda.quest";
const LAMBDA_API_KEY_ENV: &str = "LAMBDA_API_KEY";
const LAMBDA_API_BASE_ENV: &str = "QUE_LAMBDA_API_BASE";

#[derive(Clone, Debug, PartialEq, Eq)]
struct LambdaExecuteOptions {
    input: String,
    version: Option<String>,
    params: Vec<(String, String)>,
    method: Method,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct LambdaExampleOptions {
    input: String,
    version: Option<String>,
    params: Vec<(String, String)>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum LambdaInvokeTarget {
    OwnedName(String),
    PublicByName {
        owner_label: String,
        function_name: String,
    },
}

fn native_lambda_help(bin_name: &str) -> String {
    format!(
        "Usage: {bin} lambda auth <api-key>\n\
         or:    {bin} lambda auth --show\n\
         or:    {bin} lambda auth --clear\n\
         or:    {bin} lambda create-key [label]\n\
         or:    {bin} lambda list\n\
         or:    {bin} lambda invoke <function-name|owner_label/function-name> [--input <text>] [--version <n>] [--param <key=value> ...] [--get|--post]\n\
         or:    {bin} lambda example <function-id|public-id> [--input <text>] [--version <n>] [--param <key=value> ...]\n\
         or:    {bin} lambda create <function-name> <file|-> [--sources <sources.json>]\n\
         or:    {bin} lambda upload <function-name> <file|-> [--sources <sources.json>]\n\
         or:    {bin} lambda get <function-id>\n\
         or:    {bin} lambda get-source <function-id> [version]\n\
         or:    {bin} lambda delete <function-id>\n\
         or:    {bin} lambda execute <function-id> [--input <text>] [--version <n>] [--param <key=value> ...] [--get|--post]\n\
         or:    {bin} lambda public-execute <public-id> [--input <text>] [--version <n>] [--param <key=value> ...] [--get|--post]\n\
         \n\
        Notes:\n\
           - API base defaults to {base} and can be overridden with `{base_env}`.\n\
           - `auth` stores the owner key locally for later private commands.\n\
           - `create`/`upload` accept `-` to read Que source from stdin.\n\
           - `create`/`upload` parse and typecheck locally before upload and require a final `[Char]` result.\n\
           - `invoke name` resolves `name` against your authenticated functions and executes it privately.\n\
           - `invoke owner/name` uses the public by-name route.\n\
           - `example` builds a public GET execute URL. If you pass `fn_...`, it resolves the matching `pub_...` first.\n\
           - `--sources` expects a JSON array file, for example:\n\
             [{{\"name\":\"dummyjson\",\"urlTemplate\":\"https://dummyjson.com/todos/user/{{user_id}}\",\"enabled\":true}}]\n\
           - `public-execute` does not need auth.\n\
           - Environment fallback: `{key_env}` is still honored if set.\n\
        ",
        bin = bin_name,
        base = LAMBDA_API_BASE_DEFAULT,
        base_env = LAMBDA_API_BASE_ENV,
        key_env = LAMBDA_API_KEY_ENV
    )
}

#[cfg(target_os = "windows")]
fn lambda_auth_file_path() -> PathBuf {
    if let Ok(local_app_data) = env::var("LOCALAPPDATA") {
        return PathBuf::from(local_app_data)
            .join("Programs")
            .join("Que")
            .join("config")
            .join("lambda-auth.json");
    }
    PathBuf::from("lambda-auth.json")
}

#[cfg(not(target_os = "windows"))]
fn lambda_auth_file_path() -> PathBuf {
    if let Ok(home) = env::var("HOME") {
        return PathBuf::from(home).join(".config").join("que").join("lambda-auth.json");
    }
    PathBuf::from("lambda-auth.json")
}

fn read_saved_lambda_api_key() -> Result<Option<String>, String> {
    let path = lambda_auth_file_path();
    if !path.exists() {
        return Ok(None);
    }
    let raw = fs
        ::read_to_string(&path)
        .map_err(|e| format!("failed to read lambda auth file '{}': {}", path.display(), e))?;
    let value: Value = serde_json
        ::from_str(&raw)
        .map_err(|e| format!("failed to parse lambda auth file '{}': {}", path.display(), e))?;
    Ok(
        value
            .get("api_key")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .map(ToOwned::to_owned)
    )
}

fn load_lambda_api_key() -> Result<Option<String>, String> {
    if let Ok(value) = env::var(LAMBDA_API_KEY_ENV) {
        let trimmed = value.trim();
        if !trimmed.is_empty() {
            return Ok(Some(trimmed.to_string()));
        }
    }
    read_saved_lambda_api_key()
}

fn require_lambda_api_key() -> Result<String, String> {
    load_lambda_api_key()?.ok_or_else(|| {
        format!("lambda api key is not configured. run `que lambda auth <api-key>` or set {}", LAMBDA_API_KEY_ENV)
    })
}

fn save_lambda_api_key(api_key: &str) -> Result<PathBuf, String> {
    let path = lambda_auth_file_path();
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs
                ::create_dir_all(parent)
                .map_err(|e| {
                    format!("failed to create lambda auth directory '{}': {}", parent.display(), e)
                })?;
        }
    }
    let payload = json!({ "api_key": api_key.trim() });
    fs
        ::write(
            &path,
            serde_json
                ::to_vec_pretty(&payload)
                .map_err(|e| format!("failed to serialize lambda auth config: {}", e))?
        )
        .map_err(|e| format!("failed to write lambda auth file '{}': {}", path.display(), e))?;
    Ok(path)
}

fn clear_lambda_api_key() -> Result<PathBuf, String> {
    let path = lambda_auth_file_path();
    if path.exists() {
        fs
            ::remove_file(&path)
            .map_err(|e| format!("failed to remove lambda auth file '{}': {}", path.display(), e))?;
    }
    Ok(path)
}

fn lambda_client() -> Result<Client, String> {
    Client::builder()
        .build()
        .map_err(|e| format!("failed to build lambda api client: {}", e))
}

fn lambda_api_base_from_env_value(value: Option<String>) -> String {
    value
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| LAMBDA_API_BASE_DEFAULT.to_string())
}

fn lambda_api_base() -> String {
    lambda_api_base_from_env_value(env::var(LAMBDA_API_BASE_ENV).ok())
}

fn lambda_url(path: &str, query: &[(String, String)]) -> Result<reqwest::Url, String> {
    let base = lambda_api_base();
    let mut url = reqwest::Url
        ::parse(&format!("{}{}", base, path))
        .map_err(|e| format!("invalid lambda api url '{}{}': {}", base, path, e))?;
    if !query.is_empty() {
        let mut pairs = url.query_pairs_mut();
        for (key, value) in query {
            pairs.append_pair(key, value);
        }
    }
    Ok(url)
}

fn lambda_send(request: reqwest::blocking::RequestBuilder) -> Result<String, String> {
    let response = request.send().map_err(|e| format!("lambda api request failed: {}", e))?;
    lambda_response_text(response)
}

fn lambda_response_text(response: Response) -> Result<String, String> {
    let status = response.status();
    let body = response
        .text()
        .map_err(|e| format!("failed to read lambda api response body: {}", e))?;
    if !status.is_success() {
        let trimmed = body.trim();
        if trimmed.is_empty() {
            return Err(format!("lambda api request failed with status {}", status));
        }
        return Err(format!("lambda api request failed with status {}: {}", status, trimmed));
    }
    Ok(body)
}

fn lambda_auth_headers(api_key: &str) -> Result<HeaderMap, String> {
    let mut headers = HeaderMap::new();
    headers.insert(
        "x-api-key",
        HeaderValue::from_str(api_key).map_err(|e|
            format!("invalid lambda api key for header use: {}", e)
        )?
    );
    Ok(headers)
}

fn parse_lambda_execute_options(args: &[String]) -> Result<LambdaExecuteOptions, String> {
    let mut input = String::new();
    let mut version = None;
    let mut params = Vec::new();
    let mut method = Method::POST;
    let mut i = 0usize;
    while i < args.len() {
        match args[i].as_str() {
            "--input" => {
                i += 1;
                let value = args.get(i).ok_or_else(|| "missing value for --input".to_string())?;
                input = value.clone();
            }
            "--version" => {
                i += 1;
                let value = args.get(i).ok_or_else(|| "missing value for --version".to_string())?;
                version = Some(value.clone());
            }
            "--param" => {
                i += 1;
                let raw = args.get(i).ok_or_else(|| "missing value for --param".to_string())?;
                let (key, value) = raw
                    .split_once('=')
                    .ok_or_else(|| {
                        format!("invalid query item '{}'; expected key=value", raw)
                    })?;
                params.push((key.to_string(), value.to_string()));
            }
            "--get" => {
                method = Method::GET;
            }
            "--post" => {
                method = Method::POST;
            }
            other => {
                return Err(format!("unknown execute option: {}", other));
            }
        }
        i += 1;
    }
    Ok(LambdaExecuteOptions {
        input,
        version,
        params,
        method,
    })
}

fn parse_lambda_example_options(args: &[String]) -> Result<LambdaExampleOptions, String> {
    let mut input = "hello".to_string();
    let mut version = None;
    let mut params = Vec::new();
    let mut i = 0usize;
    while i < args.len() {
        match args[i].as_str() {
            "--input" => {
                i += 1;
                let value = args.get(i).ok_or_else(|| "missing value for --input".to_string())?;
                input = value.clone();
            }
            "--version" => {
                i += 1;
                let value = args.get(i).ok_or_else(|| "missing value for --version".to_string())?;
                version = Some(value.clone());
            }
            "--param" => {
                i += 1;
                let raw = args.get(i).ok_or_else(|| "missing value for --param".to_string())?;
                let (key, value) = raw
                    .split_once('=')
                    .ok_or_else(|| {
                        format!("invalid query item '{}'; expected key=value", raw)
                    })?;
                params.push((key.to_string(), value.to_string()));
            }
            other => {
                return Err(format!("unknown example option: {}", other));
            }
        }
        i += 1;
    }
    Ok(LambdaExampleOptions {
        input,
        version,
        params,
    })
}

fn lambda_example_url(
    public_id: &str,
    input: &str,
    version: Option<&str>,
    params: &[(String, String)]
) -> Result<String, String> {
    let mut query = params.to_vec();
    query.push(("input".to_string(), input.to_string()));
    if let Some(version) = version {
        query.push(("version".to_string(), version.to_string()));
    }
    Ok(lambda_url(&format!("/v1/public/{}/execute", public_id), &query)?.to_string())
}

fn parse_lambda_invoke_target(target: &str) -> Result<LambdaInvokeTarget, String> {
    if target.starts_with("fn_") || target.starts_with("pub_") {
        return Err(
            "lambda invoke is name-based. use `que lambda execute <fn_...>` or `que lambda public-execute <pub_...>` for id-based execution".to_string()
        );
    }
    if let Some((owner_label, function_name)) = target.split_once('/') {
        if owner_label.is_empty() || function_name.is_empty() {
            return Err(
                "lambda invoke owner/name expects both owner label and function name".to_string()
            );
        }
        return Ok(LambdaInvokeTarget::PublicByName {
            owner_label: owner_label.to_string(),
            function_name: function_name.to_string(),
        });
    }
    if target.trim().is_empty() {
        return Err("lambda invoke expects a non-empty function name".to_string());
    }
    Ok(LambdaInvokeTarget::OwnedName(target.to_string()))
}

fn lambda_value_str<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str)
}

fn lambda_value_i64(value: &Value, key: &str) -> Option<i64> {
    value.get(key).and_then(Value::as_i64)
}

fn resolve_public_id_from_function_json(value: &Value) -> Result<String, String> {
    lambda_value_str(value, "public_id")
        .filter(|v| !v.is_empty())
        .map(ToOwned::to_owned)
        .ok_or_else(|| "function is not public or missing public_id".to_string())
}

fn resolve_owned_function_id_from_functions_list(
    text: &str,
    function_name: &str
) -> Result<String, String> {
    let parsed: Value = serde_json
        ::from_str(text)
        .map_err(|e| format!("failed to parse lambda list response: {}", e))?;
    let functions = parsed
        .get("functions")
        .and_then(Value::as_array)
        .ok_or_else(|| "lambda list response is missing 'functions'".to_string())?;

    let matches = functions
        .iter()
        .filter(|function| lambda_value_str(function, "function_name") == Some(function_name))
        .collect::<Vec<_>>();

    match matches.len() {
        0 =>
            Err(
                format!("no function named '{}' was found in your current user functions", function_name)
            ),
        1 =>
            lambda_value_str(matches[0], "function_id")
                .filter(|v| !v.is_empty())
                .map(ToOwned::to_owned)
                .ok_or_else(|| format!("function '{}' is missing function_id", function_name)),
        _ =>
            Err(
                format!("multiple functions named '{}' were found in your current user functions; use `que lambda execute <fn_...>` instead", function_name)
            ),
    }
}

fn format_lambda_functions_list(text: &str) -> Option<String> {
    let parsed: Value = serde_json::from_str(text).ok()?;
    let functions = parsed.get("functions")?.as_array()?;
    if functions.is_empty() {
        return Some("No functions.".to_string());
    }

    let mut lines = Vec::new();
    for (idx, function) in functions.iter().enumerate() {
        if idx > 0 {
            lines.push(String::new());
        }
        let name = lambda_value_str(function, "function_name").unwrap_or("<unnamed>");
        let function_id = lambda_value_str(function, "function_id").unwrap_or("<missing>");
        let visibility = lambda_value_str(function, "visibility").unwrap_or("unknown");
        let public_id = lambda_value_str(function, "public_id").unwrap_or("-");
        let version = lambda_value_i64(function, "latest_version")
            .map(|v| v.to_string())
            .unwrap_or_else(|| "-".to_string());
        let status = lambda_value_str(function, "latest_compile_status").unwrap_or("unknown");
        let updated = lambda_value_str(function, "updated_at").unwrap_or("-");

        lines.push(format!("{}", name));
        lines.push(format!("  fn: {}", function_id));
        lines.push(format!("  public: {} ({})", public_id, visibility));
        lines.push(format!("  latest: v{} {}", version, status));
        lines.push(format!("  updated: {}", updated));
        if
            let Some(err) = lambda_value_str(function, "latest_compile_error").filter(
                |v| !v.is_empty()
            )
        {
            lines.push(format!("  compile-error: {}", err));
        }
    }

    Some(lines.join("\n"))
}

fn read_lambda_source_text(source_path: &str) -> Result<String, String> {
    if source_path == "-" {
        let mut input = String::new();
        io
            ::stdin()
            .read_to_string(&mut input)
            .map_err(|e| format!("failed to read lambda source from stdin: {}", e))?;
        return Ok(input);
    }
    fs::read_to_string(source_path).map_err(|e|
        format!("failed to read lambda source '{}': {}", source_path, e)
    )
}

fn parse_lambda_sources_json(path: &str) -> Result<Value, String> {
    let raw = fs
        ::read_to_string(path)
        .map_err(|e| format!("failed to read lambda sources '{}': {}", path, e))?;
    serde_json
        ::from_str(&raw)
        .map_err(|e| format!("failed to parse lambda sources '{}': {}", path, e))
}

fn validate_lambda_program_source(source: &str) -> Result<(), String> {
    let std_ast = crate::baked::load_ast();
    let mut lib_defs = crate::baked::ast_to_definitions(std_ast, "active library")?;
    crate::externals::extend_with_builtin_host_externs(&mut lib_defs)?;
    let merged = crate::parser
        ::merge_std_and_program(source, lib_defs)
        .map_err(|e| format!("lambda source failed to parse/desugar: {}", e))?;
    let (typ, _typed) = infer_with_builtins_typed(
        &merged,
        crate::types::create_builtin_environment(crate::types::TypeEnv::new())
    ).map_err(|e| format!("lambda source failed typecheck: {}", e))?;
    let expected = crate::types::Type::List(Box::new(crate::types::Type::Char));
    if typ != expected {
        return Err(
            format!(
                "lambda source must evaluate to [Char], got {}",
                normalize_signature(&typ.to_string())
            )
        );
    }
    Ok(())
}

fn run_lambda_cli(bin_name: &str, args: &[String]) -> Result<(), String> {
    let Some(command) = args.first().map(String::as_str) else {
        println!("{}", native_lambda_help(bin_name));
        return Ok(());
    };

    match command {
        "help" | "--help" | "-h" => {
            println!("{}", native_lambda_help(bin_name));
        }
        "auth" => {
            match args.get(1).map(String::as_str) {
                Some("--show") => {
                    let key = load_lambda_api_key()?.ok_or_else(|| {
                        "lambda api key is not configured".to_string()
                    })?;
                    println!("{}", key);
                }
                Some("--clear") => {
                    let path = clear_lambda_api_key()?;
                    println!("cleared lambda auth at {}", path.display());
                }
                Some(api_key) if !api_key.trim().is_empty() => {
                    let path = save_lambda_api_key(api_key)?;
                    println!("saved lambda auth at {}", path.display());
                }
                _ => {
                    return Err(
                        format!("Usage: {} lambda auth <api-key> | --show | --clear", bin_name)
                    );
                }
            }
        }
        "create-key" => {
            if args.len() > 2 {
                return Err(format!("Usage: {} lambda create-key [label]", bin_name));
            }
            let client = lambda_client()?;
            let body = match args.get(1) {
                Some(label) => json!({ "label": label }),
                None => json!({}),
            };
            let text = lambda_send(
                client
                    .post(lambda_url("/v1/api-keys", &[])?)
                    .header(CONTENT_TYPE, "application/json")
                    .json(&body)
            )?;
            println!("{}", text);
        }
        "list" => {
            let client = lambda_client()?;
            let api_key = require_lambda_api_key()?;
            let text = lambda_send(
                client
                    .get(lambda_url("/v1/functions", &[])?)
                    .headers(lambda_auth_headers(&api_key)?)
            )?;
            println!("{}", format_lambda_functions_list(&text).unwrap_or(text));
        }
        "invoke" => {
            if args.len() < 2 {
                return Err(
                    format!("Usage: {} lambda invoke <function-name|owner_label/function-name> [--input <text>] [--version <n>] [--param <key=value> ...] [--get|--post]", bin_name)
                );
            }
            let target = parse_lambda_invoke_target(&args[1])?;
            let options = parse_lambda_execute_options(&args[2..])?;
            let mut query = options.params.clone();
            if let Some(version) = &options.version {
                query.push(("version".to_string(), version.clone()));
            }
            let client = lambda_client()?;
            let text = match target {
                LambdaInvokeTarget::OwnedName(function_name) => {
                    let api_key = require_lambda_api_key()?;
                    let list_text = lambda_send(
                        client
                            .get(lambda_url("/v1/functions", &[])?)
                            .headers(lambda_auth_headers(&api_key)?)
                    )?;
                    let function_id = resolve_owned_function_id_from_functions_list(
                        &list_text,
                        &function_name
                    )?;
                    let path = format!("/v1/functions/id/{}/execute", function_id);
                    if options.method == Method::GET {
                        query.push(("input".to_string(), options.input.clone()));
                        lambda_send(
                            client
                                .get(lambda_url(&path, &query)?)
                                .headers(lambda_auth_headers(&api_key)?)
                        )?
                    } else {
                        lambda_send(
                            client
                                .post(lambda_url(&path, &query)?)
                                .headers(lambda_auth_headers(&api_key)?)
                                .header(CONTENT_TYPE, "text/plain")
                                .body(options.input.clone())
                        )?
                    }
                }
                LambdaInvokeTarget::PublicByName { owner_label, function_name } => {
                    let path = format!(
                        "/v1/public/by-name/{}/{}/execute",
                        owner_label,
                        function_name
                    );
                    if options.method == Method::GET {
                        query.push(("input".to_string(), options.input.clone()));
                        lambda_send(client.get(lambda_url(&path, &query)?))?
                    } else {
                        lambda_send(
                            client
                                .post(lambda_url(&path, &query)?)
                                .header(CONTENT_TYPE, "text/plain")
                                .body(options.input.clone())
                        )?
                    }
                }
            };
            println!("{}", text);
        }
        "example" => {
            if args.len() < 2 {
                return Err(
                    format!("Usage: {} lambda example <function-id|public-id> [--input <text>] [--version <n>] [--param <key=value> ...]", bin_name)
                );
            }
            let target_id = args[1].clone();
            let options = parse_lambda_example_options(&args[2..])?;
            let public_id = if target_id.starts_with("pub_") {
                target_id
            } else if target_id.starts_with("fn_") {
                let client = lambda_client()?;
                let api_key = require_lambda_api_key()?;
                let text = lambda_send(
                    client
                        .get(lambda_url(&format!("/v1/functions/id/{}", target_id), &[])?)
                        .headers(lambda_auth_headers(&api_key)?)
                )?;
                let parsed: Value = serde_json
                    ::from_str(&text)
                    .map_err(|e| {
                        format!("failed to parse lambda function metadata response: {}", e)
                    })?;
                resolve_public_id_from_function_json(&parsed)?
            } else {
                return Err(
                    "lambda example expects a function id starting with fn_ or a public id starting with pub_".to_string()
                );
            };
            println!(
                "{}",
                lambda_example_url(
                    &public_id,
                    &options.input,
                    options.version.as_deref(),
                    &options.params
                )?
            );
        }
        "create" | "upload" => {
            if args.len() < 3 {
                return Err(
                    format!(
                        "Usage: {} lambda {} <function-name> <file|-> [--sources <sources.json>]",
                        bin_name,
                        command
                    )
                );
            }
            let function_name = args[1].clone();
            let source_path = args[2].clone();
            let mut sources_path = None;
            let mut i = 3usize;
            while i < args.len() {
                match args[i].as_str() {
                    "--sources" => {
                        i += 1;
                        let value = args
                            .get(i)
                            .ok_or_else(|| "missing value for --sources".to_string())?;
                        sources_path = Some(value.clone());
                    }
                    other => {
                        return Err(format!("unknown upload option: {}", other));
                    }
                }
                i += 1;
            }
            let source_text = read_lambda_source_text(&source_path)?;
            validate_lambda_program_source(&source_text)?;
            let mut payload = json!({ "source_text": source_text });
            if let Some(path) = sources_path {
                let sources = parse_lambda_sources_json(&path)?;
                if let Some(object) = payload.as_object_mut() {
                    object.insert("sources".to_string(), sources);
                }
            }
            let client = lambda_client()?;
            let api_key = require_lambda_api_key()?;
            let text = lambda_send(
                client
                    .post(lambda_url(&format!("/v1/functions/{}", function_name), &[])?)
                    .headers(lambda_auth_headers(&api_key)?)
                    .header(CONTENT_TYPE, "application/json")
                    .json(&payload)
            )?;
            println!("{}", text);
        }
        "get" => {
            if args.len() != 2 {
                return Err(format!("Usage: {} lambda get <function-id>", bin_name));
            }
            let client = lambda_client()?;
            let api_key = require_lambda_api_key()?;
            let text = lambda_send(
                client
                    .get(lambda_url(&format!("/v1/functions/id/{}", args[1]), &[])?)
                    .headers(lambda_auth_headers(&api_key)?)
            )?;
            println!("{}", text);
        }
        "get-source" => {
            if args.len() < 2 || args.len() > 3 {
                return Err(
                    format!("Usage: {} lambda get-source <function-id> [version]", bin_name)
                );
            }
            let mut query = Vec::new();
            if let Some(version) = args.get(2) {
                query.push(("version".to_string(), version.clone()));
            }
            let client = lambda_client()?;
            let api_key = require_lambda_api_key()?;
            let text = lambda_send(
                client
                    .get(lambda_url(&format!("/v1/functions/id/{}/source", args[1]), &query)?)
                    .headers(lambda_auth_headers(&api_key)?)
            )?;
            println!("{}", text);
        }
        "delete" => {
            if args.len() != 2 {
                return Err(format!("Usage: {} lambda delete <function-id>", bin_name));
            }
            let client = lambda_client()?;
            let api_key = require_lambda_api_key()?;
            let text = lambda_send(
                client
                    .delete(lambda_url(&format!("/v1/functions/id/{}", args[1]), &[])?)
                    .headers(lambda_auth_headers(&api_key)?)
            )?;
            println!("{}", text);
        }
        "execute" | "public-execute" => {
            if args.len() < 2 {
                return Err(
                    format!(
                        "Usage: {} lambda {} <id> [--input <text>] [--version <n>] [--param <key=value> ...] [--get|--post]",
                        bin_name,
                        command
                    )
                );
            }
            let id = args[1].clone();
            let options = parse_lambda_execute_options(&args[2..])?;
            let mut query = options.params.clone();
            if let Some(version) = &options.version {
                query.push(("version".to_string(), version.clone()));
            }
            let path = if command == "execute" {
                format!("/v1/functions/id/{}/execute", id)
            } else {
                format!("/v1/public/{}/execute", id)
            };
            let client = lambda_client()?;
            let text = if options.method == Method::GET {
                query.push(("input".to_string(), options.input.clone()));
                let request = client.get(lambda_url(&path, &query)?);
                let request = if command == "execute" {
                    request.headers(lambda_auth_headers(&require_lambda_api_key()?)?)
                } else {
                    request
                };
                lambda_send(request)?
            } else {
                let request = client
                    .post(lambda_url(&path, &query)?)
                    .header(CONTENT_TYPE, "text/plain")
                    .body(options.input.clone());
                let request = if command == "execute" {
                    request.headers(lambda_auth_headers(&require_lambda_api_key()?)?)
                } else {
                    request
                };
                lambda_send(request)?
            };
            println!("{}", text);
        }
        other => {
            return Err(
                format!("unknown lambda command: {}\n{}", other, native_lambda_help(bin_name))
            );
        }
    }

    Ok(())
}

fn native_shell_env_help(bin_name: &str) -> String {
    format!(
        "Environment:\n\
           QUE_LIB_PATH       Override external baked library file path.\n\
           QUE_WASM_OPT       Wasmtime/Cranelift optimization level (default: speed).\n\
                              Allowed: none | speed | speed_and_size.\n\
           QUE_DEVIRTUALIZE   Call-head devirtualization mode (default: aggressive).\n\
                              Allowed: off | known-heads | aggressive.\n\
           QUE_TCO            Tail-call optimization mode (default: conservative).\n\
                              Allowed: conservative | aggressive.\n\
           QUE_BOUNDS_CHECK   Vector get() bounds check (default: on). Disable with 0|false|off|no.\n\
           QUE_VEC_MIN_CAP    Minimum initial vector capacity (default: 2, range: 1..4096).\n\
           QUE_VEC_GROWTH_NUM Vector growth numerator (default: 2, range: 1..64).\n\
           QUE_VEC_GROWTH_DEN Vector growth denominator (default: 1, range: 1..64).\n\
           QUE_DECIMAL_SCALE  Dec fixed-point scale (default: 1000). Must be a power of 10 up to 1000000.\n\
           QUE_DIV_ZERO_CHECK Division/modulo by zero trap check (default: off). Enable with 1|true|on|yes.\n\
           QUE_INT_OVERFLOW_CHECK   Integer overflow trap check for +,-,* and mut ops (default: off).\n\
           QUE_FLOAT_OVERFLOW_CHECK Dec NaN/Inf trap check for +.,-.,*.,/. and mut ops (default: off).\n\
         \n\
         Example:\n\
           QUE_WASM_OPT=speed QUE_DEVIRTUALIZE=aggressive QUE_TCO=conservative QUE_BOUNDS_CHECK=0 QUE_VEC_MIN_CAP=8 QUE_VEC_GROWTH_NUM=3 QUE_VEC_GROWTH_DEN=2 QUE_DECIMAL_SCALE=1000 {bin} script.que\n\
         \n\
         Setup some env flags:\n\
         \n\
           export QUE_WASM_OPT=speed QUE_TCO=aggressive QUE_DEVIRTUALIZE=aggressive QUE_BOUNDS_CHECK=0 QUE_VEC_MIN_CAP=8 QUE_VEC_GROWTH_NUM=2 QUE_VEC_GROWTH_DEN=1 QUE_DECIMAL_SCALE=1000\n\
         \n\
         Fallback to default ones:\n\
         \n\
           unset QUE_WASM_OPT QUE_TCO QUE_DEVIRTUALIZE QUE_BOUNDS_CHECK QUE_VEC_MIN_CAP QUE_VEC_GROWTH_NUM QUE_VEC_GROWTH_DEN QUE_DECIMAL_SCALE",
        bin = bin_name
    )
}

fn native_shell_learn() -> &'static str {
    "Que is a functional, expression-only Lisp with S-expressions.\n\
    \n\
    Core:\n\
    - Function call: (f a b)\n\
    - Nested application works: ((f a) b)\n\
    - Function application is left-associated: (f a b c) means (((f a) b) c), so calling with fewer arguments returns a partially applied function.\n\
    - (apply f a b) is an alias for nested application, so `(apply (f a) b)` matches `((f a) b)`.\n\
    - Everything is an expression; last expression is the return value.\n\
    - (let name value) creates immutable bindings.\n\
    - (do e1 e2 ... en) evaluates in order, returns en, and does NOT create a new scope.\n\
    - Unit is 0 (nil).\n\
    \n\
    Control:\n\
    - (if cond then else)\n\
    - (cond c1 e1 c2 e2 ... default)\n\
    - Branches must return the same type.\n\
    - Loop with (while cond body).\n\
    \n\
    Functions:\n\
    - (lambda a b body)\n\
    - Alternative form: (lambda (a b c) e1 e2 ... en)\n\
    - When parameters are wrapped in parentheses, the body can contain multiple expressions without needing (do ...).\n\
    - The last expression is returned.\n\
    - Recursive functions must use letrec: (letrec f (lambda ... (f ...)))\n\
    - Destructuring works in params:\n\
      - tuples: {a b}\n\
      - vectors: [a b c]\n\
      - '_' skips/ignores a binding slot.\n\
      - vectors use explicit rest: [a b c . rest]\n\
    \n\
    Macros:\n\
    - Top-level only: (letmacro name ...)\n\
    - Single-clause: (letmacro inc1 (lambda x (qq (+ (uq x) 1))))\n\
    - Multi-clause by arity: (letmacro unless ((cond) ...) ((cond body) ...) ((cond then else) ...))\n\
    - Variadic params use '.' before the rest name: (lambda cond . body ...)\n\
    - quote returns syntax as data; qq builds syntax conveniently.\n\
    - qq builds syntax, uq inserts one syntax value, uqs splices a rest syntax list into qq.\n\
    - gensym returns a fresh syntax name for generated bindings.\n\
    - Macro bodies can use compile-time do and let.\n\
    - (macroexpand-1 expr) expands one macro layer and returns the expanded source as a string.\n\
    - (macroexpand expr) fully expands recursively and returns the expanded source as a string.\n\
    - Macros run at compile time before type inference; infer only sees the expanded result.\n\
    \n\
    Types:\n\
    - Int, Dec, Bool, Char\n\
    - Vector [T] (homogeneous)\n\
    - Tuple {A B}\n\
    - String is [Char]\n\
    - Equality example: = (Int), =. (Dec), =# (Char), =? (Bool)\n\
    - Operator suffixes: '.' for Dec, '#' for Char, '?' for Bool.\n\
    - String literal uses double quotes, e.g. \"Hello World\".\n\
    - Char literal uses single quotes, e.g. 'a'.\n\
    \n\
    Pipe operators:\n\
    - `(|> x f g h)` means `(h (g (f x)))` (left-to-right flow with data last)\n\
    - `(comp f g h)` builds a function equivalent to `(lambda x (h (g (f x))))`\n\
    \n\
    Mutation and effects:\n\
    - mut/alter! are for local primitive scalar mutation only (Int/Dec/Bool/Char), same lambda scope.\n\
    - &mut/&alter! are for shared mutation across lambda scopes via boxed references.\n\
    - Vector/state mutation uses set!, push!, pop!.\n\
    - Functions with side effects (mutation or I/O) must end with !.\n\
    - If a function mutates args, the mutated arg must be the first arg.\n\
    - If mutating multiple values, pass them inside the first arg (typically a tuple).\n\
    \n\
    Built-ins:\n\
    - set! pop! length get car cdr cons fst snd while\n\
    + - * / mod = < > <= >= +. -. *. /. mod. =. <. >. <=. >=. +# -# *# /# =# =?\n\
    and or not & | ^ >> << ~ Int->Dec Dec->Int true false nil\n\
    ARGV print! sleep! clear! list-dir! mkdir! read! delete! write! move!"
}

fn binding_name_from_def(expr: &Expression) -> Option<String> {
    let Expression::Apply(items) = expr else {
        return None;
    };
    if items.len() < 3 {
        return None;
    }
    let Expression::Word(keyword) = &items[0] else {
        return None;
    };
    if keyword != "let" && keyword != "letrec" && keyword != "mut" {
        return None;
    }
    let Expression::Word(name) = &items[1] else {
        return None;
    };
    Some(name.clone())
}

fn emit_text_output(out_path: Option<&str>, text: &str) -> Result<(), String> {
    if let Some(path) = out_path {
        fs::write(path, text).map_err(|e| format!("failed to write '{}': {}", path, e))?;
    } else {
        println!("{}", text);
    }
    Ok(())
}

fn emit_bytes_output(out_path: Option<&str>, bytes: &[u8]) -> Result<(), String> {
    if let Some(path) = out_path {
        fs::write(path, bytes).map_err(|e| format!("failed to write '{}': {}", path, e))?;
    } else {
        let mut stdout = io::stdout().lock();
        stdout.write_all(bytes).map_err(|e| format!("failed to write stdout: {}", e))?;
        stdout.flush().map_err(|e| format!("failed to flush stdout: {}", e))?;
    }
    Ok(())
}

fn format_top_level_type_lines(typed: &TypedExpression, user_form_count: usize) -> String {
    let forms = user_form_nodes(typed, user_form_count);
    let mut lines = Vec::new();

    for (idx, form) in forms.iter().enumerate() {
        if let Some(name) = binding_name_from_def(&form.expr) {
            let binding_typ = form.children
                .get(2)
                .and_then(|child| child.typ.as_ref())
                .or(form.typ.as_ref());
            let rendered = binding_typ
                .map(|typ| normalize_signature(&typ.to_string()))
                .unwrap_or_else(|| "_".to_string());
            lines.push(format!("{} : {}", name, rendered));
        } else {
            let rendered = form.typ
                .as_ref()
                .map(|typ| normalize_signature(&typ.to_string()))
                .unwrap_or_else(|| "_".to_string());
            lines.push(format!("form[{}] : {}", idx, rendered));
        }
    }

    let result_type = forms
        .last()
        .and_then(|form| form.typ.as_ref())
        .or(typed.typ.as_ref())
        .map(|typ| normalize_signature(&typ.to_string()))
        .unwrap_or_else(|| "_".to_string());
    lines.push(format!("result : {}", result_type));

    lines.join("\n")
}

fn active_library_definitions() -> Result<Vec<Expression>, String> {
    let mut defs = crate::baked::ast_to_definitions(crate::baked::load_ast(), "active")?;
    crate::externals::extend_with_builtin_host_externs(&mut defs)?;
    Ok(defs)
}

fn wildcard_match(pattern: &str, text: &str) -> bool {
    let p = pattern.as_bytes();
    let t = text.as_bytes();
    let mut dp = vec![vec![false; t.len() + 1]; p.len() + 1];
    dp[0][0] = true;
    for i in 1..=p.len() {
        if p[i - 1] == b'*' {
            dp[i][0] = dp[i - 1][0];
        }
    }
    for i in 1..=p.len() {
        for j in 1..=t.len() {
            dp[i][j] = match p[i - 1] {
                b'*' => dp[i - 1][j] || dp[i][j - 1],
                b'?' => dp[i - 1][j - 1],
                c => c == t[j - 1] && dp[i - 1][j - 1],
            };
        }
    }
    dp[p.len()][t.len()]
}

fn infer_library_symbol_type(name: &str, lib_defs: &[Expression]) -> Result<String, String> {
    let merged = crate::parser::merge_std_and_program(name, lib_defs.to_vec())?;
    let (typ, _typed) = infer_with_builtins_typed(
        &merged,
        crate::types::create_builtin_environment(crate::types::TypeEnv::new())
    )?;
    Ok(normalize_signature(&typ.to_string()))
}

fn run_library_explore_via_io(args: &[String]) -> Result<(), String> {
    if args.is_empty() || args[0] == "--help" || args[0] == "-h" {
        println!(
            "Usage: queio --lib names [pattern]\n\
             or:    queio --lib types [pattern]\n\
             or:    queio --lib source <name>\n\
             \n\
             Wildcards:\n\
               *  matches any sequence\n\
               ?  matches one character\n\
             \n\
             Examples:\n\
               queio --lib names '*map*'\n\
               queio --lib types 'std/vector/*'\n\
               queio --lib source map"
        );
        return Ok(());
    }

    let lib_defs = active_library_definitions()?;
    let mut by_name: BTreeMap<String, Expression> = BTreeMap::new();
    for def in &lib_defs {
        if let Some(name) = binding_name_from_def(def) {
            by_name.insert(name, def.clone());
        }
    }
    let all_names = by_name.keys().cloned().collect::<Vec<_>>();

    match args[0].as_str() {
        "names" => {
            let pattern = args.get(1).map(String::as_str).unwrap_or("*");
            if args.len() > 2 {
                return Err("Usage: queio --lib names [pattern]".to_string());
            }
            for name in all_names.iter().filter(|name| wildcard_match(pattern, name)) {
                println!("{}", name);
            }
            Ok(())
        }
        "types" => {
            let pattern = args.get(1).map(String::as_str).unwrap_or("*");
            if args.len() > 2 {
                return Err("Usage: queio --lib types [pattern]".to_string());
            }
            for name in all_names.iter().filter(|name| wildcard_match(pattern, name)) {
                match infer_library_symbol_type(name, &lib_defs) {
                    Ok(typ) => println!("{} : {}", name, typ),
                    Err(err) => println!("{} : <type error: {}>", name, err),
                }
            }
            Ok(())
        }
        "source" => {
            if args.len() != 2 {
                return Err("Usage: queio --lib source <name>".to_string());
            }
            let name = &args[1];
            let Some(expr) = by_name.get(name) else {
                return Err(format!("library symbol '{}' not found", name));
            };
            println!("name: {}", name);
            println!("source:");
            println!("{}", expr.to_lisp());
            Ok(())
        }
        other => Err(format!("unknown --lib command '{}'", other)),
    }
}

enum LibraryInstallMode {
    Install,
    Uninstall,
}

fn load_existing_library_definitions(path: &Path) -> Result<Vec<Expression>, String> {
    if path.exists() {
        let ast = crate::baked::load_ast_from_path(path)?;
        return crate::baked::ast_to_definitions(ast, &path.display().to_string());
    }
    Ok(Vec::new())
}

fn run_library_install_via_io(mode: LibraryInstallMode, args: &[String]) -> Result<(), String> {
    let mut argv = args.to_vec();
    if take_help_flag_from_argv(&mut argv) {
        println!(
            "Usage: queio --install [helpers.que ...] [--out <lib.lisp>]\n\
             or:    queio --uninstall [--out <lib.lisp>]"
        );
        return Ok(());
    }

    let out_path = take_install_output_path_from_argv(&mut argv).map_err(|e|
        format!("invalid install args: {}", e)
    )?;
    let mut bundle_paths = Vec::new();
    for token in argv {
        if token.starts_with("--") {
            return Err(format!("unknown install flag '{}'", token));
        }
        bundle_paths.push(token);
    }

    if matches!(mode, LibraryInstallMode::Uninstall) && !bundle_paths.is_empty() {
        return Err("--uninstall does not accept bundle paths".to_string());
    }

    let output = out_path.map(PathBuf::from).unwrap_or_else(crate::baked::external_library_path);

    if matches!(mode, LibraryInstallMode::Uninstall) {
        if output.exists() {
            fs
                ::remove_file(&output)
                .map_err(|e| format!("failed to remove library '{}': {}", output.display(), e))?;
            eprintln!("library uninstalled from {}", output.display());
        } else {
            eprintln!("library '{}' is already absent", output.display());
        }
        return Ok(());
    }

    let cwd = env::current_dir().map_err(|e| format!("failed to read current directory: {}", e))?;
    let mut defs = load_existing_library_definitions(&output)?;
    defs.extend(crate::project::load_bundle_definitions(&cwd, &bundle_paths)?);
    let wrapped = Expression::Apply(
        std::iter::once(Expression::Word("do".to_string())).chain(defs).collect()
    );

    if let Some(parent) = output.parent() {
        if !parent.as_os_str().is_empty() {
            fs
                ::create_dir_all(parent)
                .map_err(|e| format!("failed to create '{}': {}", parent.display(), e))?;
        }
    }
    fs
        ::write(&output, format!("{}\n", wrapped.to_lisp()))
        .map_err(|e| { format!("failed to write baked library '{}': {}", output.display(), e) })?;
    eprintln!("library installed to {}", output.display());
    Ok(())
}

pub struct ShellStoreData {
    pub wasi_ctx: WasiCtx,
    pub resource_table: ResourceTable,
    wasi_p1_ctx: wasmtime_wasi::p1::WasiP1Ctx,
    pub script_cwd: Option<PathBuf>,
    pub shell_policy: ShellPolicy,
}

impl ShellStoreData {
    pub fn new_with_security(
        script_cwd: Option<PathBuf>,
        shell_policy: ShellPolicy
    ) -> wasmtime::Result<Self> {
        let mut p2_builder = WasiCtxBuilder::new();
        p2_builder.inherit_stdio();
        p2_builder.inherit_args();
        p2_builder.inherit_env();

        let mut p1_builder = WasiCtxBuilder::new();
        p1_builder.inherit_stdio();
        p1_builder.inherit_args();
        p1_builder.inherit_env();

        Ok(Self {
            wasi_ctx: p2_builder.build(),
            resource_table: ResourceTable::new(),
            wasi_p1_ctx: p1_builder.build_p1(),
            script_cwd,
            shell_policy,
        })
    }
}

impl WasiView for ShellStoreData {
    fn ctx(&mut self) -> WasiCtxView<'_> {
        WasiCtxView {
            ctx: &mut self.wasi_ctx,
            table: &mut self.resource_table,
        }
    }
}

fn memory_export(caller: &mut Caller<'_, ShellStoreData>) -> wasmtime::Result<Memory> {
    caller
        .get_export("memory")
        .and_then(Extern::into_memory)
        .ok_or_else(|| wasmtime::Error::msg("guest export 'memory' not found"))
}

fn read_i32(
    memory: &Memory,
    caller: &Caller<'_, ShellStoreData>,
    addr: i32
) -> wasmtime::Result<i32> {
    let offset = usize
        ::try_from(addr)
        .map_err(|_| wasmtime::Error::msg(format!("invalid read address: {}", addr)))?;
    let mut bytes = [0u8; 4];
    memory
        .read(caller, offset, &mut bytes)
        .map_err(|_| wasmtime::Error::msg(format!("out of bounds read at {}", addr)))?;
    Ok(i32::from_le_bytes(bytes))
}

fn write_i32(
    memory: &Memory,
    caller: &mut Caller<'_, ShellStoreData>,
    addr: i32,
    value: i32
) -> wasmtime::Result<()> {
    let offset = usize
        ::try_from(addr)
        .map_err(|_| wasmtime::Error::msg(format!("invalid write address: {}", addr)))?;
    memory
        .write(caller, offset, &value.to_le_bytes())
        .map_err(|_| wasmtime::Error::msg(format!("out of bounds write at {}", addr)))
}

fn guest_alloc(caller: &mut Caller<'_, ShellStoreData>) -> wasmtime::Result<TypedFunc<i32, i32>> {
    for name in ["$alloc", "alloc"] {
        if let Some(func) = caller.get_export(name).and_then(Extern::into_func) {
            if let Ok(typed) = func.typed::<i32, i32>(&mut *caller) {
                return Ok(typed);
            }
        }
    }
    Err(wasmtime::Error::msg("guest export '$alloc'/'alloc' not found"))
}

pub fn read_lisp_vector(
    caller: &mut Caller<'_, ShellStoreData>,
    vec_ptr: i32
) -> wasmtime::Result<Vec<i32>> {
    let memory = memory_export(caller)?;
    let len = read_i32(&memory, &*caller, vec_ptr + VEC_LEN_OFFSET)?;
    let data_ptr = read_i32(&memory, &*caller, vec_ptr + VEC_DATA_PTR_OFFSET)?;
    if len < 0 {
        return Err(wasmtime::Error::msg(format!("negative vector len: {}", len)));
    }

    let mut values = Vec::with_capacity(len as usize);
    for i in 0..len {
        values.push(read_i32(&memory, &*caller, data_ptr + i * 4)?);
    }
    Ok(values)
}

pub fn write_lisp_vector(
    caller: &mut Caller<'_, ShellStoreData>,
    values: &[i32]
) -> wasmtime::Result<i32> {
    let alloc = guest_alloc(caller)?;
    let vec_len = i32
        ::try_from(values.len())
        .map_err(|_| wasmtime::Error::msg("output too large for i32 vector length"))?;
    let header_ptr = alloc.call(&mut *caller, VEC_HEADER_SIZE)?;
    let data_ptr = alloc.call(&mut *caller, vec_len * 4)?;
    let memory = memory_export(caller)?;

    for (i, value) in values.iter().copied().enumerate() {
        let offset =
            i32::try_from(i).map_err(|_| wasmtime::Error::msg("output index overflow"))? * 4;
        write_i32(&memory, caller, data_ptr + offset, value)?;
    }

    write_i32(&memory, caller, header_ptr + VEC_LEN_OFFSET, vec_len)?;
    write_i32(&memory, caller, header_ptr + VEC_CAP_OFFSET, vec_len)?;
    write_i32(&memory, caller, header_ptr + VEC_RC_OFFSET, 1)?;
    write_i32(&memory, caller, header_ptr + VEC_ELEM_REF_OFFSET, 0)?;
    write_i32(&memory, caller, header_ptr + VEC_DATA_PTR_OFFSET, data_ptr)?;
    write_i32(&memory, caller, header_ptr + VEC_MAGIC_OFFSET, VEC_MAGIC)?;
    Ok(header_ptr)
}

fn read_lisp_string(
    caller: &mut Caller<'_, ShellStoreData>,
    vec_ptr: i32
) -> wasmtime::Result<String> {
    let codes = read_lisp_vector(caller, vec_ptr)?;
    Ok(
        codes
            .into_iter()
            .map(|n| char::from_u32(n as u32).unwrap_or('\u{FFFD}'))
            .collect::<String>()
    )
}

fn write_lisp_string(
    caller: &mut Caller<'_, ShellStoreData>,
    value: &str
) -> wasmtime::Result<i32> {
    let codes = value
        .chars()
        .map(|c| i32::try_from(u32::from(c)).unwrap_or(0))
        .collect::<Vec<_>>();
    write_lisp_vector(caller, &codes)
}

fn resolve_target_path(caller: &Caller<'_, ShellStoreData>, raw: &str) -> PathBuf {
    let candidate = Path::new(raw);
    if candidate.is_absolute() {
        return candidate.to_path_buf();
    }

    if let Some(script_cwd) = caller.data().script_cwd.as_ref() {
        return script_cwd.join(candidate);
    }

    candidate.to_path_buf()
}

fn list_dir_text(path: &Path) -> Result<String, String> {
    let entries = fs
        ::read_dir(path)
        .map_err(|e: io::Error| format!("failed to read directory '{}': {}", path.display(), e))?;
    let mut names = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|e: io::Error| format!("failed to read dir entry: {}", e))?;
        names.push(entry.file_name().to_string_lossy().into_owned());
    }
    names.sort();
    if names.is_empty() {
        Ok(String::new())
    } else {
        Ok(format!("{}\n", names.join("\n")))
    }
}

pub fn host_list_dir(
    mut caller: Caller<'_, ShellStoreData>,
    path_vec_ptr: i32
) -> wasmtime::Result<i32> {
    let path = read_lisp_string(&mut caller, path_vec_ptr)?;
    caller
        .data()
        .shell_policy.require(ShellPermission::Read, "list-dir!", &path)
        .map_err(wasmtime::Error::msg)?;

    let target = resolve_target_path(&caller, &path);
    let output = list_dir_text(&target).map_err(wasmtime::Error::msg)?;
    write_lisp_string(&mut caller, &output)
}

pub fn host_read_file(
    mut caller: Caller<'_, ShellStoreData>,
    path_vec_ptr: i32
) -> wasmtime::Result<i32> {
    let path = read_lisp_string(&mut caller, path_vec_ptr)?;
    caller
        .data()
        .shell_policy.require(ShellPermission::Read, "read!", &path)
        .map_err(wasmtime::Error::msg)?;

    let target = resolve_target_path(&caller, &path);
    let output = fs
        ::read_to_string(&target)
        .map_err(|e| {
            wasmtime::Error::msg(format!("failed to read '{}': {}", target.display(), e))
        })?;
    write_lisp_string(&mut caller, &output)
}

pub fn host_write_file(
    mut caller: Caller<'_, ShellStoreData>,
    path_vec_ptr: i32,
    data_vec_ptr: i32
) -> wasmtime::Result<i32> {
    let path = read_lisp_string(&mut caller, path_vec_ptr)?;
    let data = read_lisp_string(&mut caller, data_vec_ptr)?;
    caller
        .data()
        .shell_policy.require(ShellPermission::Write, "write!", &path)
        .map_err(wasmtime::Error::msg)?;

    let target = resolve_target_path(&caller, &path);
    if let Some(parent) = target.parent() {
        if !parent.as_os_str().is_empty() {
            fs
                ::create_dir_all(parent)
                .map_err(|e| {
                    wasmtime::Error::msg(
                        format!("failed to create parent dirs '{}': {}", parent.display(), e)
                    )
                })?;
        }
    }
    fs
        ::write(&target, data.as_bytes())
        .map_err(|e| {
            wasmtime::Error::msg(format!("failed to write '{}': {}", target.display(), e))
        })?;

    Ok(0)
}

pub fn host_mkdir_p(
    mut caller: Caller<'_, ShellStoreData>,
    path_vec_ptr: i32
) -> wasmtime::Result<i32> {
    let path = read_lisp_string(&mut caller, path_vec_ptr)?;
    caller
        .data()
        .shell_policy.require(ShellPermission::Write, "mkdir!", &path)
        .map_err(wasmtime::Error::msg)?;

    let target = resolve_target_path(&caller, &path);
    fs
        ::create_dir_all(&target)
        .map_err(|e| {
            wasmtime::Error::msg(format!("failed to mkdir '{}': {}", target.display(), e))
        })?;
    Ok(0)
}

pub fn host_delete(
    mut caller: Caller<'_, ShellStoreData>,
    path_vec_ptr: i32
) -> wasmtime::Result<i32> {
    let path = read_lisp_string(&mut caller, path_vec_ptr)?;
    caller
        .data()
        .shell_policy.require(ShellPermission::Delete, "delete!", &path)
        .map_err(wasmtime::Error::msg)?;

    let target = resolve_target_path(&caller, &path);
    let meta = fs
        ::symlink_metadata(&target)
        .map_err(|e| {
            wasmtime::Error::msg(
                format!("failed to inspect path '{}' for delete: {}", target.display(), e)
            )
        })?;
    if meta.is_dir() {
        fs
            ::remove_dir_all(&target)
            .map_err(|e| {
                wasmtime::Error::msg(
                    format!("failed to delete directory '{}': {}", target.display(), e)
                )
            })?;
    } else {
        fs
            ::remove_file(&target)
            .map_err(|e| {
                wasmtime::Error::msg(format!("failed to delete file '{}': {}", target.display(), e))
            })?;
    }
    Ok(0)
}

pub fn host_move(
    mut caller: Caller<'_, ShellStoreData>,
    src_vec_ptr: i32,
    dst_vec_ptr: i32
) -> wasmtime::Result<i32> {
    let src = read_lisp_string(&mut caller, src_vec_ptr)?;
    let dst = read_lisp_string(&mut caller, dst_vec_ptr)?;
    caller
        .data()
        .shell_policy.require(ShellPermission::Write, "move!", &format!("{} -> {}", src, dst))
        .map_err(wasmtime::Error::msg)?;

    let src_path = resolve_target_path(&caller, &src);
    let dst_path = resolve_target_path(&caller, &dst);
    if let Some(parent) = dst_path.parent() {
        if !parent.as_os_str().is_empty() {
            fs
                ::create_dir_all(parent)
                .map_err(|e| {
                    wasmtime::Error::msg(
                        format!("failed to create destination dirs '{}': {}", parent.display(), e)
                    )
                })?;
        }
    }
    fs
        ::rename(&src_path, &dst_path)
        .map_err(|e| {
            wasmtime::Error::msg(
                format!(
                    "failed to move '{}' to '{}': {}",
                    src_path.display(),
                    dst_path.display(),
                    e
                )
            )
        })?;

    Ok(0)
}

pub fn host_print(
    mut caller: Caller<'_, ShellStoreData>,
    text_vec_ptr: i32
) -> wasmtime::Result<i32> {
    let text = read_lisp_string(&mut caller, text_vec_ptr)?;
    caller
        .data()
        .shell_policy.require(ShellPermission::Write, "print!", "<stdout>")
        .map_err(wasmtime::Error::msg)?;

    let mut out = io::stdout();
    out
        .write_all(text.as_bytes())
        .map_err(|e| wasmtime::Error::msg(format!("failed to write stdout: {}", e)))?;
    out.flush().map_err(|e| wasmtime::Error::msg(format!("failed to flush stdout: {}", e)))?;
    Ok(0)
}

pub fn host_sleep(caller: Caller<'_, ShellStoreData>, millis: i32) -> wasmtime::Result<i32> {
    caller
        .data()
        .shell_policy.require(ShellPermission::Write, "sleep!", "<clock>")
        .map_err(wasmtime::Error::msg)?;

    if millis < 0 {
        return Err(wasmtime::Error::msg(format!("sleep! expects non-negative ms, got {}", millis)));
    }
    thread::sleep(Duration::from_millis(millis as u64));
    Ok(0)
}

pub fn host_clear(caller: Caller<'_, ShellStoreData>) -> wasmtime::Result<i32> {
    caller
        .data()
        .shell_policy.require(ShellPermission::Write, "clear!", "<stdout>")
        .map_err(wasmtime::Error::msg)?;

    let mut out = io::stdout();
    out
        .write_all(b"\x1b[2J\x1b[H")
        .map_err(|e| wasmtime::Error::msg(format!("failed to clear stdout: {}", e)))?;
    out.flush().map_err(|e| wasmtime::Error::msg(format!("failed to flush stdout: {}", e)))?;
    Ok(0)
}

pub fn add_shell_to_linker(linker: &mut Linker<ShellStoreData>) -> wasmtime::Result<()> {
    // Core wasm modules (like this backend) use WASIp1 imports.
    wasmtime_wasi::p1::add_to_linker_sync(linker, |state| &mut state.wasi_p1_ctx)?;
    linker.func_wrap("host", "list_dir", host_list_dir)?;
    linker.func_wrap("host", "read_file", host_read_file)?;
    linker.func_wrap("host", "write_file", host_write_file)?;
    linker.func_wrap("host", "mkdir_p", host_mkdir_p)?;
    linker.func_wrap("host", "delete", host_delete)?;
    linker.func_wrap("host", "move", host_move)?;
    linker.func_wrap("host", "print", host_print)?;
    linker.func_wrap("host", "sleep", host_sleep)?;
    linker.func_wrap("host", "clear", host_clear)?;
    Ok(())
}

fn user_form_nodes<'a>(
    typed: &'a TypedExpression,
    user_form_count: usize
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

fn format_scope_path(scope: Option<&InferErrorScope>) -> String {
    match scope {
        Some(meta) => {
            let lambda_path = if meta.lambda_path.is_empty() {
                "<root>".to_string()
            } else {
                meta.lambda_path
                    .iter()
                    .map(|idx| format!("#{}", idx))
                    .collect::<Vec<String>>()
                    .join(" -> ")
            };
            format!("top_form={} lambda_path={}", meta.user_top_form, lambda_path)
        }
        None => "<none>".to_string(),
    }
}

fn format_range_line(range_idx: usize, range: crate::lsp_native_core::CoreRange) -> String {
    format!(
        "location[{}]: {}:{} -> {}:{}",
        range_idx,
        range.start.line + 1,
        range.start.character + 1,
        range.end.line + 1,
        range.end.character + 1
    )
}

fn push_location_lines(
    out: &mut Vec<String>,
    source_text: &str,
    message: &str,
    scope: Option<&InferErrorScope>
) {
    let should_locate = scope.is_some() || extract_error_snippet(message).is_some();
    if !should_locate {
        out.push("location: <unresolved>".to_string());
        return;
    }

    let ranges = infer_error_ranges(source_text, message, scope);
    if ranges.is_empty() {
        out.push("location: <unresolved>".to_string());
        return;
    }

    for (idx, range) in ranges.iter().copied().take(8).enumerate() {
        out.push(format_range_line(idx, range));
        if let Some(line) = source_text.lines().nth(range.start.line as usize) {
            out.push(format!("location_line[{}]: {}", idx, line.trim_end()));
        }
    }

    if ranges.len() > 8 {
        out.push(format!("location_more: {}", ranges.len() - 8));
    }
}

fn typed_node_label(expr: &Expression) -> String {
    match expr {
        Expression::Word(name) => name.clone(),
        Expression::Int(v) => v.to_string(),
        Expression::Dec(v) => format!("{:?}", v),
        Expression::Apply(items) => {
            if items.is_empty() {
                return "()".to_string();
            }
            match &items[0] {
                Expression::Word(head) => format!("({} ...)", head),
                _ => "(apply ...)".to_string(),
            }
        }
    }
}

fn is_lambda_expr(expr: &Expression) -> bool {
    if let Expression::Apply(items) = expr {
        if let Some(Expression::Word(head)) = items.first() {
            return head == "lambda";
        }
    }
    false
}

fn lambda_body_child(node: &TypedExpression) -> Option<&TypedExpression> {
    if !is_lambda_expr(&node.expr) {
        return None;
    }
    node.children.last()
}

fn find_nth_lambda_in_scope<'a>(
    root: &'a TypedExpression,
    nth: usize
) -> Option<&'a TypedExpression> {
    fn walk<'a>(
        node: &'a TypedExpression,
        nth: usize,
        counter: &mut usize
    ) -> Option<&'a TypedExpression> {
        if is_lambda_expr(&node.expr) {
            if *counter == nth {
                return Some(node);
            }
            *counter += 1;
            // Do not recurse inside lambda body at this depth:
            // nested lambdas belong to the next scope depth.
            return None;
        }

        for child in &node.children {
            if let Some(found) = walk(child, nth, counter) {
                return Some(found);
            }
        }
        None
    }

    let mut counter = 0usize;
    walk(root, nth, &mut counter)
}

fn scope_focus_node<'a>(
    typed: &'a TypedExpression,
    user_form_count: usize,
    scope: Option<&InferErrorScope>
) -> Option<(usize, &'a TypedExpression)> {
    let scope = scope?;
    let forms = user_form_nodes(typed, user_form_count);
    let form = *forms.get(scope.user_top_form)?;
    let mut cursor = form;

    for lambda_idx in &scope.lambda_path {
        let lambda_node = find_nth_lambda_in_scope(cursor, *lambda_idx)?;
        cursor = lambda_body_child(lambda_node).unwrap_or(lambda_node);
    }

    Some((scope.user_top_form, cursor))
}

fn push_typed_tree_lines(
    out: &mut Vec<String>,
    node: &TypedExpression,
    depth: usize,
    max_nodes: usize
) {
    if out.len() >= max_nodes {
        return;
    }

    let indent = "  ".repeat(depth);
    let typ = node.typ
        .as_ref()
        .map(|t| t.to_string())
        .unwrap_or_else(|| "_".to_string());
    out.push(format!("{}{} :: {}", indent, typed_node_label(&node.expr), typ));

    for child in &node.children {
        if out.len() >= max_nodes {
            break;
        }
        push_typed_tree_lines(out, child, depth + 1, max_nodes);
    }
}

fn typed_user_forms_debug_dump(typed: &TypedExpression, user_form_count: usize) -> String {
    let mut lines = Vec::new();
    let forms = user_form_nodes(typed, user_form_count);
    for (idx, form) in forms.iter().enumerate() {
        lines.push(format!("form[{}]:", idx));
        push_typed_tree_lines(&mut lines, form, 1, 220);
        if lines.len() >= 220 {
            break;
        }
    }

    if lines.len() >= 220 {
        lines.push("... truncated ...".to_string());
    }

    lines.join("\n")
}

fn build_debug_error_report(
    debug_mode: DebugMode,
    phase: &str,
    source_text: &str,
    message: &str,
    scope: Option<&InferErrorScope>,
    user_desugared: Option<&Expression>,
    user_form_count: usize,
    typed: Option<&TypedExpression>
) -> String {
    let mut out = Vec::new();
    out.push(format!("debug.phase: {}", phase));
    out.push(format!("debug.error: {}", message));
    if debug_mode.includes_code() || debug_mode.includes_types() {
        out.push(format!("debug.summary: {}", diagnostic_summary_without_snippet(message)));
    }
    out.push(format!("debug.scope_path: {}", format_scope_path(scope)));
    out.push(
        "debug.location_explainer: location[i] ranges are in the original source file (not desugared), 1-based line:column; i=0 is the primary match.".to_string()
    );
    push_location_lines(&mut out, source_text, message, scope);

    if debug_mode.includes_code() {
        if let Some(desugared) = user_desugared {
            out.push("debug.desugared_source:".to_string());
            out.push(desugared.to_lisp());
        }
    }

    if debug_mode.includes_types() {
        if let Some(typed_ast) = typed {
            if let Some((form_idx, focus)) = scope_focus_node(typed_ast, user_form_count, scope) {
                let mut focus_lines = Vec::new();
                push_typed_tree_lines(&mut focus_lines, focus, 0, usize::MAX);
                out.push(
                    format!("debug.focus: form={} scope={}", form_idx, format_scope_path(scope))
                );
                out.extend(focus_lines);
            }

            out.push("debug.types:".to_string());
            out.push(typed_user_forms_debug_dump(typed_ast, user_form_count));
        }
    }

    out.join("\n")
}

#[cfg(test)]
mod tests {
    use super::{
        format_lambda_functions_list,
        init_project_config_file,
        lambda_api_base_from_env_value,
        lambda_example_url,
        native_shell_help,
        parse_lambda_example_options,
        parse_lambda_execute_options,
        parse_lambda_invoke_target,
        resolve_project_entry_path,
        resolve_owned_function_id_from_functions_list,
        take_debug_mode_from_argv,
        take_emit_request_from_argv,
        take_help_flag_from_argv,
        take_no_result_flag_from_argv,
        take_shell_policy_from_argv,
        validate_lambda_program_source,
        wildcard_match,
        DebugMode,
        EmitKind,
        LambdaInvokeTarget,
        ShellPermission,
        ShellPolicy,
    };
    use reqwest::Method;
    use std::collections::HashSet;

    #[test]
    fn parse_policy_empty_permissions() {
        let mut args = vec!["alpha".to_string(), "--allow".to_string()];
        let policy = take_shell_policy_from_argv(&mut args).unwrap();
        assert_eq!(args, vec!["alpha".to_string()]);
        assert!(policy.require(ShellPermission::Read, "read", "./x").is_err());
    }

    #[test]
    fn parse_policy_with_permissions() {
        let mut args = vec![
            "main.que".to_string(),
            "--allow".to_string(),
            "read".to_string(),
            "write".to_string()
        ];
        let policy = take_shell_policy_from_argv(&mut args).unwrap();
        assert_eq!(args, vec!["main.que".to_string()]);
        assert!(policy.require(ShellPermission::Read, "read", "./x").is_ok());
        assert!(policy.require(ShellPermission::Write, "mkdir", "./x").is_ok());
        assert!(policy.require(ShellPermission::Delete, "delete", "./x").is_err());
    }

    #[test]
    fn parse_policy_rejects_unknown_permission() {
        let mut args = vec!["main.que".to_string(), "--allow".to_string(), "foo".to_string()];
        let err = take_shell_policy_from_argv(&mut args).unwrap_err();
        assert!(err.contains("unknown shell permission 'foo'"));
    }

    #[test]
    fn disabled_policy_blocks_operations() {
        let policy = ShellPolicy::disabled();
        let err = policy.require(ShellPermission::Read, "read", "./x").unwrap_err();
        assert!(err.contains("host io is disabled"));
    }

    #[test]
    fn policy_requires_specific_permission() {
        let mut perms = HashSet::new();
        perms.insert(ShellPermission::Read);
        let policy = ShellPolicy::enabled(perms);
        assert!(policy.require(ShellPermission::Read, "list-dir", ".").is_ok());
        assert!(policy.require(ShellPermission::Write, "mkdir", "./x").is_err());
    }

    #[test]
    fn take_debug_basic_strips_flag() {
        let mut args = vec![
            "script.que".to_string(),
            "foo".to_string(),
            "--debug".to_string(),
            "bar".to_string()
        ];
        let mode = take_debug_mode_from_argv(&mut args);
        assert_eq!(mode, DebugMode::Basic);
        assert_eq!(args, vec!["script.que".to_string(), "foo".to_string(), "bar".to_string()]);
    }

    #[test]
    fn take_debug_all_and_allow_can_coexist() {
        let mut args = vec![
            "script.que".to_string(),
            "--debug".to_string(),
            "all".to_string(),
            "--allow".to_string(),
            "read".to_string()
        ];
        let mode = take_debug_mode_from_argv(&mut args);
        assert_eq!(mode, DebugMode::All);
        let policy = take_shell_policy_from_argv(&mut args).expect("policy should parse");
        assert_eq!(args, vec!["script.que".to_string()]);
        assert!(policy.require(ShellPermission::Read, "read", "./x").is_ok());
    }

    #[test]
    fn take_debug_code_mode() {
        let mut args = vec!["script.que".to_string(), "--debug".to_string(), "code".to_string()];
        let mode = take_debug_mode_from_argv(&mut args);
        assert_eq!(mode, DebugMode::Code);
        assert_eq!(args, vec!["script.que".to_string()]);
    }

    #[test]
    fn take_debug_types_mode() {
        let mut args = vec!["script.que".to_string(), "--debug".to_string(), "types".to_string()];
        let mode = take_debug_mode_from_argv(&mut args);
        assert_eq!(mode, DebugMode::Types);
        assert_eq!(args, vec!["script.que".to_string()]);
    }

    #[test]
    fn take_debug_code_plus_types_merges_to_all() {
        let mut args = vec![
            "script.que".to_string(),
            "--debug".to_string(),
            "code".to_string(),
            "--debug".to_string(),
            "types".to_string()
        ];
        let mode = take_debug_mode_from_argv(&mut args);
        assert_eq!(mode, DebugMode::All);
        assert_eq!(args, vec!["script.que".to_string()]);
    }

    #[test]
    fn take_debug_does_not_consume_unrelated_next_arg() {
        let mut args = vec![
            "script.que".to_string(),
            "--debug".to_string(),
            "user-arg".to_string()
        ];
        let mode = take_debug_mode_from_argv(&mut args);
        assert_eq!(mode, DebugMode::Basic);
        assert_eq!(args, vec!["script.que".to_string(), "user-arg".to_string()]);
    }

    #[test]
    fn take_help_strips_help_flags() {
        let mut args = vec![
            "script.que".to_string(),
            "--help".to_string(),
            "-h".to_string(),
            "user-arg".to_string()
        ];
        let has_help = take_help_flag_from_argv(&mut args);
        assert!(has_help);
        assert_eq!(args, vec!["script.que".to_string(), "user-arg".to_string()]);
    }

    #[test]
    fn take_help_returns_false_when_missing() {
        let mut args = vec!["script.que".to_string(), "user-arg".to_string()];
        let has_help = take_help_flag_from_argv(&mut args);
        assert!(!has_help);
        assert_eq!(args, vec!["script.que".to_string(), "user-arg".to_string()]);
    }

    #[test]
    fn take_no_result_strips_flag() {
        let mut args = vec![
            "script.que".to_string(),
            "--no-result".to_string(),
            "user-arg".to_string()
        ];
        let has_no_result = take_no_result_flag_from_argv(&mut args);
        assert!(has_no_result);
        assert_eq!(args, vec!["script.que".to_string(), "user-arg".to_string()]);
    }

    #[test]
    fn take_no_result_returns_false_when_missing() {
        let mut args = vec!["script.que".to_string(), "user-arg".to_string()];
        let has_no_result = take_no_result_flag_from_argv(&mut args);
        assert!(!has_no_result);
        assert_eq!(args, vec!["script.que".to_string(), "user-arg".to_string()]);
    }

    #[test]
    fn take_emit_request_parses_kind_and_out_path() {
        let mut args = vec![
            "script.que".to_string(),
            "user-arg".to_string(),
            "--emit".to_string(),
            "wat".to_string(),
            "--out".to_string(),
            "out.wat".to_string()
        ];
        let request = take_emit_request_from_argv(&mut args).expect("emit should parse");
        assert_eq!(args, vec!["script.que".to_string(), "user-arg".to_string()]);
        let request = request.expect("emit request should exist");
        assert_eq!(request.kind, EmitKind::Wat);
        assert_eq!(request.out_path.as_deref(), Some("out.wat"));
    }

    #[test]
    fn take_emit_request_parses_legacy_emit_source_flag() {
        let mut args = vec!["script.que".to_string(), "--emit-source".to_string()];
        let request = take_emit_request_from_argv(&mut args).expect("legacy emit should parse");
        assert_eq!(args, vec!["script.que".to_string()]);
        let request = request.expect("emit request should exist");
        assert_eq!(request.kind, EmitKind::Source);
        assert_eq!(request.out_path, None);
    }

    #[test]
    fn take_emit_request_rejects_missing_kind() {
        let mut args = vec!["script.que".to_string(), "--emit".to_string()];
        let err = take_emit_request_from_argv(&mut args).expect_err("missing kind should fail");
        assert!(err.contains("--emit requires one of: source | wat | wasm | types"));
    }

    #[test]
    fn wildcard_match_supports_star_and_question() {
        assert!(wildcard_match("*map*", "std/vector/map"));
        assert!(wildcard_match("map/?", "map/i"));
        assert!(wildcard_match("sum", "sum"));
        assert!(!wildcard_match("map/?", "map/int"));
        assert!(!wildcard_match("reduce/*/i", "reduce/i"));
    }

    #[test]
    fn lambda_execute_options_parse_get_version_and_params() {
        let args = vec![
            "--get".to_string(),
            "--input".to_string(),
            "hello".to_string(),
            "--version".to_string(),
            "18".to_string(),
            "--param".to_string(),
            "source=github".to_string(),
            "--param".to_string(),
            "status=200".to_string()
        ];
        let options = parse_lambda_execute_options(&args).expect(
            "lambda execute opts should parse"
        );
        assert_eq!(options.method, Method::GET);
        assert_eq!(options.input, "hello");
        assert_eq!(options.version.as_deref(), Some("18"));
        assert_eq!(
            options.params,
            vec![
                ("source".to_string(), "github".to_string()),
                ("status".to_string(), "200".to_string())
            ]
        );
    }

    #[test]
    fn lambda_execute_options_reject_bad_param_shape() {
        let args = vec!["--param".to_string(), "missing-separator".to_string()];
        let err = parse_lambda_execute_options(&args).expect_err("bad param should fail");
        assert!(err.contains("expected key=value"));
    }

    #[test]
    fn native_help_mentions_lambda_commands() {
        let help = native_shell_help("que");
        assert!(help.contains("que lambda <command>"));
        assert!(help.contains("https://lambda.quest"));
        assert!(help.contains("que init"));
        assert!(help.contains("que.toml"));
        assert!(!help.contains("--deps"));
    }

    #[test]
    fn validate_lambda_program_accepts_char_output() {
        validate_lambda_program_source("\"ok\"").expect(
            "string result should be valid lambda output"
        );
    }

    #[test]
    fn validate_lambda_program_rejects_non_char_output() {
        let err = validate_lambda_program_source("42").expect_err(
            "non-string result should be rejected"
        );
        assert!(err.contains("must evaluate to [Char]"));
        assert!(err.contains("Int"));
    }

    #[test]
    fn lambda_example_options_parse_default_input() {
        let options = parse_lambda_example_options(&[]).expect("example opts should parse");
        assert_eq!(options.input, "hello");
        assert_eq!(options.version, None);
        assert!(options.params.is_empty());
    }

    #[test]
    fn lambda_example_url_encodes_input_and_params() {
        let url = lambda_example_url(
            "pub_123",
            "5 6",
            Some("2"),
            &[("source".to_string(), "github api".to_string())]
        ).expect("example url should build");
        assert_eq!(
            url,
            "https://lambda.quest/v1/public/pub_123/execute?source=github+api&input=5+6&version=2"
        );
    }

    #[test]
    fn format_lambda_functions_list_renders_compact_summary() {
        let raw =
            r#"{"functions":[{"function_id":"fn_abc","function_name":"tic-tac-toe","visibility":"public","public_id":"pub_abc","created_at":"2026-04-19T19:32:23.452Z","updated_at":"2026-04-19T19:55:31.231Z","latest_version":3,"latest_compile_status":"ready","latest_compile_error":null}]}"#;
        let rendered = format_lambda_functions_list(raw).expect(
            "function list should render from json"
        );
        assert!(rendered.contains("tic-tac-toe"));
        assert!(rendered.contains("fn: fn_abc"));
        assert!(rendered.contains("public: pub_abc (public)"));
        assert!(rendered.contains("latest: v3 ready"));
        assert!(!rendered.contains("example:"));
    }

    #[test]
    fn parse_lambda_invoke_target_supports_owner_and_plain_name() {
        assert_eq!(
            parse_lambda_invoke_target("anthony/power").expect("owner/name should parse"),
            LambdaInvokeTarget::PublicByName {
                owner_label: "anthony".to_string(),
                function_name: "power".to_string(),
            }
        );
        assert_eq!(
            parse_lambda_invoke_target("power").expect("plain name should parse"),
            LambdaInvokeTarget::OwnedName("power".to_string())
        );
    }

    #[test]
    fn parse_lambda_invoke_target_rejects_ids() {
        let err = parse_lambda_invoke_target("fn_abc").expect_err("ids should be rejected");
        assert!(err.contains("name-based"));
    }

    #[test]
    fn resolve_owned_function_id_from_functions_list_finds_unique_name() {
        let raw =
            r#"{"functions":[{"function_id":"fn_power","function_name":"power"},{"function_id":"fn_range","function_name":"range"}]}"#;
        let function_id = resolve_owned_function_id_from_functions_list(raw, "power").expect(
            "function should resolve"
        );
        assert_eq!(function_id, "fn_power");
    }

    #[test]
    fn resolve_owned_function_id_from_functions_list_rejects_duplicates() {
        let raw =
            r#"{"functions":[{"function_id":"fn_power_1","function_name":"power"},{"function_id":"fn_power_2","function_name":"power"}]}"#;
        let err = resolve_owned_function_id_from_functions_list(raw, "power").expect_err(
            "duplicate names should fail"
        );
        assert!(err.contains("multiple functions named 'power'"));
    }

    #[test]
    fn lambda_api_base_defaults_to_lambda_quest() {
        assert_eq!(lambda_api_base_from_env_value(None), "https://lambda.quest");
    }

    #[test]
    fn lambda_api_base_honors_override() {
        assert_eq!(
            lambda_api_base_from_env_value(Some("https://lambda2.quest".to_string())),
            "https://lambda2.quest"
        );
    }

    #[test]
    fn init_project_config_file_writes_default_and_rejects_existing() {
        let base = std::env::temp_dir().join(format!(
            "que-init-project-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock should be after epoch")
                .as_nanos()
        ));
        std::fs::create_dir_all(&base).expect("temp dir should be created");

        let path = init_project_config_file(&base).expect("config should be created");
        let text = std::fs::read_to_string(&path).expect("config should be readable");
        assert!(text.contains("entry = \"main.que\""));
        let err = init_project_config_file(&base).expect_err("second init should fail");
        assert!(err.contains("already exists"));
    }

    #[test]
    fn resolve_project_entry_path_uses_config_entry_when_script_is_omitted() {
        let base = std::env::temp_dir().join(format!(
            "que-entry-project-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock should be after epoch")
                .as_nanos()
        ));
        std::fs::create_dir_all(&base).expect("temp dir should be created");
        std::fs::write(
            base.join(crate::project::PROJECT_CONFIG_FILE),
            "entry = \"main.que\"\ndeps = []\n",
        )
        .expect("config should be written");
        std::fs::write(base.join("main.que"), "(+ 1 2)").expect("entry should be written");

        let (_path, program, root) =
            resolve_project_entry_path(&base, None).expect("entry should resolve from config");
        assert_eq!(program.trim(), "(+ 1 2)");
        assert_eq!(
            std::fs::canonicalize(&root).expect("root should canonicalize"),
            std::fs::canonicalize(&base).expect("base should canonicalize")
        );
    }

}

pub fn run_native_shell() -> Result<(), String> {
    let args: Vec<String> = env::args().collect();
    let bin_name = args
        .first()
        .and_then(|p| Path::new(p).file_name())
        .and_then(|p| p.to_str())
        .unwrap_or("queio");
    if matches!(args.get(1).map(String::as_str), Some("--help" | "-h")) {
        println!("{}", native_shell_help(bin_name));
        return Ok(());
    }
    if matches!(args.get(1).map(String::as_str), Some("init")) {
        let cwd = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let path = init_project_config_file(&cwd)?;
        println!("wrote {}", path.display());
        return Ok(());
    }
    if matches!(args.get(1).map(String::as_str), Some("lambda")) {
        run_lambda_cli(bin_name, &args.iter().skip(2).cloned().collect::<Vec<_>>())?;
        return Ok(());
    }
    if matches!(args.get(1).map(String::as_str), Some("--learn")) {
        println!("{}", native_shell_learn());
        return Ok(());
    }
    if matches!(args.get(1).map(String::as_str), Some("--env")) {
        println!("{}", native_shell_env_help(bin_name));
        return Ok(());
    }
    if matches!(args.get(1).map(String::as_str), Some("--lib")) {
        run_library_explore_via_io(&args.iter().skip(2).cloned().collect::<Vec<_>>())?;
        return Ok(());
    }
    if matches!(args.get(1).map(String::as_str), Some("--install" | "--bake")) {
        run_library_install_via_io(
            LibraryInstallMode::Install,
            &args.iter().skip(2).cloned().collect::<Vec<_>>()
        )?;
        return Ok(());
    }
    if matches!(args.get(1).map(String::as_str), Some("--uninstall")) {
        run_library_install_via_io(
            LibraryInstallMode::Uninstall,
            &args.iter().skip(2).cloned().collect::<Vec<_>>()
        )?;
        return Ok(());
    }
    let eval_mode = matches!(args.get(1).map(String::as_str), Some("--eval" | "-e"));
    let (program, mut argv, script_cwd) = if eval_mode {
        let Some(source) = args.get(2) else {
            return Err(format!("missing source after --eval\n{}", native_shell_help(bin_name)));
        };
        let cwd = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        (source.clone(), args.iter().skip(3).cloned().collect::<Vec<_>>(), cwd)
    } else {
        let cwd = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let explicit_path = args
            .get(1)
            .filter(|token| !token.starts_with("--"))
            .map(String::as_str);
        let (_script_path, program, script_cwd) = resolve_project_entry_path(&cwd, explicit_path)
            .map_err(|message| format!("{}\n{}", message, native_shell_help(bin_name)))?;
        let argv_start = if explicit_path.is_some() { 2 } else { 1 };
        (program, args.iter().skip(argv_start).cloned().collect::<Vec<_>>(), script_cwd)
    };

    if take_help_flag_from_argv(&mut argv) {
        println!("{}", native_shell_help(bin_name));
        return Ok(());
    }
    let suppress_result_output = take_no_result_flag_from_argv(&mut argv);
    let emit_request = take_emit_request_from_argv(&mut argv)?;
    apply_project_env_vars(&script_cwd)?;
    let debug_mode = crate::io::take_debug_mode_from_argv(&mut argv);
    if debug_mode.is_enabled() {
        enable_debug_runtime_guards();
    }
    let shell_policy = crate::io
        ::take_shell_policy_from_argv(&mut argv)
        .map_err(|e| format!("invalid shell policy: {}", e))?;
    let analysis_source = crate::lsp_native_core::strip_comment_bodies_preserve_newlines(&program);
    let needs_user_form_count =
        debug_mode.is_enabled() ||
        matches!(
            emit_request.as_ref().map(|req| req.kind),
            Some(EmitKind::Types)
        );
    let user_form_count = if needs_user_form_count {
        crate::lsp_native_core
            ::parse_user_exprs_for_symbol_collection(&analysis_source)
            .as_ref()
            .map(|exprs| exprs.len())
            .unwrap_or_else(|| crate::lsp_native_core::top_level_form_ranges(&program).len())
    } else {
        0
    };
    let user_desugared = if debug_mode.includes_code() {
        crate::parser::build(&program).ok()
    } else {
        None
    };

    let std_ast = crate::baked::load_ast();
    let mut lib_defs = crate::baked::ast_to_definitions(std_ast, "active library")?;
    crate::externals::extend_with_builtin_host_externs(&mut lib_defs)?;
    lib_defs.extend(load_project_library_definitions(&script_cwd)?);
    let wrapped_ast = match crate::parser::merge_std_and_program(&program, lib_defs) {
        Ok(expr) => expr,
        Err(message) => {
            if debug_mode.is_enabled() {
                return Err(
                    build_debug_error_report(
                        debug_mode,
                        "parse+desugar",
                        &program,
                        &message,
                        None,
                        user_desugared.as_ref(),
                        user_form_count,
                        None
                    )
                );
            }
            return Err(message);
        }
    };

    if let Some(request) = emit_request.as_ref() {
        match request.kind {
            EmitKind::Source => {
                emit_text_output(request.out_path.as_deref(), &wrapped_ast.to_lisp())?;
                return Ok(());
            }
            EmitKind::Types => {
                let (base_env, base_next_id) = crate::types::create_builtin_environment(
                    crate::types::TypeEnv::new()
                );
                let inferred = crate::infer::infer_with_builtins_typed_lsp(
                    &wrapped_ast,
                    (base_env, base_next_id),
                    user_form_count
                );
                let (_typ, typed_ast) = match inferred {
                    Ok(ok) => ok,
                    Err(InferErrorInfo { message, scope, partial_typed_ast }) => {
                        if debug_mode.is_enabled() {
                            return Err(
                                build_debug_error_report(
                                    debug_mode,
                                    "type-inference",
                                    &program,
                                    &message,
                                    scope.as_ref(),
                                    user_desugared.as_ref(),
                                    user_form_count,
                                    partial_typed_ast.as_ref()
                                )
                            );
                        }
                        return Err(message);
                    }
                };
                let rendered = format_top_level_type_lines(&typed_ast, user_form_count);
                emit_text_output(request.out_path.as_deref(), &rendered)?;
                return Ok(());
            }
            EmitKind::Wat | EmitKind::Wasm => {}
        }
    }

    let wat_src = if debug_mode.is_enabled() {
        let (base_env, base_next_id) = crate::types::create_builtin_environment(
            crate::types::TypeEnv::new()
        );
        let inferred = crate::infer::infer_with_builtins_typed_lsp(
            &wrapped_ast,
            (base_env, base_next_id),
            user_form_count
        );

        match inferred {
            Ok((_typ, typed_ast)) => {
                crate::wat
                    ::compile_program_to_wat_typed(&typed_ast)
                    .map_err(|message| {
                        build_debug_error_report(
                            debug_mode,
                            "wat-lowering",
                            &program,
                            &message,
                            None,
                            user_desugared.as_ref(),
                            user_form_count,
                            Some(&typed_ast)
                        )
                    })?
            }
            Err(InferErrorInfo { message, scope, partial_typed_ast }) => {
                return Err(
                    build_debug_error_report(
                        debug_mode,
                        "type-inference",
                        &program,
                        &message,
                        scope.as_ref(),
                        user_desugared.as_ref(),
                        user_form_count,
                        partial_typed_ast.as_ref()
                    )
                );
            }
        }
    } else {
        crate::wat::compile_program_to_wat(&wrapped_ast)?
    };

    if let Some(request) = emit_request.as_ref() {
        match request.kind {
            EmitKind::Wat => {
                emit_text_output(request.out_path.as_deref(), &wat_src)?;
                return Ok(());
            }
            EmitKind::Wasm => {
                let bytes = wat
                    ::parse_str(&wat_src)
                    .map_err(|e| format!("failed to encode wat as wasm: {}", e))?;
                emit_bytes_output(request.out_path.as_deref(), &bytes)?;
                return Ok(());
            }
            EmitKind::Source | EmitKind::Types => unreachable!("handled earlier"),
        }
    }

    let store_data = ShellStoreData::new_with_security(Some(script_cwd), shell_policy).map_err(|e|
        e.to_string()
    )?;
    if suppress_result_output {
        crate::runtime::run_wat_text_no_result(&wat_src, store_data, &argv, |linker| {
            add_shell_to_linker(linker).map_err(|e| e.to_string())
        })?;
    } else {
        let decoded = crate::runtime::run_wat_text(&wat_src, store_data, &argv, |linker| {
            add_shell_to_linker(linker).map_err(|e| e.to_string())
        })?;
        println!("\x1b[32m{}\x1b[0m", decoded);
    }

    Ok(())
}
