use crate::parser::{ self, Expression };
use std::env;
use std::fs;
use std::path::{ Path, PathBuf };

const ENV_LIB_PATH: &str = "QUE_LIB_PATH";
const DEFAULT_LIB_PATH: &str = "/usr/local/share/que/que-lib.lisp";

pub fn external_library_path() -> PathBuf {
    env::var(ENV_LIB_PATH)
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(DEFAULT_LIB_PATH))
}

fn parse_ast_source(source: &str, label: &str) -> Result<Expression, String> {
    parser::build(source).map_err(|e| format!("Failed to parse library '{}': {}", label, e))
}

pub fn load_ast_from_path(path: &Path) -> Result<Expression, String> {
    let source = fs::read_to_string(path).map_err(|e|
        format!("Failed to read library '{}': {}", path.display(), e)
    )?;
    parse_ast_source(&source, &path.display().to_string())
}

#[cfg(not(target_arch = "wasm32"))]
fn candidate_library_paths() -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Ok(path) = env::var(ENV_LIB_PATH) {
        if !path.trim().is_empty() {
            out.push(PathBuf::from(path));
        }
    }
    out.push(PathBuf::from(DEFAULT_LIB_PATH));

    if let Ok(exe) = env::current_exe() {
        if let Some(dir) = exe.parent() {
            out.push(dir.join("que-lib.lisp"));
            out.push(dir.join("../share/que/que-lib.lisp"));
        }
    }

    out.push(PathBuf::from("./dist/que-lib.lisp"));
    out
}

#[cfg(not(target_arch = "wasm32"))]
fn load_from_external_paths() -> Result<Option<Expression>, String> {
    for path in candidate_library_paths() {
        if !path.exists() {
            continue;
        }
        match load_ast_from_path(&path) {
            Ok(ast) => return Ok(Some(ast)),
            Err(err) => return Err(err),
        }
    }
    Ok(None)
}

#[cfg(target_arch = "wasm32")]
fn load_embedded_wasm_library() -> Result<Expression, String> {
    let combined = format!(
        "{}\n{}\n{}\n{}",
        include_str!("../lisp/const.lisp"),
        include_str!("../lisp/std.lisp"),
        include_str!("../lisp/fp.lisp"),
        include_str!("../lisp/ds.lisp")
    );
    parser::build(&combined).map_err(|e| format!("Failed to parse embedded wasm library: {}", e))
}

pub fn load_ast() -> Expression {
    #[cfg(target_arch = "wasm32")]
    {
        return load_embedded_wasm_library()
            .unwrap_or_else(|_| Expression::Apply(vec![Expression::Word("do".to_string())]));
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
    match load_from_external_paths() {
        Ok(Some(ast)) => return ast,
        Ok(None) => {}
        Err(err) => panic!("{}", err),
    }
    Expression::Apply(vec![Expression::Word("do".to_string())])
    }
}
