use std::collections::HashMap;
use std::fs::{ self, File };
use std::io::{ self, Write };
use std::path::Path;

use que::infer::infer_with_builtins_typed;
use que::lsp_native_core::normalize_signature;
use que::parser::{ self, Expression };
use que::types::{ create_builtin_environment, TypeEnv };
use serde::Serialize;

fn infer_std_symbol(name: &str, std_items: &[Expression]) -> Result<String, String> {
    let merged = parser::merge_std_and_program(name, std_items.to_vec())?;
    let (typ, _typed) = infer_with_builtins_typed(
        &merged,
        create_builtin_environment(TypeEnv::new())
    )?;
    Ok(normalize_signature(&typ.to_string()))
}

#[derive(Clone)]
struct SymbolDef {
    name: String,
    source: String,
    is_std: bool,
}

#[derive(Serialize, Clone)]
struct DumpItem(pub String, pub String, pub String);

#[derive(Serialize)]
struct SplitDump {
    non_std: Vec<DumpItem>,
    std: Vec<DumpItem>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum OutputMode {
    Flat,
    SplitStd,
}

struct Cli {
    output_path: String,
    include_std: bool,
    output_mode: OutputMode,
}

fn parse_cli() -> Result<Cli, String> {
    let mut output_path = "./example/dist/lib.json".to_string();
    let mut include_std = true;
    let mut output_mode = OutputMode::Flat;

    let mut args = std::env::args().skip(1).peekable();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--no-std" => include_std = false,
            "--split-std" => output_mode = OutputMode::SplitStd,
            "--output" => {
                let Some(path) = args.next() else {
                    return Err("--output requires a path".to_string());
                };
                output_path = path;
            }
            _ => {
                if arg.starts_with("--") {
                    return Err(format!("unknown flag '{}'", arg));
                }
                output_path = arg;
            }
        }
    }

    Ok(Cli {
        output_path,
        include_std,
        output_mode,
    })
}

fn top_level_binding_name(expr: &Expression) -> Option<String> {
    let Expression::Apply(list) = expr else {
        return None;
    };
    if list.len() < 3 {
        return None;
    }
    let Expression::Word(keyword) = &list[0] else {
        return None;
    };
    if keyword != "let" && keyword != "let*" && keyword != "mut" {
        return None;
    }
    let Expression::Word(name) = &list[1] else {
        return None;
    };
    Some(name.clone())
}

fn matching_close(ch: char) -> Option<char> {
    match ch {
        '(' => Some(')'),
        '[' => Some(']'),
        '{' => Some('}'),
        _ => None,
    }
}

fn extract_top_level_forms(source: &str) -> io::Result<Vec<String>> {
    let mut out = Vec::new();
    let mut stack: Vec<char> = Vec::new();
    let mut in_string = false;
    let mut in_char = false;
    let mut in_comment = false;
    let mut start: Option<usize> = None;

    for (idx, ch) in source.char_indices() {
        if in_comment {
            if ch == '\n' {
                in_comment = false;
            }
            continue;
        }
        if in_string {
            if ch == '"' {
                in_string = false;
            }
            continue;
        }
        if in_char {
            if ch == '\'' {
                in_char = false;
            }
            continue;
        }

        match ch {
            ';' => in_comment = true,
            '"' => in_string = true,
            '\'' => in_char = true,
            '(' | '[' | '{' => {
                if stack.is_empty() && ch == '(' {
                    start = Some(idx);
                }
                stack.push(ch);
            }
            ')' | ']' | '}' => {
                let Some(open) = stack.pop() else {
                    return Err(
                        io::Error::new(
                            io::ErrorKind::InvalidData,
                            format!("Unexpected '{}' at byte {}", ch, idx)
                        )
                    );
                };
                let expected = matching_close(open).unwrap();
                if ch != expected {
                    return Err(
                        io::Error::new(
                            io::ErrorKind::InvalidData,
                            format!(
                                "Mismatched delimiter '{}' (expected '{}') at byte {}",
                                ch,
                                expected,
                                idx
                            )
                        )
                    );
                }
                if stack.is_empty() {
                    if let Some(s) = start.take() {
                        out.push(source[s..idx + ch.len_utf8()].trim().to_string());
                    }
                }
            }
            _ => {}
        }
    }

    if in_string {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "Unclosed string literal"));
    }
    if in_char {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "Unclosed char literal"));
    }
    if !stack.is_empty() {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "Unclosed delimiter"));
    }
    Ok(out)
}

