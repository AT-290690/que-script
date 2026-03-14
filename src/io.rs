use crate::infer::{ infer_with_builtins_typed, InferErrorInfo, InferErrorScope, TypedExpression };
use crate::lsp_native_core::{
    diagnostic_summary_without_snippet,
    extract_error_snippet,
    infer_error_ranges,
    normalize_signature,
};
use crate::parser::Expression;
use std::collections::{ BTreeMap, HashSet };
use std::env;
use std::fs;
use std::io;
use std::io::Write as _;
use std::path::{ Path, PathBuf };
use std::thread;
use std::time::Duration;
use wasmtime::{ Caller, Extern, Memory, TypedFunc };
use wasmtime::Linker;
use wasmtime_wasi::{ ResourceTable, WasiCtx, WasiCtxBuilder, WasiCtxView, WasiView };

const VEC_LEN_OFFSET: i32 = 0;
const VEC_CAP_OFFSET: i32 = 4;
const VEC_RC_OFFSET: i32 = 8;
const VEC_ELEM_REF_OFFSET: i32 = 12;
const VEC_DATA_PTR_OFFSET: i32 = 16;
const VEC_HEADER_SIZE: i32 = 20;

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

fn parse_bundle_definitions(source: &str, label: &str) -> Result<Vec<Expression>, String> {
    let root = crate::parser
        ::build(source)
        .map_err(|e| format!("failed to parse bundle '{}': {}", label, e))?;
    let defs = crate::baked::ast_to_definitions(root, label)?;
    for (idx, item) in defs.iter().enumerate() {
        let Expression::Apply(form) = item else {
            return Err(
                format!(
                    "bundle '{}' must contain only top-level definitions; found non-definition at form {}: {}",
                    label,
                    idx,
                    item.to_lisp()
                )
            );
        };
        if form.len() < 3 {
            return Err(
                format!(
                    "bundle '{}' must contain only top-level definitions; malformed form {}: {}",
                    label,
                    idx,
                    item.to_lisp()
                )
            );
        }
        let Expression::Word(kw) = &form[0] else {
            return Err(
                format!(
                    "bundle '{}' must contain only top-level definitions; malformed form {}: {}",
                    label,
                    idx,
                    item.to_lisp()
                )
            );
        };
        if kw != "let" && kw != "let*" && kw != "mut" {
            return Err(
                format!(
                    "bundle '{}' must contain only top-level definitions; found '{}' at form {}",
                    label,
                    kw,
                    idx
                )
            );
        }
    }
    Ok(defs)
}

fn load_bundle_definitions(
    script_cwd: &Path,
    bundle_paths: &[String]
) -> Result<Vec<Expression>, String> {
    let mut out = Vec::new();
    for bundle_path in bundle_paths {
        let raw = Path::new(bundle_path);
        let resolved = if raw.is_absolute() { raw.to_path_buf() } else { script_cwd.join(raw) };
        if resolved.extension().and_then(|e| e.to_str()) != Some("que") {
            return Err(format!("bundle '{}' must be a .que file", resolved.display()));
        }
        let source = fs
            ::read_to_string(&resolved)
            .map_err(|e| format!("failed to read bundle '{}': {}", resolved.display(), e))?;
        let mut defs = parse_bundle_definitions(&source, &resolved.display().to_string())?;
        out.append(&mut defs);
    }
    Ok(out)
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

fn native_shell_help(bin_name: &str) -> String {
    format!(
        "Usage: {bin} <script.que> [arg ...] [--debug [basic|code|types|all]] [--allow <read|write|delete|all> [...]]\n\
         or:    {bin} --eval <source> [arg ...] [--debug [basic|code|types|all]] [--allow <read|write|delete|all> [...]]\n\
         or:    {bin} --install [helpers.que ...] [--out <que-lib.lisp>]\n\
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
           --debug        Enable compiler/runtime debug report on errors (default: basic locations).\n\
                         Also forces QUE_INT_OVERFLOW_CHECK, QUE_FLOAT_OVERFLOW_CHECK,\n\
                         QUE_DIV_ZERO_CHECK, and QUE_BOUNDS_CHECK to ON for this run.\n\
           --no-result    Do not print/decode the final evaluated program value.\n\
           --allow        Enable host io permissions (read, write, delete, all).\n\
         \n\
         Notes:\n\
           - Recommended: run with `--debug` for stronger safety checks and richer diagnostics.\n\
           - Script arguments come before --allow.\n\
           - `--install` accepts helper .que files as positional arguments.\n\
           - `--lib names [pattern]` lists available library names.\n\
           - `--lib types [pattern]` prints name and inferred type.\n\
           - `--lib source <name>` prints the exact symbol source.\n\
           - Inline eval example: `{bin} --eval '(+ 1 2)'`.\n\
           - Wildcards in pattern: `*` any sequence, `?` single char.\n\
           - --debug, --no-result and --help can appear after the script path.\n\
           - `--install` writes/extends an external library file (used by all binaries).\n\
           - `--uninstall` removes the active external library file.\n\
           - Default output path: /usr/local/share/que/que-lib.lisp.\n\
           - In installed setups, without an external library file only language builtins are available.\n\
           - After install/uninstall, restart editor/LSP to reload library state.\n\
           - Once installed, helper bundle source files can be removed.\n\
         ",
        bin = bin_name
    )
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
           QUE_DIV_ZERO_CHECK Division/modulo by zero trap check (default: off). Enable with 1|true|on|yes.\n\
           QUE_INT_OVERFLOW_CHECK   Integer overflow trap check for +,-,* and mut ops (default: off).\n\
           QUE_FLOAT_OVERFLOW_CHECK Float NaN/Inf trap check for +.,-.,*.,/. and mut ops (default: off).\n\
         \n\
         Example:\n\
           QUE_WASM_OPT=speed QUE_DEVIRTUALIZE=aggressive QUE_TCO=conservative QUE_BOUNDS_CHECK=0 QUE_VEC_MIN_CAP=8 QUE_VEC_GROWTH_NUM=3 QUE_VEC_GROWTH_DEN=2 {bin} script.que\n\
         \n\
         Setup some env flags:\n\
         \n\
           export QUE_WASM_OPT=speed QUE_TCO=aggressive QUE_DEVIRTUALIZE=aggressive QUE_BOUNDS_CHECK=0 QUE_VEC_MIN_CAP=8 QUE_VEC_GROWTH_NUM=2 QUE_VEC_GROWTH_DEN=1\n\
         \n\
         Fallback to default ones:\n\
         \n\
           unset QUE_WASM_OPT QUE_TCO QUE_DEVIRTUALIZE QUE_BOUNDS_CHECK QUE_VEC_MIN_CAP QUE_VEC_GROWTH_NUM QUE_VEC_GROWTH_DEN",
        bin = bin_name
    )
}

