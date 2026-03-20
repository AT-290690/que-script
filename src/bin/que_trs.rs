#[path = "../../miscs/transpilers/js.rs"]
mod js_transpiler;
#[path = "../../miscs/transpilers/clojure.rs"]
mod clojure_transpiler;
#[path = "../../miscs/transpilers/ocaml.rs"]
mod ocaml_transpiler;

use std::fs;
use std::path::Path;

fn usage() -> String {
    "Usage: que-trs (--ml | --js | --clj) --s <source.que> [--d <output>]\n\
     Transpiles a Que program after std merge, macro expansion, tree-shaking, and desugaring.\n\
     Examples:\n\
       que-trs --ml --s ./example/main.que --d ./example/dist/main.ml\n\
       que-trs --js --s ./example/main.que --d ./example/dist/main.js\n\
       que-trs --clj --s ./example/main.que --d ./example/dist/main.clj"
        .to_string()
}

fn main() -> Result<(), String> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if args.is_empty() || args.iter().any(|a| a == "--help" || a == "-h") {
        println!("{}", usage());
        return Ok(());
    }

    let mut target: Option<&'static str> = None;
    let mut src: Option<String> = None;
    let mut dst: Option<String> = None;

    let mut i = 0usize;
    while i < args.len() {
        match args[i].as_str() {
            "--ml" => {
                target = Some("ml");
                i += 1;
            }
            "--js" => {
                target = Some("js");
                i += 1;
            }
            "--clj" => {
                target = Some("clj");
                i += 1;
            }
            "--s" => {
                i += 1;
                if i >= args.len() {
                    return Err("--s requires a source path".to_string());
                }
                src = Some(args[i].clone());
                i += 1;
            }
            "--d" => {
                i += 1;
                if i >= args.len() {
                    return Err("--d requires an output path".to_string());
                }
                dst = Some(args[i].clone());
                i += 1;
            }
            other => {
                return Err(format!("unknown argument '{}'\n{}", other, usage()));
            }
        }
    }

    let target = target.ok_or_else(|| format!("missing target flag\n{}", usage()))?;
    let src = src.ok_or_else(|| format!("missing --s <source.que>\n{}", usage()))?;
    let program = fs::read_to_string(&src).map_err(|e| format!("failed to read '{}': {}", src, e))?;

    let std_ast = que::baked::load_ast();
    let lib_defs = que::baked::ast_to_definitions(std_ast, "active library")?;
    let wrapped_ast = que::parser::merge_std_and_program(&program, lib_defs)?;

    let output = match target {
        "ml" => ocaml_transpiler::compile_program_to_ocaml(&wrapped_ast)?,
        "js" => js_transpiler::compile_program_to_js(&wrapped_ast),
        "clj" => clojure_transpiler::compile_program_to_clj(&wrapped_ast),
        _ => unreachable!(),
    };

    if let Some(path) = dst {
        if let Some(parent) = Path::new(&path).parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)
                    .map_err(|e| format!("failed to create '{}': {}", parent.display(), e))?;
            }
        }
        fs::write(&path, output).map_err(|e| format!("failed to write '{}': {}", path, e))?;
    } else {
        println!("{}", output);
    }

    Ok(())
}