fn collect_defs_from_file(path: &str, is_std: bool) -> io::Result<Vec<SymbolDef>> {
    let text = fs::read_to_string(path)?;
    let mut out = Vec::new();
    for form in extract_top_level_forms(&text)? {
        let built = parser::build(&form).map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;
        let exprs = match built {
            Expression::Apply(items) => items,
            other => vec![other],
        };
        for expr in exprs.iter().skip(1) {
            let Some(name) = top_level_binding_name(expr) else {
                continue;
            };
            out.push(SymbolDef {
                name,
                source: form.clone(),
                is_std,
            });
        }
    }
    Ok(out)
}

fn build_final_symbol_defs(include_std: bool) -> io::Result<Vec<SymbolDef>> {
    let mut files = vec![
        ("./lisp/const.lisp", false),
        ("./lisp/fp.lisp", false),
        ("./lisp/ds.lisp", false),
    ];
    if include_std {
        files.insert(1, ("./lisp/macros.lisp", true));
        files.insert(1, ("./lisp/std.lisp", true));
    }

    let mut by_name: HashMap<String, SymbolDef> = HashMap::new();
    let mut order: Vec<String> = Vec::new();

    for (path, is_std) in files {
        for def in collect_defs_from_file(path, is_std)? {
            if !by_name.contains_key(&def.name) {
                order.push(def.name.clone());
            }
            by_name.insert(def.name.clone(), def);
        }
    }

    let mut out = Vec::new();
    for name in order {
        if let Some(def) = by_name.remove(&name) {
            out.push(def);
        }
    }
    Ok(out)
}

fn to_dump_items(defs: &[SymbolDef], std_items: &[Expression]) -> Vec<DumpItem> {
    let mut out = Vec::new();
    for def in defs {
        match infer_std_symbol(&def.name, std_items) {
            Ok(typ) => out.push(DumpItem(def.name.clone(), typ, def.source.clone())),
            Err(err) => eprintln!("std infer failed for '{}': {}", def.name, err),
        }
    }
    out
}

fn run() -> io::Result<()> {
    let cli = parse_cli().map_err(|err| io::Error::new(io::ErrorKind::InvalidInput, err))?;

    let std_ast = que::baked::load_ast();
    let std_items = que::baked
        ::ast_to_definitions(std_ast, "active library")
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;

    let defs = build_final_symbol_defs(cli.include_std)?;

    let mut non_std_defs = Vec::new();
    let mut std_defs = Vec::new();
    for def in defs {
        if def.is_std {
            std_defs.push(def);
        } else {
            non_std_defs.push(def);
        }
    }

    let non_std_items = to_dump_items(&non_std_defs, &std_items);
    let std_items_dump = to_dump_items(&std_defs, &std_items);

    if let Some(parent) = Path::new(&cli.output_path).parent() {
        fs::create_dir_all(parent)?;
    }
    let mut file = File::create(&cli.output_path)?;
    match cli.output_mode {
        OutputMode::Flat => {
            let mut flat = non_std_items;
            if cli.include_std {
                flat.extend(std_items_dump);
            }
            serde_json
                ::to_writer_pretty(&mut file, &flat)
                .map_err(|err| io::Error::other(err.to_string()))?;
        }
        OutputMode::SplitStd => {
            let payload = SplitDump {
                non_std: non_std_items,
                std: if cli.include_std { std_items_dump } else { Vec::new() },
            };
            serde_json
                ::to_writer_pretty(&mut file, &payload)
                .map_err(|err| io::Error::other(err.to_string()))?;
        }
    }
    writeln!(file)?;

    eprintln!("wrote {}", cli.output_path);
    Ok(())
}

fn main() {
    if let Err(err) = run() {
        eprintln!("quelibdump error: {}", err);
        std::process::exit(1);
    }
}
