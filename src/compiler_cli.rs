//! Compiler and runner commands used by the unified `que` CLI.
//!
//! This module deliberately has no dependency on the embedded Wasmtime host.

use crate::infer::{infer_with_builtins_typed_lsp, InferErrorInfo, TypedExpression};
use crate::parser::Expression;
use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::io::{Read, Write};
use std::path::Path;
use std::process::{Command, Output};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum WasmRuntime {
    Wasmtime,
    Wasmer,
    Wamr,
}

impl WasmRuntime {
    fn parse(value: &str) -> Result<(Self, String), String> {
        let executable = match value {
            "wamr" => "iwasm".to_string(),
            other => other.to_string(),
        };
        let name = Path::new(&executable)
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or(value)
            .strip_suffix(".exe")
            .unwrap_or_else(|| {
                Path::new(&executable)
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or(value)
            });
        let adapter = match name {
            "wasmtime" => Self::Wasmtime,
            "wasmer" => Self::Wasmer,
            "wamr" | "iwasm" => Self::Wamr,
            _ => {
                return Err(format!(
                    "unsupported WebAssembly runtime '{value}'; use wasmtime, wasmer, iwasm/wamr, or a path to one of those executables"
                ))
            }
        };
        Ok((adapter, executable))
    }

    fn command(
        self,
        executable: String,
        wasm: &Path,
        program_args: &[String],
        grants_filesystem: bool,
    ) -> Command {
        let mut command = Command::new(executable);
        match self {
            Self::Wasmtime => {
                command.arg("run");
                if grants_filesystem {
                    command.arg("--dir").arg(".");
                }
                command.arg(wasm).args(program_args);
            }
            Self::Wasmer => {
                command.arg("run").arg(wasm);
                if grants_filesystem {
                    command.arg("--volume").arg(".:.");
                }
                if !program_args.is_empty() {
                    command.arg("--").args(program_args);
                }
            }
            Self::Wamr => {
                if grants_filesystem {
                    command.arg("--dir=.");
                }
                command.arg(wasm).args(program_args);
            }
        }
        command
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum EmitKind {
    Source,
    OptSource,
    Wat,
    Wasm,
    Types,
}

fn help() -> &'static str {
    "que — Que compiler, tooling, and runner

Usage:
  que <program.que> [arguments ...] [--opt] [--runtime <runtime>]
  que compile <program.que> [--opt] [--out <program.wasm>]
  que run-wasi <program.que> [arguments ...] [--opt] [--runtime <runtime>]
  que wat <program.que>
  que wat --eval <source>
  que --eval <source> [arguments ...] [--opt|--debug]
  que --lib <names|types|source> [pattern|name]
  que --env
  que <program.que> --emit <source|opt-source|wat|wasm|types> [--out <file>]
  que explain <program.que> [--json] [--opt] [--out <file>]
  que fmt <program.que> [--check|--stdout]
  que fmt --stdin

`run-wasi` uses a user-installed runtime: wasmtime (default), wasmer, or
iwasm/WAMR. Select it with --runtime or QUE_WASM_RUNTIME. Native-C tooling is
kept separately in the repository's scripts directory."
}

pub fn run() -> Result<(), String> {
    run_with_args(env::args().skip(1).collect())
}

pub fn run_with_args(mut args: Vec<String>) -> Result<(), String> {
    if args.is_empty() || matches!(args[0].as_str(), "--help" | "-h") {
        println!("{}", help());
        return Ok(());
    }
    if matches!(args[0].as_str(), "--version" | "-V") {
        println!("que {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }
    if matches!(args[0].as_str(), "--eval" | "-e") {
        return run_eval(args);
    }
    if args[0] == "--env" {
        println!("{}", environment_help());
        return Ok(());
    }
    if args[0] == "--lib" {
        return run_library_explore(&args[1..]);
    }
    match args[0].as_str() {
        "fmt" => return run_fmt(&args[1..]),
        "explain" => return run_explain(&args[1..]),
        "compile" => {
            args.remove(0);
            return run_compile(args, Some(EmitKind::Wasm));
        }
        "run-wasi" => {
            args.remove(0);
            return run_wasi(args);
        }
        "wat" => {
            args.remove(0);
            return run_wat(args);
        }
        _ => {}
    }
    if args.iter().any(|arg| arg == "--emit") {
        run_compile(args, None)
    } else {
        run_compile(args, Some(EmitKind::Wasm))
    }
}

fn run_eval(mut args: Vec<String>) -> Result<(), String> {
    if args.len() < 2 {
        return Err("--eval requires source text".into());
    }
    args.remove(0);
    let source = args.remove(0);
    let path = env::temp_dir().join(format!("que-eval-{}.que", std::process::id()));
    fs::write(&path, source).map_err(|error| format!("failed to stage --eval source: {error}"))?;
    args.insert(0, path.to_string_lossy().into_owned());
    let previous_label = env::var_os("QUE_INTERNAL_SOURCE_LABEL");
    env::set_var("QUE_INTERNAL_SOURCE_LABEL", "--eval source");
    let result = if args.iter().any(|arg| arg == "--emit") {
        run_compile(args, None)
    } else {
        run_wasi(args)
    };
    if let Some(label) = previous_label {
        env::set_var("QUE_INTERNAL_SOURCE_LABEL", label);
    } else {
        env::remove_var("QUE_INTERNAL_SOURCE_LABEL");
    }
    let _ = fs::remove_file(&path);
    let staged_path = path.to_string_lossy().into_owned();
    result.map_err(|error| error.replace(&staged_path, "--eval source"))
}

fn environment_help() -> &'static str {
    "Environment:
  QUE_WASM_RUNTIME       Runtime adapter/executable (wasmtime, wasmer, iwasm/WAMR).
  QUE_LIB_PATH           Override the baked library path.
  QUE_WASM_OPT           Optimization level: none, speed, or speed_and_size.
  QUE_DEVIRTUALIZE       Call devirtualization: off, known-heads, or aggressive.
  QUE_TCO                Tail-call optimization: off, conservative, or aggressive.
  QUE_SMALL_SCALAR_INLINE_COST  Scalar helper inline budget.
  QUE_LOOP_UNROLL_MAX    Maximum constant loop trip count to unroll.
  QUE_LOOP_UNROLL_COST   Maximum loop unroll cost.
  QUE_BOUNDS_CHECK       Runtime vector bounds checks.
  QUE_STATIC_BOUNDS      Static correctness analysis.
  QUE_INT_OVERFLOW_CHECK Runtime Int overflow checks.
  QUE_DEC_OVERFLOW_CHECK Runtime Dec overflow checks.
  QUE_DIV_ZERO_CHECK     Runtime division/modulo-zero checks.
  QUE_DECIMAL_SCALE      Dec fixed-point scale.
  QUE_VEC_MIN_CAP        Minimum vector capacity.
  QUE_VEC_GROWTH_NUM     Vector growth numerator.
  QUE_VEC_GROWTH_DEN     Vector growth denominator."
}

#[derive(Clone)]
enum LibrarySymbol {
    Source(Expression),
    Macro(Expression),
    Builtin(Option<String>, &'static str),
}

fn binding_name(expr: &Expression) -> Option<String> {
    let Expression::Apply(items) = expr else {
        return None;
    };
    if items.len() < 3
        || !matches!(&items[0], Expression::Word(word) if matches!(word.as_str(), "let" | "letrec" | "letmacro" | "mut"))
    {
        return None;
    }
    match &items[1] {
        Expression::Word(name) => Some(name.clone()),
        _ => None,
    }
}

fn wildcard_match(pattern: &str, text: &str) -> bool {
    let (pattern, text) = (pattern.as_bytes(), text.as_bytes());
    let mut matches = vec![vec![false; text.len() + 1]; pattern.len() + 1];
    matches[0][0] = true;
    for i in 1..=pattern.len() {
        if pattern[i - 1] == b'*' {
            matches[i][0] = matches[i - 1][0];
        }
        for j in 1..=text.len() {
            matches[i][j] = match pattern[i - 1] {
                b'*' => matches[i - 1][j] || matches[i][j - 1],
                b'?' => matches[i - 1][j - 1],
                byte => byte == text[j - 1] && matches[i - 1][j - 1],
            };
        }
    }
    matches[pattern.len()][text.len()]
}

fn library_symbols() -> Result<(BTreeMap<String, LibrarySymbol>, Vec<Expression>), String> {
    let mut definitions =
        crate::baked::ast_to_definitions(crate::baked::load_ast(), "active library")?;
    crate::externals::extend_with_builtin_host_externs(&mut definitions)?;
    let (environment, _) = crate::types::create_builtin_environment(crate::types::TypeEnv::new());
    let mut symbols = BTreeMap::new();
    if let Some(scope) = environment.scopes.first() {
        for (name, scheme) in scope {
            symbols.insert(
                name.clone(),
                LibrarySymbol::Builtin(
                    Some(crate::lsp_native_core::normalize_signature(
                        &scheme.typ.to_string(),
                    )),
                    "compiler/runtime built-in",
                ),
            );
        }
    }
    for definition in &definitions {
        let Some(name) = binding_name(definition) else {
            continue;
        };
        if name.starts_with('_') || name.starts_with("std/") {
            continue;
        }
        let symbol = if matches!(definition, Expression::Apply(items) if matches!(items.first(), Some(Expression::Word(word)) if word == "letmacro"))
        {
            LibrarySymbol::Macro(definition.clone())
        } else {
            LibrarySymbol::Source(definition.clone())
        };
        symbols.insert(name, symbol);
    }
    Ok((symbols, definitions))
}

fn run_library_explore(args: &[String]) -> Result<(), String> {
    if args.is_empty() || matches!(args[0].as_str(), "--help" | "-h") {
        println!("Usage: que --lib names [pattern]\n       que --lib types [pattern]\n       que --lib source <name>");
        return Ok(());
    }
    let (symbols, definitions) = library_symbols()?;
    match args[0].as_str() {
        "names" | "types" => {
            if args.len() > 2 {
                return Err(format!("--lib {} accepts at most one pattern", args[0]));
            }
            let pattern = args.get(1).map(String::as_str).unwrap_or("*");
            for (name, symbol) in symbols
                .iter()
                .filter(|(name, _)| wildcard_match(pattern, name))
            {
                if args[0] == "names" {
                    println!("{name}");
                    continue;
                }
                let typ = match symbol {
                    LibrarySymbol::Builtin(Some(typ), _) => typ.clone(),
                    LibrarySymbol::Builtin(None, _) => "<built-in>".into(),
                    LibrarySymbol::Macro(_) => "<macro>".into(),
                    LibrarySymbol::Source(_) => {
                        let merged =
                            crate::parser::merge_std_and_program(name, definitions.clone())?;
                        infer_program(name, &merged)?
                            .typ
                            .as_ref()
                            .map(|typ| {
                                crate::lsp_native_core::normalize_signature(&typ.to_string())
                            })
                            .unwrap_or_else(|| "_".into())
                    }
                };
                println!("{name} : {typ}");
            }
            Ok(())
        }
        "source" => {
            if args.len() != 2 {
                return Err("--lib source requires one symbol name".into());
            }
            let name = &args[1];
            let symbol = symbols
                .get(name)
                .ok_or_else(|| format!("library symbol '{name}' not found"))?;
            println!("name: {name}");
            match symbol {
                LibrarySymbol::Source(expr) => {
                    println!("kind: library\nsource:\n{}", expr.to_lisp())
                }
                LibrarySymbol::Macro(expr) => println!("kind: macro\nsource:\n{}", expr.to_lisp()),
                LibrarySymbol::Builtin(typ, description) => {
                    println!("kind: built-in");
                    if let Some(typ) = typ {
                        println!("type: {typ}");
                    }
                    println!("source:\n<{description}>");
                }
            }
            Ok(())
        }
        command => Err(format!("unknown --lib command '{command}'")),
    }
}

fn run_wat(mut args: Vec<String>) -> Result<(), String> {
    if matches!(args.first().map(String::as_str), Some("--eval" | "-e")) {
        if args.len() != 2 {
            return Err("wat --eval requires exactly one source argument".into());
        }
        let source = args.remove(1);
        let merged = merged_program(&source)?;
        let typed = infer_program(&source, &merged)?;
        let wat = crate::wat::compile_program_to_wat_typed(&typed)?;
        println!("{wat}");
        return Ok(());
    }
    args.push("--emit".into());
    args.push("wat".into());
    run_compile(args, None)
}

fn take_flag(args: &mut Vec<String>, flag: &str) -> bool {
    if let Some(index) = args.iter().position(|arg| arg == flag) {
        args.remove(index);
        true
    } else {
        false
    }
}

fn take_value(args: &mut Vec<String>, flag: &str) -> Result<Option<String>, String> {
    let Some(index) = args.iter().position(|arg| arg == flag) else {
        return Ok(None);
    };
    if index + 1 >= args.len() {
        return Err(format!("{flag} requires a value"));
    }
    let value = args.remove(index + 1);
    args.remove(index);
    Ok(Some(value))
}

fn enable_opt() {
    env::set_var("QUE_WASM_OPT", "speed");
    env::set_var("QUE_DEVIRTUALIZE", "aggressive");
    env::set_var("QUE_SMALL_SCALAR_INLINE_COST", "512");
    env::set_var("QUE_LOOP_UNROLL_MAX", "16");
    env::set_var("QUE_LOOP_UNROLL_COST", "2000");
    env::set_var("QUE_BOUNDS_CHECK", "0");
    env::set_var("QUE_INT_OVERFLOW_CHECK", "0");
    env::set_var("QUE_DEC_OVERFLOW_CHECK", "0");
    env::set_var("QUE_DIV_ZERO_CHECK", "0");
    env::set_var("QUE_OPT_PROOF_CODEGEN", "1");
}

fn enable_debug() {
    env::set_var("QUE_BOUNDS_CHECK", "1");
    env::set_var("QUE_INT_OVERFLOW_CHECK", "1");
    env::set_var("QUE_DEC_OVERFLOW_CHECK", "1");
    env::set_var("QUE_DIV_ZERO_CHECK", "1");
    env::set_var("QUEC_DEBUG_ANALYSIS", "1");
}

fn read_program(path: &str) -> Result<String, String> {
    fs::read_to_string(path).map_err(|error| format!("failed to read '{path}': {error}"))
}

fn merged_program_with_definitions(
    source: &str,
    extra_definitions: Vec<Expression>,
) -> Result<Expression, String> {
    let std_ast = crate::baked::load_ast();
    let mut definitions = crate::baked::ast_to_definitions(std_ast, "active library")?;
    crate::externals::extend_with_builtin_host_externs(&mut definitions)?;
    definitions.extend(extra_definitions);
    crate::parser::merge_std_and_program(source, definitions)
}

fn merged_program(source: &str) -> Result<Expression, String> {
    merged_program_with_definitions(source, Vec::new())
}

fn merged_project_program(path: &str, source: &str) -> Result<Expression, String> {
    let script = fs::canonicalize(path).unwrap_or_else(|_| Path::new(path).to_path_buf());
    let start = script.parent().unwrap_or_else(|| Path::new("."));
    let Some(project) = crate::project::discover_project_config(start)? else {
        return merged_program(source);
    };
    for (key, value) in &project.config.env {
        env::set_var(key, value);
    }
    let definitions =
        crate::project::load_bundle_definitions(&project.root_dir, &project.config.deps)?;
    merged_program_with_definitions(source, definitions)
}

fn user_form_count(source: &str) -> usize {
    let clean = crate::lsp_native_core::strip_comment_bodies_preserve_newlines(source);
    crate::lsp_native_core::parse_user_exprs_for_symbol_collection(&clean)
        .map(|forms| forms.len())
        .unwrap_or_else(|| crate::lsp_native_core::top_level_form_ranges(source).len())
}

fn infer_program(source: &str, merged: &Expression) -> Result<TypedExpression, String> {
    let (env, next_id) = crate::types::create_builtin_environment(crate::types::TypeEnv::new());
    infer_with_builtins_typed_lsp(merged, (env, next_id), user_form_count(source))
        .map(|(_, typed)| typed)
        .map_err(|InferErrorInfo { message, .. }| {
            crate::lsp_native_core::normalize_diagnostic_message(&message)
        })
}

fn type_lines(typed: &TypedExpression, count: usize) -> String {
    let forms = match &typed.expr {
        Expression::Apply(items) if matches!(items.first(), Some(Expression::Word(word)) if word == "do") => {
            &items[1..]
        }
        _ => &[],
    };
    let typed_forms = match &typed.children[..] {
        [body] if matches!(&body.expr, Expression::Apply(items) if matches!(items.first(), Some(Expression::Word(word)) if word == "do")) => {
            &body.children[..]
        }
        children => children,
    };
    let start = typed_forms.len().saturating_sub(count);
    let mut out = String::new();
    for (offset, form) in typed_forms[start..].iter().enumerate() {
        let label = forms
            .get(start + offset)
            .and_then(|expr| match expr {
                Expression::Apply(items)
                    if items.len() >= 2
                        && matches!(&items[0], Expression::Word(word) if word == "let") =>
                {
                    match &items[1] {
                        Expression::Word(name) => Some(name.as_str()),
                        _ => None,
                    }
                }
                _ => None,
            })
            .map(str::to_owned)
            .unwrap_or_else(|| format!("form[{offset}]"));
        let typ = form
            .typ
            .as_ref()
            .map(ToString::to_string)
            .unwrap_or_else(|| "_".into());
        out.push_str(&format!("{label} : {typ}\n"));
    }
    if let Some(typ) = typed.typ.as_ref() {
        out.push_str(&format!("result : {typ}\n"));
    }
    out
}

fn write_output(path: Option<&str>, bytes: &[u8]) -> Result<(), String> {
    if let Some(path) = path {
        fs::write(path, bytes).map_err(|error| format!("failed to write '{path}': {error}"))
    } else {
        std::io::stdout()
            .write_all(bytes)
            .map_err(|error| error.to_string())
    }
}

fn run_compile(mut args: Vec<String>, default: Option<EmitKind>) -> Result<(), String> {
    let opt = take_flag(&mut args, "--opt");
    let debug = take_flag(&mut args, "--debug");
    let wasi = take_flag(&mut args, "--wasi");
    if wasi {
        env::set_var("QUE_WASI_HOST", "1");
    }
    let out = take_value(&mut args, "--out")?;
    let emit = if let Some(value) = take_value(&mut args, "--emit")? {
        match value.as_str() {
            "source" => EmitKind::Source,
            "opt-source" => EmitKind::OptSource,
            "wat" => EmitKind::Wat,
            "wasm" => EmitKind::Wasm,
            "types" => EmitKind::Types,
            _ => return Err(format!("unknown emit kind '{value}'")),
        }
    } else {
        default.ok_or_else(|| "missing --emit kind".to_string())?
    };
    if opt {
        enable_opt();
    } else if debug {
        enable_debug();
    }
    let path = args
        .first()
        .ok_or_else(|| "missing program path".to_string())?;
    let display_path = env::var("QUE_INTERNAL_SOURCE_LABEL").unwrap_or_else(|_| path.clone());
    let source = read_program(path)?;
    let merged = merged_project_program(path, &source)
        .map_err(|error| source_error(&display_path, error))?;
    if emit == EmitKind::Source {
        return write_output(out.as_deref(), format!("{}\n", merged.to_lisp()).as_bytes());
    }
    let typed =
        infer_program(&source, &merged).map_err(|error| source_error(&display_path, error))?;
    if env::var("QUEC_DEBUG_ANALYSIS").as_deref() == Ok("1") {
        emit_debug_analysis_warnings(&display_path, &source, &typed);
    }
    if emit == EmitKind::Types {
        return write_output(
            out.as_deref(),
            type_lines(&typed, user_form_count(&source)).as_bytes(),
        );
    }
    let optimized = crate::op::optimize_typed_ast(&typed);
    if emit == EmitKind::OptSource {
        return write_output(
            out.as_deref(),
            format!("{}\n", optimized.expr.to_lisp()).as_bytes(),
        );
    }
    let wat = crate::wat::compile_program_to_wat_typed(&typed)
        .map_err(|error| source_error(&display_path, error))?;
    if emit == EmitKind::Wat {
        write_output(out.as_deref(), wat.as_bytes())
    } else {
        let wasm = wat::parse_str(&wat).map_err(|error| {
            source_error(&display_path, format!("failed to encode Wasm: {error}"))
        })?;
        write_output(out.as_deref(), &wasm)
    }
}

fn run_wasi(mut args: Vec<String>) -> Result<(), String> {
    let opt = take_flag(&mut args, "--opt");
    let debug = take_flag(&mut args, "--debug");
    let no_result = take_flag(&mut args, "--no-result");
    let runtime = take_value(&mut args, "--runtime")?
        .or_else(|| env::var("QUE_WASM_RUNTIME").ok())
        .unwrap_or_else(|| "wasmtime".to_string());
    let (runtime_adapter, runtime_executable) = WasmRuntime::parse(&runtime)?;
    env::set_var("QUE_WASI_NO_RESULT", if no_result { "1" } else { "0" });
    let permissions = if let Some(index) = args.iter().position(|arg| arg == "--allow") {
        let tail = args.drain(index..).skip(1).collect::<Vec<_>>();
        if let Some(separator) = tail.iter().position(|arg| arg == "in") {
            let permissions = tail[..separator].join(",");
            args.extend_from_slice(&tail[separator + 1..]);
            permissions
        } else {
            tail.join(",")
        }
    } else {
        String::new()
    };
    env::set_var("QUE_WASI_ALLOW", &permissions);
    let path = args
        .first()
        .cloned()
        .ok_or_else(|| "run-wasi requires a program path".to_string())?;
    args.remove(0);
    let wasm = env::temp_dir().join(format!("que-wasi-{}.wasm", std::process::id()));
    let mut compile = vec![
        path.clone(),
        "--wasi".to_string(),
        "--out".to_string(),
        wasm.to_string_lossy().into_owned(),
    ];
    if opt {
        compile.push("--opt".to_string());
    } else if debug {
        compile.push("--debug".to_string());
    }
    run_compile(compile, Some(EmitKind::Wasm))?;
    let grants_filesystem = permissions
        .split(',')
        .any(|permission| matches!(permission.trim(), "all" | "*" | "read" | "write" | "delete"));
    let output = runtime_adapter
        .command(runtime_executable.clone(), &wasm, &args, grants_filesystem)
        .output()
        .map_err(|error| {
            format!(
                "failed to start WebAssembly runtime '{runtime_executable}': {error}. Install it or choose another with --runtime"
            )
        })?;
    let _ = fs::remove_file(&wasm);
    write_runtime_output(&output)?;
    if output.status.success() {
        Ok(())
    } else {
        Err(runtime_failure_message(&output, runtime, &path, debug))
    }
}

fn source_error(path: &str, error: impl std::fmt::Display) -> String {
    format!("in '{path}': {error}")
}

fn emit_debug_analysis_warnings(path: &str, source: &str, typed: &TypedExpression) {
    for finding in crate::static_analysis::analyze_user_program_diagnostics_detailed(
        typed,
        user_form_count(source),
    ) {
        let message = crate::lsp_native_core::restore_generated_source_names(&finding.message);
        let summary = crate::lsp_native_core::static_analysis_diagnostic_summary(&message);
        let range = crate::lsp_native_core::static_analysis_diagnostic_ranges(
            source,
            &finding.message,
            finding.user_form_index,
        )
        .into_iter()
        .next();
        if let Some(range) = range {
            eprintln!(
                "Warning: {path}:{}:{}: {summary}",
                range.start.line + 1,
                range.start.character + 1
            );
            if let Some(snippet) = crate::lsp_native_core::text_for_range(source, range) {
                eprintln!("  {}", snippet.replace('\n', " ").trim());
            }
        } else {
            eprintln!("Warning: {path}: {message}");
        }
    }
}

fn write_runtime_output(output: &Output) -> Result<(), String> {
    std::io::stdout()
        .write_all(&output.stdout)
        .map_err(|error| format!("failed to write program output: {error}"))?;
    if output.status.success() {
        std::io::stderr()
            .write_all(&output.stderr)
            .map_err(|error| format!("failed to write program diagnostics: {error}"))?;
    }
    Ok(())
}

fn runtime_failure_message(
    output: &Output,
    runtime: String,
    source_path: &str,
    debug: bool,
) -> String {
    let stderr = String::from_utf8_lossy(&output.stderr);
    let reason = runtime_failure_reason(&stderr, debug);
    let status = output
        .status
        .code()
        .map(|code| format!("exit code {code}"))
        .unwrap_or_else(|| "terminated by signal".to_string());
    format!("in '{source_path}': runtime error using {runtime}: {reason} ({status})")
}

fn runtime_failure_reason(stderr: &str, debug: bool) -> String {
    if let Some(guard) = stderr
        .lines()
        .map(str::trim)
        .find(|line| line.starts_with("debug.guard_trap:"))
    {
        return guard.to_string();
    }

    let lower = stderr.to_ascii_lowercase();
    if lower.contains("out of bounds")
        || lower.contains("invalid read address")
        || lower.contains("invalid write address")
    {
        "vector access was outside its valid range".to_string()
    } else if lower.contains("divide by zero") {
        "division or modulo by zero".to_string()
    } else if debug {
        "a debug safety check trapped (check vector bounds, division by zero, and Int overflow)"
            .to_string()
    } else {
        "the WebAssembly program trapped; rerun with --debug for Que safety checks".to_string()
    }
}

fn run_explain(raw: &[String]) -> Result<(), String> {
    let mut args = raw.to_vec();
    let json = take_flag(&mut args, "--json");
    let opt = take_flag(&mut args, "--opt");
    let out = take_value(&mut args, "--out")?;
    if opt {
        enable_opt();
    }
    let path = args
        .first()
        .ok_or_else(|| "explain requires a program path".to_string())?;
    let source = read_program(path)?;
    let merged = merged_project_program(path, &source)?;
    let typed = infer_program(&source, &merged)?;
    let count = user_form_count(&source);
    let wat = crate::wat::compile_program_to_split_wat_typed(&typed)?;
    let report = crate::explain::explain_program_with_effects_and_source(
        &typed,
        &wat.user_wat,
        count,
        &Default::default(),
        Some(&source),
    );
    let rendered = if json {
        crate::explain::render_json(&report)?
    } else {
        crate::explain::render_text(&report)
    };
    write_output(out.as_deref(), format!("{rendered}\n").as_bytes())
}

fn run_fmt(raw: &[String]) -> Result<(), String> {
    let mut args = raw.to_vec();
    let stdin = take_flag(&mut args, "--stdin");
    let check = take_flag(&mut args, "--check");
    let stdout = stdin || take_flag(&mut args, "--stdout");
    let (path, source) = if stdin {
        let mut source = String::new();
        std::io::stdin()
            .read_to_string(&mut source)
            .map_err(|error| error.to_string())?;
        (None, source)
    } else {
        let path = args
            .first()
            .ok_or_else(|| "fmt requires a path or --stdin".to_string())?;
        (Some(path.clone()), read_program(path)?)
    };
    let formatted = crate::formatter::format_source(&source)?;
    if check {
        if formatted != source {
            return Err(format!(
                "{} is not formatted",
                path.as_deref().unwrap_or("stdin")
            ));
        }
    } else if stdout {
        print!("{formatted}");
    } else if let Some(path) = path {
        fs::write(&path, formatted)
            .map_err(|error| format!("failed to write '{path}': {error}"))?;
    }
    Ok(())
}

#[cfg(test)]
mod runtime_adapter_tests {
    use super::*;

    fn command_parts(command: &Command) -> (String, Vec<String>) {
        (
            command.get_program().to_string_lossy().into_owned(),
            command
                .get_args()
                .map(|arg| arg.to_string_lossy().into_owned())
                .collect(),
        )
    }

    #[test]
    fn wasmtime_adapter_places_capabilities_before_module() {
        let (adapter, executable) = WasmRuntime::parse("wasmtime").unwrap();
        let command = adapter.command(
            executable,
            Path::new("program.wasm"),
            &["hello".into()],
            true,
        );
        assert_eq!(
            command_parts(&command),
            (
                "wasmtime".into(),
                vec!["run", "--dir", ".", "program.wasm", "hello"]
                    .into_iter()
                    .map(str::to_string)
                    .collect()
            )
        );
    }

    #[test]
    fn wasmer_adapter_uses_volume_and_argument_separator() {
        let (adapter, executable) = WasmRuntime::parse("wasmer").unwrap();
        let command = adapter.command(
            executable,
            Path::new("program.wasm"),
            &["hello".into()],
            true,
        );
        assert_eq!(
            command_parts(&command),
            (
                "wasmer".into(),
                vec!["run", "program.wasm", "--volume", ".:.", "--", "hello"]
                    .into_iter()
                    .map(str::to_string)
                    .collect()
            )
        );
    }

    #[test]
    fn wamr_alias_selects_iwasm_adapter() {
        let (adapter, executable) = WasmRuntime::parse("wamr").unwrap();
        let command = adapter.command(executable, Path::new("program.wasm"), &[], true);
        assert_eq!(
            command_parts(&command),
            (
                "iwasm".into(),
                vec!["--dir=.", "program.wasm"]
                    .into_iter()
                    .map(str::to_string)
                    .collect()
            )
        );
    }

    #[test]
    fn supported_runtime_path_is_preserved() {
        let (adapter, executable) = WasmRuntime::parse("/opt/runtime/bin/wasmtime").unwrap();
        assert_eq!(adapter, WasmRuntime::Wasmtime);
        assert_eq!(executable, "/opt/runtime/bin/wasmtime");
    }

    #[test]
    fn unknown_runtime_is_rejected() {
        assert!(WasmRuntime::parse("mystery-vm").is_err());
    }

    #[test]
    fn runtime_failure_reason_keeps_que_guard_diagnostic() {
        let stderr = "Error: failed to run\ndebug.guard_trap: integer overflow on mul/square (QUE_INT_OVERFLOW_CHECK)\nwasm backtrace:";
        assert_eq!(
            runtime_failure_reason(stderr, true),
            "debug.guard_trap: integer overflow on mul/square (QUE_INT_OVERFLOW_CHECK)"
        );
    }

    #[test]
    fn runtime_failure_reason_hides_wasm_backtrace_for_debug_traps() {
        let stderr = "Error: failed to run main module\n\nCaused by:\n    0: error while executing at wasm backtrace:\n    1: wasm trap: wasm `unreachable` instruction executed";
        let reason = runtime_failure_reason(stderr, true);
        assert!(reason.contains("debug safety check"), "{reason}");
        assert!(!reason.contains("backtrace"), "{reason}");
    }

    #[test]
    fn runtime_failure_reason_classifies_memory_traps() {
        let reason = runtime_failure_reason("invalid write address: -2147483632", false);
        assert_eq!(reason, "vector access was outside its valid range");
    }
}
