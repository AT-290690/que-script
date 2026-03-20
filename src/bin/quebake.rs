use que::parser::Expression;
use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

fn parse_bundle_paths_from_argv(argv: &[String]) -> Result<Vec<String>, String> {
    let mut out = Vec::new();
    let mut i = 0usize;
    while i < argv.len() {
        match argv[i].as_str() {
            "--help" | "-h" => {
                i += 1;
            }
            "--bundle" => {
                i += 1;
                if i >= argv.len() || argv[i].starts_with("--") {
                    return Err("--bundle requires at least one .que path".to_string());
                }
                while i < argv.len() && !argv[i].starts_with("--") {
                    out.push(argv[i].clone());
                    i += 1;
                }
            }
            "--out" => {
                i += 2;
            }
            other if other.starts_with("--") => {
                return Err(format!("unknown flag '{}'", other));
            }
            other => {
                out.push(other.to_string());
                i += 1;
            }
        }
    }
    Ok(out)
}

fn parse_output_path_from_argv(argv: &[String]) -> Result<PathBuf, String> {
    let mut out: Option<PathBuf> = None;
    let mut i = 0usize;
    while i < argv.len() {
        match argv[i].as_str() {
            "--out" => {
                i += 1;
                if i >= argv.len() || argv[i].starts_with("--") {
                    return Err("--out requires a path".to_string());
                }
                out = Some(PathBuf::from(&argv[i]));
                i += 1;
            }
            _ => {
                i += 1;
            }
        }
    }
    Ok(out.unwrap_or_else(|| PathBuf::from("./dist/que-lib.lisp")))
}

fn parse_definitions_only(source: &str, label: &str) -> Result<Vec<Expression>, String> {
    let root = que::parser::build_library(source)
        .map_err(|e| format!("failed to parse '{}': {}", label, e))?;
    let Expression::Apply(items) = root else {
        return Err(format!(
            "'{}' did not parse as top-level do expression",
            label
        ));
    };
    if !matches!(items.first(), Some(Expression::Word(w)) if w == "do") {
        return Err(format!(
            "'{}' did not parse as top-level do expression",
            label
        ));
    }
    let mut defs = Vec::new();
    for (idx, item) in items.iter().enumerate().skip(1) {
        let Expression::Apply(form) = item else {
            return Err(format!(
                "'{}' must contain only top-level definitions; found non-definition at form {}: {}",
                label,
                idx,
                item.to_lisp()
            ));
        };
        if form.len() < 3 {
            return Err(format!(
                "'{}' must contain only top-level definitions; malformed form {}: {}",
                label,
                idx,
                item.to_lisp()
            ));
        }
        let Expression::Word(kw) = &form[0] else {
            return Err(format!(
                "'{}' must contain only top-level definitions; malformed form {}: {}",
                label,
                idx,
                item.to_lisp()
            ));
        };
        if kw != "let" && kw != "letrec" && kw != "letmacro" && kw != "mut" {
            return Err(format!(
                "'{}' must contain only top-level definitions; found '{}' at form {}",
                label, kw, idx
            ));
        }
        defs.push(item.clone());
    }
    Ok(defs)
}

fn load_defs_from_path(path: &str, require_que_ext: bool) -> Result<Vec<Expression>, String> {
    let p = Path::new(path);
    if require_que_ext && p.extension().and_then(|e| e.to_str()) != Some("que") {
        return Err(format!("bundle '{}' must be a .que file", path));
    }
    let source = fs::read_to_string(p).map_err(|e| format!("failed to read '{}': {}", path, e))?;
    parse_definitions_only(&source, path)
}

fn base_library_defs() -> Result<Vec<Expression>, String> {
    let mut defs = Vec::new();
    for path in [
        "./lisp/const.lisp",
        "./lisp/macros.lisp",
        "./lisp/std.lisp",
        "./lisp/fp.lisp",
        "./lisp/ds.lisp",
        "./lisp/csv.lisp",
    ] {
        defs.extend(load_defs_from_path(path, false)?);
    }
    Ok(defs)
}

fn usage(bin_name: &str) -> String {
    format!(
        "Usage: {bin} [--bundle <helpers.que> [more.que ...]] [helpers.que ...] [--out <que-lib.lisp>]\n\
         \n\
         Bakes const/macros/std/fp/ds plus optional helper bundles into an external library file.\n\
         Helper bundles must contain only top-level definitions (let/letrec/letmacro/mut).\n\
         Rebuild/reinstall binaries (and restart LSP/editor) after baking.\n\
         After install, helper bundle source files may be removed.\n\
         \n\
         Default output: ./dist/que-lib.lisp\n\
         \n\
         Example:\n\
           {bin} --bundle ./helpers/math.que ./helpers/text.que --out ./dist/que-lib.lisp",
        bin = bin_name
    )
}

fn run() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();
    let bin_name = args
        .first()
        .and_then(|p| Path::new(p).file_name())
        .and_then(|p| p.to_str())
        .unwrap_or("quebake");
    if args.iter().any(|arg| (arg == "--help" || arg == "-h")) {
        println!("{}", usage(bin_name));
        return Ok(());
    }

    let bundle_paths = parse_bundle_paths_from_argv(&args[1..])
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;
    let output_path = parse_output_path_from_argv(&args[1..])
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;

    let mut defs =
        base_library_defs().map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    for path in &bundle_paths {
        defs.extend(
            load_defs_from_path(path, true)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?,
        );
    }

    let wrapped = Expression::Apply(
        std::iter::once(Expression::Word("do".to_string()))
            .chain(defs)
            .collect(),
    );

    if let Some(parent) = output_path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }
    fs::write(&output_path, format!("{}\n", wrapped.to_lisp()))?;
    if !bundle_paths.is_empty() {
        eprintln!("quebake: included {} bundle file(s)", bundle_paths.len());
    }
    eprintln!("quebake: wrote {}", output_path.display());
    Ok(())
}

fn main() {
    if let Err(err) = run() {
        eprintln!("quebake error: {}", err);
        std::process::exit(1);
    }
}
