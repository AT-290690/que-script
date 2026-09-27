//! Lightweight compiler CLI used by `quec`.
//!
//! This module deliberately has no dependency on the embedded Wasmtime host.

use crate::infer::{infer_with_builtins_typed_lsp, InferErrorInfo, TypedExpression};
use crate::parser::Expression;
use std::env;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};

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
    "quec — lightweight Que compiler and optional native-C runner

Usage:
  quec <program.que> > program.wasm
  quec compile <program.que> [--opt] [--out <program.wasm>]
  quec run <program.que> [arguments ...] [--opt] [--allow <permissions ...>]
  quec run-wasi <program.que> [arguments ...] [--opt]
  quec <program.que> --emit <source|opt-source|wat|wasm|c|types> [--out <file>]
  quec explain <program.que> [--json] [--opt] [--out <file>]
  quec fmt <program.que> [--check|--stdout]
  quec fmt --stdin

`quec run` uses the separately installed wasm2c and C compiler. Wasmtime is not
linked into quec. Set QUEC_NATIVE_SCRIPT to override the native-C driver path."
}

pub fn run() -> Result<(), String> {
    let mut args = env::args().skip(1).collect::<Vec<_>>();
    if args.is_empty() || matches!(args[0].as_str(), "--help" | "-h") {
        println!("{}", help());
        return Ok(());
    }
    if matches!(args[0].as_str(), "--version" | "-V") {
        println!("quec {}", env!("CARGO_PKG_VERSION"));
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
        _ => {}
    }
    if args.iter().any(|arg| arg == "--emit") {
        run_compile(args, None)
    } else {
        run_compile(args, Some(EmitKind::Wasm))
    }
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
        let wasm_path = env::temp_dir().join(format!("quec-emit-c-{}.wasm", std::process::id()));
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
    let permissions = if let Some(index) = args.iter().position(|arg| arg == "--allow") {
        let permissions = args[index + 1..].join(",");
        args.truncate(index);
        permissions
    } else {
        String::new()
    };
    env::set_var("QUE_WASI_ALLOW", &permissions);
    let path = args
        .first()
        .cloned()
        .ok_or_else(|| "run-wasi requires a program path".to_string())?;
    args.remove(0);
    let wasm = env::temp_dir().join(format!("quec-wasi-{}.wasm", std::process::id()));
    let mut compile = vec![
        path,
        "--wasi".to_string(),
        "--out".to_string(),
        wasm.to_string_lossy().into_owned(),
    ];
    if opt {
        compile.push("--opt".to_string());
    }
    run_compile(compile, Some(EmitKind::Wasm))?;
    let grants_filesystem = permissions
        .split(',')
        .any(|permission| matches!(permission.trim(), "all" | "*" | "read" | "write" | "delete"));
    let mut command = Command::new("wasmtime");
    command.arg("run");
    if grants_filesystem {
        command.arg("--dir").arg(".");
    }
    let status = command
        .arg(&wasm)
        .args(args)
        .status()
        .map_err(|error| format!("failed to start external wasmtime: {error}"))?;
    let _ = fs::remove_file(&wasm);
    status_result(status, "external Wasmtime execution")
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
    if let Some(path) = env::var_os("QUEC_NATIVE_SCRIPT") {
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
    Err("native-C driver not found; set QUEC_NATIVE_SCRIPT to compile-native-c.sh".into())
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
    let build = env::temp_dir().join(format!("quec-native-{}", std::process::id()));
    fs::create_dir_all(&build)
        .map_err(|error| format!("failed to create native build directory: {error}"))?;
    let script = native_script()?;
    let status = Command::new(script)
        .arg(&path)
        .arg(&build)
        .env(
            "QUEC_COMPILER",
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
