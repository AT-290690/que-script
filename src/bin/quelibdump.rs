use std::fs::{ self, File };
use std::io::{ self, Write };
use std::path::Path;

use que::infer::infer_with_builtins_typed;
use que::lsp_native_core::normalize_signature;
use que::parser::{ self, Expression };
use que::types::{ create_builtin_environment, TypeEnv };

fn infer_std_symbol(name: &str, std_items: &[Expression]) -> Result<String, String> {
    let merged = parser::merge_std_and_program(name, std_items.to_vec())?;
    let (typ, _typed) = infer_with_builtins_typed(
        &merged,
        create_builtin_environment(TypeEnv::new())
    )?;
    Ok(normalize_signature(&typ.to_string()))
}

fn collect_std_names(std_ast: &Expression) -> Vec<String> {
    let mut out = Vec::new();
    let Expression::Apply(items) = std_ast else {
        return out;
    };
    for expr in items.iter().skip(1) {
        let Expression::Apply(list) = expr else {
            continue;
        };
        if list.len() < 2 {
            continue;
        }
        let Expression::Word(keyword) = &list[0] else {
            continue;
        };
        if keyword != "let" && keyword != "let*" {
            continue;
        }
        let Expression::Word(name) = &list[1] else {
            continue;
        };
        out.push(name.clone());
    }
    out
}

fn run() -> io::Result<()> {
    let output_path = std::env
        ::args()
        .nth(1)
        .unwrap_or_else(|| "./example/dist/lib.json".to_string());

    let std_ast = que::baked::load_ast();
    let std_items = match &std_ast {
        Expression::Apply(items) => items[1..].to_vec(),
        _ => {
            return Err(
                io::Error::new(io::ErrorKind::InvalidData, "failed to load standard library AST")
            );
        }
    };

    let mut names: Vec<[String; 2]> = Vec::new();

    for name in collect_std_names(&std_ast) {
        if name.ends_with('!') {
            names.push([name, "unsafe".to_string()]);
            continue;
        }
        match infer_std_symbol(&name, &std_items) {
            Ok(typ) => names.push([name, typ]),
            Err(err) => eprintln!("std infer failed for '{}': {}", name, err),
        }
    }

    if let Some(parent) = Path::new(&output_path).parent() {
        fs::create_dir_all(parent)?;
    }
    let mut file = File::create(&output_path)?;
    serde_json
        ::to_writer_pretty(&mut file, &names)
        .map_err(|err| io::Error::new(io::ErrorKind::Other, err.to_string()))?;
    writeln!(file)?;

    eprintln!("wrote {}", output_path);
    Ok(())
}

fn main() {
    if let Err(err) = run() {
        eprintln!("quelibdump error: {}", err);
        std::process::exit(1);
    }
}
