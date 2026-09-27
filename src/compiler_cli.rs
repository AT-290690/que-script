//! Compiler and runner commands used by the unified `que` CLI.
//!
//! This module deliberately has no dependency on the embedded Wasmtime host.

use crate::infer::{infer_with_builtins_typed_lsp, InferErrorInfo, TypedExpression};
use crate::parser::Expression;
use std::env;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};

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
    C,
    Types,
}

fn help() -> &'static str {
    "que — Que compiler, tooling, and runner

Usage:
  que <program.que> [arguments ...] [--opt] [--runtime <runtime>]
  que compile <program.que> [--opt] [--out <program.wasm>]
  que run <program.que> [arguments ...] [--opt] [--allow <permissions ...>]
  que run-wasi <program.que> [arguments ...] [--opt] [--runtime <runtime>]
  que wat <program.que>
  que wat --eval <source>
  que <program.que> --emit <source|opt-source|wat|wasm|c|types> [--out <file>]
  que explain <program.que> [--json] [--opt] [--out <file>]
  que fmt <program.que> [--check|--stdout]
  que fmt --stdin

`que run` uses the separately installed wasm2c and C compiler. `run-wasi` uses
a user-installed runtime: wasmtime (default), wasmer, or iwasm/WAMR. Select it
with --runtime or QUE_WASM_RUNTIME. Set QUE_NATIVE_SCRIPT to override the
native-C driver path."
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
    match args[0].as_str() {
        "fmt" => return run_fmt(&args[1..]),
        "explain" => return run_explain(&args[1..]),
        "compile" => {
            args.remove(0);
            return run_compile(args, Some(EmitKind::Wasm));
        }
        "run" => {
            args.remove(0);
            return run_native(args);
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

fn merged_program(source: &str) -> Result<Expression, String> {
    let std_ast = crate::baked::load_ast();
    let mut definitions = crate::baked::ast_to_definitions(std_ast, "active library")?;
    crate::externals::extend_with_builtin_host_externs(&mut definitions)?;
    crate::parser::merge_std_and_program(source, definitions)
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
        .map_err(|InferErrorInfo { message, .. }| message)
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
            "c" => EmitKind::C,
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
    let source = read_program(path)?;
    let merged = merged_program(&source)?;
    if emit == EmitKind::Source {
        return write_output(out.as_deref(), format!("{}\n", merged.to_lisp()).as_bytes());
    }
    let typed = infer_program(&source, &merged)?;
    if env::var("QUEC_DEBUG_ANALYSIS").as_deref() == Ok("1") {
        for warning in crate::static_analysis::analyze_user_program_diagnostics(
            &typed,
            user_form_count(&source),
        ) {
            eprintln!("Warning: {warning}");
        }
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
    let wat = crate::wat::compile_program_to_wat_typed(&typed)?;
    if emit == EmitKind::Wat {
        write_output(out.as_deref(), wat.as_bytes())
    } else if emit == EmitKind::Wasm {
        let wasm =
            wat::parse_str(&wat).map_err(|error| format!("failed to encode Wasm: {error}"))?;
        write_output(out.as_deref(), &wasm)
    } else {
        let output = out.unwrap_or_else(|| {
            Path::new(path)
                .with_extension("c")
                .to_string_lossy()
                .into_owned()
        });
        let wasm =
            wat::parse_str(&wat).map_err(|error| format!("failed to encode Wasm: {error}"))?;
        let wasm_path = env::temp_dir().join(format!("que-emit-c-{}.wasm", std::process::id()));
        fs::write(&wasm_path, wasm)
            .map_err(|error| format!("failed to stage Wasm for wasm2c: {error}"))?;
        let status = Command::new("wasm2c")
            .arg(&wasm_path)
            .arg("-n")
            .arg("main")
            .arg("-o")
            .arg(&output)
            .status()
            .map_err(|error| format!("failed to start wasm2c: {error}"))?;
        let _ = fs::remove_file(&wasm_path);
        status_result(status, "C emission")?;
        println!("{output}");
        Ok(())
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
        path,
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
    let status = runtime_adapter
        .command(runtime_executable.clone(), &wasm, &args, grants_filesystem)
        .status()
        .map_err(|error| {
            format!(
                "failed to start WebAssembly runtime '{runtime_executable}': {error}. Install it or choose another with --runtime"
            )
        })?;
    let _ = fs::remove_file(&wasm);
    status_result(status, &format!("external {runtime:?} execution"))
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
    let merged = merged_program(&source)?;
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

fn native_script() -> Result<PathBuf, String> {
    if let Some(path) =
        env::var_os("QUE_NATIVE_SCRIPT").or_else(|| env::var_os("QUEC_NATIVE_SCRIPT"))
    {
        return Ok(PathBuf::from(path));
    }
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/compile-native-c.sh");
    if manifest.is_file() {
        return Ok(manifest);
    }
    if let Ok(executable) = env::current_exe() {
        if let Some(prefix) = executable.parent().and_then(Path::parent) {
            let installed = prefix.join("share/que/compile-native-c.sh");
            if installed.is_file() {
                return Ok(installed);
            }
        }
    }
    Err("native-C driver not found; set QUE_NATIVE_SCRIPT to compile-native-c.sh".into())
}

fn status_result(status: ExitStatus, operation: &str) -> Result<(), String> {
    if status.success() {
        Ok(())
    } else {
        Err(format!("{operation} failed with {status}"))
    }
}

fn run_native(mut args: Vec<String>) -> Result<(), String> {
    let opt = take_flag(&mut args, "--opt");
    let debug = take_flag(&mut args, "--debug");
    if opt && debug {
        return Err("--opt and --debug cannot be used together".to_string());
    }
    if opt {
        enable_opt();
    }
    if debug {
        enable_debug();
    }
    let path = args
        .first()
        .cloned()
        .ok_or_else(|| "missing program path".to_string())?;
    let build = env::temp_dir().join(format!("que-native-{}", std::process::id()));
    fs::create_dir_all(&build)
        .map_err(|error| format!("failed to create native build directory: {error}"))?;
    let script = native_script()?;
    let status = Command::new(script)
        .arg(&path)
        .arg(&build)
        .env(
            "QUE_COMPILER",
            env::current_exe().map_err(|error| error.to_string())?,
        )
        .stdout(Stdio::null())
        .status()
        .map_err(|error| format!("failed to start native-C compiler: {error}"))?;
    status_result(status, "native-C compilation")?;

    let mut allow = Vec::new();
    if let Some(index) = args.iter().position(|arg| arg == "--allow") {
        let mut i = index + 1;
        while i < args.len() && !args[i].starts_with("--") {
            allow.push(args[i].clone());
            i += 1;
        }
        args.drain(index..i);
    }
    args.remove(0);
    let mut command = Command::new(build.join("main"));
    if !allow.is_empty() {
        command.env("QUE_ALLOW", allow.join(","));
    }
    command.args(args);
    let status = command
        .status()
        .map_err(|error| format!("failed to run native program: {error}"))?;
    let result = status_result(status, "native program");
    let _ = fs::remove_dir_all(&build);
    result
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
}