fn native_shell_learn() -> &'static str {
    "Que is a functional, expression-only Lisp with S-expressions.\n\
    \n\
    Core:\n\
    - Function call: (f a b)\n\
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
    - Recursive functions must use let*: (let* f (lambda ... (f ...)))\n\
    - Destructuring works in params:\n\
      - tuples: {a b}\n\
      - vectors: [a b c]\n\
      - '.' skips/ignores rest; for vectors rest marker is last.\n\
    \n\
    Types:\n\
    - Int, Float, Bool, Char\n\
    - Vector [T] (homogeneous)\n\
    - Tuple {A B}\n\
    - String is [Char]\n\
    - Equality example: = (Int), =. (Float), =# (Char), =? (Bool)\n\
    - Operator suffixes: '.' for Float, '#' for Char, '?' for Bool.\n\
    - String literal uses double quotes, e.g. \"Hello World\".\n\
    - Char literal uses single quotes, e.g. 'a'.\n\
    \n\
    Pipe operators:\n\
    - `(|> x f g h)` means `(h (g (f x)))` (left-to-right flow with data last)\n\
    \n\
    Mutation and effects:\n\
    - mut/alter! are for local primitive scalar mutation only (Int/Float/Bool/Char), same lambda scope.\n\
    - &mut/&alter! are for shared mutation across lambda scopes via boxed references.\n\
    - Vector/state mutation uses set!, push!, pop!.\n\
    - Functions with side effects (mutation or I/O) must end with !.\n\
    - If a function mutates args, the mutated arg must be the first arg.\n\
    - If mutating multiple values, pass them inside the first arg (typically a tuple).\n\
    \n\
    Built-ins:\n\
    - set! pop! length get car cdr fst snd while\n\
    + - * / mod = < > <= >= +. -. *. /. mod. =. <. >. <=. >=. +# -# *# /# =# =?\n\
    and or not & | ^ >> << ~ Int->Float Float->Int true false nil\n\
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
    if keyword != "let" && keyword != "let*" && keyword != "mut" {
        return None;
    }
    let Expression::Word(name) = &items[1] else {
        return None;
    };
    Some(name.clone())
}

