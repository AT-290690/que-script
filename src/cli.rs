#![allow(dead_code)]
#![allow(warnings)]

use crate::baked::load_ast;
use crate::infer::infer_with_builtins_env;
use crate::ir::load_bytecode;
use crate::parser::build;
use crate::vm::parse_bitecode;
use crate::format::format;
use std::env;
use std::fs;
use std::io::Read;
use std::num::Wrapping;
use std::cell::RefCell;
use flate2::write::GzEncoder;
use flate2::Compression;
use std::io::{self, Write};
#[cfg(feature = "repl")]
use crate::repl::repl;
thread_local! {
    static STD: RefCell<crate::parser::Expression> = RefCell::new(crate::baked::load_ast());
}
pub fn run_code(program: String) -> String {
    STD.with(|std| {
        let std_ast = std.borrow();
        if let crate::parser::Expression::Apply(items) = &*std_ast {
            match crate::parser::merge_std_and_program(&program, items[1..].to_vec()) {
                Ok(wrapped_ast) => {
                    match crate::infer::infer_with_builtins(&wrapped_ast, crate::types::create_builtin_environment(crate::types::TypeEnv::new())) {
                        Ok(typ) =>
                             match crate::vm::run(&wrapped_ast, crate::vm::VM::new()) {
                                Ok(res) => return format!("{}\n{:?}", typ, res),
                                Err(err) => return err,
                            }
                        Err(err) => return err,
                    }
                }
                Err(err) => return err,
            }
        }
        "No expressions...".to_string()
    })
}
fn compress(data: &str) -> Vec<u8> {
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(data.as_bytes()).expect("Failed to compress data");
    encoder.finish().expect("Failed to finish compression")
}

fn dump_wrapped_libs(expr: &str, path: &str) -> io::Result<()> {
    let mut file = fs::File::create(path)?;
    writeln!(file, "use std::io::Read;fn decompress(compressed: &[u8]) -> crate::parser::Expression {{use flate2::read::GzDecoder;let mut decoder = GzDecoder::new(compressed);let mut decompressed_data = String::new();decoder.read_to_string(&mut decompressed_data).expect(\"Failed to decompress data\");let expressions =crate::parser::parse(&decompressed_data).expect(\"Failed to parse decompressed data\");expressions.first().expect(\"No expressions returned\").clone()}}");
    let compressed_code = compress(&expr);
    writeln!(file, "pub fn load_ast() -> crate::parser::Expression {{")?;
    writeln!(file, "decompress(&{:?})", compressed_code)?;
    writeln!(file, "}}")?;
    Ok(())
}


pub fn dump_wrapped_bytecode(code: Vec<crate::vm::Instruction>, path: &str) -> std::io::Result<()> {
    let mut file: fs::File = fs::File::create(path)?;
    writeln!(file, "use crate::vm::Instruction::*;")?;
    // writeln!(
    //     file,
    //     "macro_rules! bytecode {{ ( $($instr:expr),* ) => {{ vec![$( $instr ),*] }};}}"
    // );
    writeln!(file, "macro_rules! s {{($s:expr) => {{ $s.to_string() }}}}")?;
    writeln!(
        file,
        "pub fn load_bytecode() -> Vec<crate::vm::Instruction> {{"
    )?;
    write!(file, "vec![")?;

    for (i, instr) in code.iter().enumerate() {
        write!(file, "{}", instr.to_rust())?;
        if i < code.len() - 1 {
            write!(file, ",")?;
        }
    }

    writeln!(file, "]")?;
    write!(file, "}}")?;
    Ok(())
}

pub fn dump_raw_bytecode(code: Vec<crate::vm::Instruction>, path: &str) -> std::io::Result<()> {
    std::fs::create_dir_all(std::path::Path::new(path).parent().unwrap()).unwrap();
    let mut file = fs::File::create(path)?;
    write!(file, "[")?;
    for (i, instr) in code.iter().enumerate() {
        write!(file, "{}", instr.serialise())?;
        if i < code.len() - 1 {
            write!(file, ",")?;
        }
    }
    writeln!(file, "]")?;
    Ok(())
}

