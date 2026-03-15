use crate::parser::{ self, Expression };
use std::env;
use std::fs;
use std::path::{ Path, PathBuf };

const ENV_LIB_PATH: &str = "QUE_LIB_PATH";

pub fn external_library_path() -> PathBuf {
    env::var(ENV_LIB_PATH)
        .map(PathBuf::from)
        .unwrap_or_else(|_| default_external_library_path())
}

fn default_external_library_path() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        if let Ok(local_app_data) = env::var("LOCALAPPDATA") {
            return PathBuf::from(local_app_data)
                .join("Programs")
                .join("Que")
                .join("share")
                .join("que")
                .join("que-lib.lisp");
        }
        return PathBuf::from("que-lib.lisp");
    }

    #[cfg(not(target_os = "windows"))]
    {
        PathBuf::from("/usr/local/share/que/que-lib.lisp")
    }
}

fn parse_ast_source(source: &str, label: &str) -> Result<Expression, String> {
    parser::build(source).map_err(|e| format!("Failed to parse library '{}': {}", label, e))
}

pub fn ast_to_definitions(ast: Expression, label: &str) -> Result<Vec<Expression>, String> {
    let mut current = ast;
    loop {
        let Expression::Apply(items) = current else {
            return Err(format!("library '{}' did not parse as top-level do expression", label));
        };
        if !matches!(items.first(), Some(Expression::Word(w)) if w == "do") {
            return Err(format!("library '{}' did not parse as top-level do expression", label));
        }

        let mut forms = items.into_iter().skip(1).collect::<Vec<_>>();
        if forms.len() == 1 {
            let only = forms.remove(0);
            match only {
                Expression::Apply(inner) if
                    matches!(inner.first(), Some(Expression::Word(w)) if w == "do")
                => {
                    current = Expression::Apply(inner);
                    continue;
                }
                other => {
                    return Ok(vec![other]);
                }
            }
        }

        return Ok(forms);
    }
}

pub fn load_ast_from_path(path: &Path) -> Result<Expression, String> {
    let source = fs::read_to_string(path).map_err(|e|
        format!("Failed to read library '{}': {}", path.display(), e)
    )?;
    parse_ast_source(&source, &path.display().to_string())
}

#[cfg(all(not(target_arch = "wasm32"), not(test)))]
fn candidate_library_paths() -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Ok(path) = env::var(ENV_LIB_PATH) {
        if !path.trim().is_empty() {
            out.push(PathBuf::from(path));
        }
    }
    out.push(default_external_library_path());

    if let Ok(exe) = env::current_exe() {
        if let Some(dir) = exe.parent() {
            out.push(dir.join("que-lib.lisp"));
            out.push(dir.join("../share/que/que-lib.lisp"));
        }
    }

    out.push(PathBuf::from("./dist/que-lib.lisp"));
    out
}

#[cfg(all(not(target_arch = "wasm32"), not(test)))]
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

#[cfg(all(test, not(target_arch = "wasm32")))]
fn load_embedded_test_library() -> Result<Expression, String> {
    let combined = format!(
        "{}\n{}\n{}\n{}",
        include_str!("../lisp/const.lisp"),
        include_str!("../lisp/std.lisp"),
        include_str!("../lisp/fp.lisp"),
        include_str!("../lisp/ds.lisp")
    );
    parser::build(&combined).map_err(|e| format!("Failed to parse embedded test library: {}", e))
}

pub fn load_ast() -> Expression {
    #[cfg(target_arch = "wasm32")]
    {
        return load_embedded_wasm_library()
            .unwrap_or_else(|_| Expression::Apply(vec![Expression::Word("do".to_string())]));
    }

    #[cfg(all(test, not(target_arch = "wasm32")))]
    {
        return load_embedded_test_library()
            .unwrap_or_else(|_| Expression::Apply(vec![Expression::Word("do".to_string())]));
    }

    #[cfg(all(not(test), not(target_arch = "wasm32")))]
    {
        match load_from_external_paths() {
            Ok(Some(ast)) => return ast,
            Ok(None) => {}
            Err(err) => panic!("{}", err),
        }
        Expression::Apply(vec![Expression::Word("do".to_string())])
    }
}