fn active_library_definitions() -> Result<Vec<Expression>, String> {
    crate::baked::ast_to_definitions(crate::baked::load_ast(), "active")
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
    defs.extend(load_bundle_definitions(&cwd, &bundle_paths)?);
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
        .map_err(|e| format!("failed to write baked library '{}': {}", output.display(), e))?;
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
        .map_err(|e|
            wasmtime::Error::msg(format!("failed to read '{}': {}", target.display(), e))
        )?;
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
        .map_err(|e|
            wasmtime::Error::msg(format!("failed to mkdir '{}': {}", target.display(), e))
        )?;
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
        .map_err(|e|
            wasmtime::Error::msg(
                format!("failed to inspect path '{}' for delete: {}", target.display(), e)
            )
        )?;
    if meta.is_dir() {
        fs
            ::remove_dir_all(&target)
            .map_err(|e|
                wasmtime::Error::msg(
                    format!("failed to delete directory '{}': {}", target.display(), e)
                )
            )?;
    } else {
        fs
            ::remove_file(&target)
            .map_err(|e|
                wasmtime::Error::msg(format!("failed to delete file '{}': {}", target.display(), e))
            )?;
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
                .map_err(|e|
                    wasmtime::Error::msg(
                        format!("failed to create destination dirs '{}': {}", parent.display(), e)
                    )
                )?;
        }
    }
    fs
        ::rename(&src_path, &dst_path)
        .map_err(|e|
            wasmtime::Error::msg(
                format!(
                    "failed to move '{}' to '{}': {}",
                    src_path.display(),
                    dst_path.display(),
                    e
                )
            )
        )?;

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
        Expression::Float(v) => format!("{:?}", v),
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
        DebugMode,
        wildcard_match,
        parse_bundle_definitions,
        take_debug_mode_from_argv,
        take_help_flag_from_argv,
        take_no_result_flag_from_argv,
        take_shell_policy_from_argv,
        ShellPermission,
        ShellPolicy,
    };
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
    fn parse_bundle_definitions_accepts_top_level_defs_only() {
        let src = "(let inc (lambda x (+ x 1)))\n(let dec (lambda x (- x 1)))";
        let defs = parse_bundle_definitions(src, "bundle.que").expect("bundle should parse");
        assert_eq!(defs.len(), 2);
    }

    #[test]
    fn parse_bundle_definitions_accepts_nested_top_level_do() {
        let src = "(do (let inc (lambda x (+ x 1))) (let dec (lambda x (- x 1))))";
        let defs = parse_bundle_definitions(src, "bundle.que").expect("bundle should parse");
        assert_eq!(defs.len(), 2);
    }

    #[test]
    fn parse_bundle_definitions_rejects_non_definition_form() {
        let src = "(let inc (lambda x (+ x 1)))\n(inc 1)";
        let err = parse_bundle_definitions(src, "bundle.que").expect_err("bundle should fail");
        assert!(err.contains("must contain only top-level definitions"));
    }

    #[test]
    fn wildcard_match_supports_star_and_question() {
        assert!(wildcard_match("*map*", "std/vector/map"));
        assert!(wildcard_match("map/?", "map/i"));
        assert!(wildcard_match("sum", "sum"));
        assert!(!wildcard_match("map/?", "map/int"));
        assert!(!wildcard_match("reduce/*/i", "reduce/i"));
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
        let Some(file_path) = args.get(1) else {
            return Err(format!("missing file_path\n{}", native_shell_help(bin_name)));
        };
        let program = fs
            ::read_to_string(&file_path)
            .map_err(|e| format!("failed to read '{}': {}", file_path, e))?;
        let script_cwd = fs
            ::canonicalize(file_path)
            .ok()
            .and_then(|path| path.parent().map(Path::to_path_buf))
            .or_else(|| Path::new(file_path).parent().map(Path::to_path_buf))
            .filter(|path| !path.as_os_str().is_empty())
            .unwrap_or_else(|| PathBuf::from("."));
        (program, args.iter().skip(2).cloned().collect::<Vec<_>>(), script_cwd)
    };

    if take_help_flag_from_argv(&mut argv) {
        println!("{}", native_shell_help(bin_name));
        return Ok(());
    }
    let suppress_result_output = take_no_result_flag_from_argv(&mut argv);
    let debug_mode = crate::io::take_debug_mode_from_argv(&mut argv);
    if debug_mode.is_enabled() {
        enable_debug_runtime_guards();
    }
    let shell_policy = crate::io
        ::take_shell_policy_from_argv(&mut argv)
        .map_err(|e| format!("invalid shell policy: {}", e))?;
    let analysis_source = crate::lsp_native_core::strip_comment_bodies_preserve_newlines(&program);
    let user_form_count = if debug_mode.is_enabled() {
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
    let lib_defs = crate::baked::ast_to_definitions(std_ast, "active library")?;
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
            Ok((_typ, typed_ast)) =>
                crate::wat
                    ::compile_program_to_wat_typed(&typed_ast)
                    .map_err(|message|
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
                    )?,
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
    let store_data = ShellStoreData::new_with_security(Some(script_cwd), shell_policy).map_err(|e|
        e.to_string()
    )?;
    if suppress_result_output {
        crate::runtime::run_wat_text_no_result(&wat_src, store_data, &argv, |linker|
            add_shell_to_linker(linker).map_err(|e| e.to_string())
        )?;
    } else {
        let decoded = crate::runtime::run_wat_text(&wat_src, store_data, &argv, |linker|
            add_shell_to_linker(linker).map_err(|e| e.to_string())
        )?;
        println!("\x1b[32m{}\x1b[0m", decoded);
    }

    Ok(())
}