pub fn dump_wrapped_js(src: String, path: &str) -> std::io::Result<()> {
    std::fs::create_dir_all(std::path::Path::new(path).parent().unwrap()).unwrap();
    let mut file = fs::File::create(path)?;
    writeln!(file, "const _ =(")?;
    writeln!(file, "{}", src)?;
    writeln!(file, ");console.log(_)")?;

    Ok(())
}
pub fn cons(a: String, b: String) -> String {
    return format!(
        "{:?}",
        parse_bitecode(&a)
            .unwrap()
            .into_iter()
            .chain(parse_bitecode(&b).unwrap().into_iter())
            .collect::<Vec<_>>(),
    );
}
pub fn cli(dir: &str) -> std::io::Result<()> {
    let path = format!("{}/{}", dir, "main.lisp");
    let dist = format!("{}/dist", dir);

    let args: Vec<String> = env::args().collect();
    let cmd = args.iter().last().unwrap();
    if cmd == "--std" {
        let combined = format!(
                                    "{}\n{}\n{}\n{}", 
                                    fs::read_to_string("./lisp/const.lisp")?,
                                    fs::read_to_string("./lisp/std.lisp")?,
                                    fs::read_to_string("./lisp/fp.lisp")?,
                                    fs::read_to_string("./lisp/ds.lisp")?);
        // let mut file = fs::File::create("./combined.lisp")?;
        // writeln!(file, "{}", combined)?;
        dump_wrapped_libs(&build(&combined).unwrap().to_lisp(), "./src/baked.rs");
    } else if cmd == "--check" {
        let program = fs::read_to_string(path)?;
        STD.with(|std| {
            let std_ast = std.borrow();
            if let crate::parser::Expression::Apply(items) = &*std_ast {
                match crate::parser::merge_std_and_program(&program, items[1..].to_vec()) {
                    Ok(wrapped_ast) => match crate::infer::infer_with_builtins(
                        &wrapped_ast,
                        crate::types::create_builtin_environment(crate::types::TypeEnv::new())
                    ) {
                        Ok(typ) => println!("{}", typ),
                        Err(e) => println!("{}", e),
                    },
                    Err(e) => println!("{}", e),
                }
            }
        });
    } else if cmd == "--js" {
        let program = fs::read_to_string(path)?;
        let std_ast = crate::baked::load_ast();
        STD.with(|std| {
            let std_ast = std.borrow();
            if let crate::parser::Expression::Apply(items) = &*std_ast {
                match crate::parser::merge_std_and_program(&program, items[1..].to_vec()) {
                    Ok(wrapped_ast) => {
                        let mut code: Vec<crate::vm::Instruction> = Vec::new();
                        let js = crate::js::compile_program_to_js(&wrapped_ast);
                        dump_wrapped_js(js, &format!("{}/main.js", dist));
                    }
                    Err(e) => println!("{}", e),
                }
            }
        });
    
    } else if cmd == "--comp" {
        let path = &args[1];
        let program = fs::read_to_string(path)?;
        STD.with(|std| {
            let std_ast = std.borrow();
            if let crate::parser::Expression::Apply(items) = &*std_ast {
                match crate::parser::merge_std_and_program(&program, items[1..].to_vec()) {
                    Ok(wrapped_ast) => {
                        let mut code: Vec<crate::vm::Instruction> = Vec::new();
                        crate::vm::compile(&wrapped_ast, &mut code);
                        let path = &args[2];
                        dump_raw_bytecode(code, &format!("{}", path));
                    }
                    Err(e) => println!("{}", e),
                }
            }
        });
    } else if cmd == "--exec" {
        let path = &args[1];
        let program = fs::read_to_string(path)?;
        match  crate::vm::exe(parse_bitecode(&program).unwrap(), crate::vm::VM::new()) {
            Ok(e) =>  println!("{:?}", e),
            Err(e) => println!("{:?}", e)
        }
    } else if cmd == "--str" {
        let program = fs::read_to_string(path)?;
        STD.with(|std| {
            let std_ast = std.borrow();
            if let crate::parser::Expression::Apply(items) = &*std_ast {
                match crate::parser::merge_std_and_program(&program, items[1..].to_vec()) {
                    Ok(wrapped_ast) => {
                        let mut code: Vec<crate::vm::Instruction> = Vec::new();
                        crate::vm::compile(&wrapped_ast, &mut code);
                        dump_raw_bytecode(code, &format!("{}/main.txt", dist));
                    }
                    Err(e) => println!("{}", e),
                }
            }
        });
     
    } else if cmd == "--bit" {
        let program = fs::read_to_string(&format!("{}/main.txt", dist))?;
        match  crate::vm::exe(parse_bitecode(&program).unwrap(), crate::vm::VM::new()) {
            Ok(e) =>  println!("{:?}", e),
            Err(e) => println!("{:?}", e)
        }
    } else if cmd == "--doc" {
        let std_ast = crate::baked::load_ast();
        let mut names = Vec::new();
        "+ +# +. - -# -. / /# /. * *# *. mod mod. = =? =# =. < <# <. > ># >. <= <=# <=. >= >=# >=. not and or ^ >> << | & ~ true false car cdr Int->Float Float->Int"
            .split(" ")
            .for_each(|p| {
                let name = p.to_string();
                match crate::infer::infer_with_builtins(&crate::parser::Expression::Word(p.to_string()), 
                crate::types::create_builtin_environment(crate::types::TypeEnv::new())
                ) {
                    Ok(typ) => names.push([name, format!("{}", typ)]),
                    Err(e) => println!("{}", e),
                }
            });
        if let crate::parser::Expression::Apply(items) = &std_ast {
            for expr in items[1..].to_vec() {
                if let crate::parser::Expression::Apply(list) = expr {
                    let a = &list[0];
                    let b = &list[1];
                    if let crate::parser::Expression::Word(kw) = a {
                        if kw == "let" {
                            if let crate::parser::Expression::Word(name) = b {
                                if name.chars().next().unwrap() == '!' {
                                    names.push([name.clone(), "unsafe".to_string()]);
                                } else {
                                    match crate::parser::merge_std_and_program(
                                        &name,
                                        items[1..].to_vec(),
                                    ) {
                                        Ok(p) => match crate::infer::infer_with_builtins(
                                            &p,
                                            crate::types::create_builtin_environment(crate::types::TypeEnv::new())
                                        ) {
                                            Ok(typ) => {
                                                // TODO: use a regex to remove the T+\d+ noise of the files
                                                names.push([name.clone(), format!("{}", typ)])
                                            }
                                            Err(e) => println!("{}", e),
                                        },
                                        Err(e) => println!("{}", e),
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        let path = "./lib.json";
        std::fs::create_dir_all(std::path::Path::new(path).parent().unwrap()).unwrap();
        let mut file = fs::File::create(path)?;
        write!(file, "{:?}", names)?;
    } else if cmd == "--sig" {
        let program = fs::read_to_string(path)?;
        let mut names = Vec::new();
        match crate::baked::load_ast() {
            crate::parser::Expression::Apply(items) => {
                match crate::parser::merge_std_and_program(&program, items.to_vec()) {
                    Ok(ast) => {
                       match infer_with_builtins_env(&ast, crate::types::create_builtin_environment(crate::types::TypeEnv::new())) {
                            Ok(env) => {
                                env
                                    .scopes
                                    .iter()
                                    .for_each(|x| {
                                        for key in x.keys().into_iter().collect::<Vec<_>>() {
                                            names.push([key.to_string(), x.get(key).unwrap().to_string()]);
                                        }
                                    })
                            },
                            Err(_) => {}
                        }
                        let path = "./sig.json";
                        std::fs::create_dir_all(std::path::Path::new(path).parent().unwrap())
                            .unwrap();
                        let mut file = fs::File::create(path)?;
                        write!(file, "{:?}", names)?;
                    }
                    Err(_) => {}
                }
            }
            _ => {}
        }
    } else if cmd == "--fmt" {
      println!("{}", crate::format::format_source(&fs::read_to_string(path)?))
    } else if cmd == "--repl" {
        if args.len() == 6 {
            let path = &args[4];
            #[cfg(feature = "repl")]
            repl(fs::read_to_string(path)?);
        } else {
            #[cfg(feature = "repl")]
            repl(String::new());
        } 
    } else {
        println!("{}", run_code(fs::read_to_string(path)?))
    }

    Ok(())
}
