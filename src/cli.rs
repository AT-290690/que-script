#![allow(dead_code)]
#![allow(warnings)]

use crate::baked::load_ast;
use crate::ir::load_bytecode;
use crate::vm::parse_bitecode;
use std::env;
use std::fs;
use std::io::Write;
use std::num::Wrapping;

fn run_code(program: String) -> String {
    let std_ast = crate::baked::load_ast();
    if let crate::parser::Expression::Apply(items) = &std_ast {
        match crate::parser::merge_std_and_program(&program, items[1..].to_vec()) {
            Ok(wrapped_ast) => {
                match crate::infer::infer_with_builtins(&wrapped_ast, crate::infer::create_builtin_environment(crate::types::TypeEnv::new())) {
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
}
fn dump_wrapped_ast(expr: crate::parser::Expression, path: &str) -> std::io::Result<()> {
    let mut file = fs::File::create(path)?;
    writeln!(file, "use crate::parser::Expression::*;")?;
    writeln!(file, "macro_rules! s {{($s:expr) => {{ $s.to_string() }}}}")?;
    writeln!(file, "pub fn load_ast() -> crate::parser::Expression {{")?;
    writeln!(file, "{}", expr.to_rust())?;
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
    if args.iter().any(|a| a == "--std") {
        let std_src = fs::read_to_string("./lisp/std.lisp")?;
        let std_ast = crate::parser::build(&std_src).unwrap();
        dump_wrapped_ast(std_ast, "./src/baked.rs");
    } else if args.iter().any(|a| a == "--check") {
        let program = fs::read_to_string(path)?;
        let std_ast = crate::baked::load_ast();
        if let crate::parser::Expression::Apply(items) = &std_ast {
            match crate::parser::merge_std_and_program(&program, items[1..].to_vec()) {
                Ok(wrapped_ast) => match crate::infer::infer_with_builtins(
                    &wrapped_ast,
                    crate::infer::create_builtin_environment(crate::types::TypeEnv::new())
                ) {
                    Ok(typ) => println!("{}", typ),
                    Err(e) => println!("{}", e),
                },
                Err(e) => println!("{}", e),
            }
        }
    } else if args.iter().any(|a| a == "--js") {
        let program = fs::read_to_string(path)?;
        let std_ast = crate::baked::load_ast();
        if let crate::parser::Expression::Apply(items) = &std_ast {
            match crate::parser::merge_std_and_program(&program, items[1..].to_vec()) {
                Ok(wrapped_ast) => {
                    let mut code: Vec<crate::vm::Instruction> = Vec::new();
                    let js = crate::js::compile_program_to_js(&wrapped_ast);
                    dump_wrapped_js(js, &format!("{}/main.js", dist));
                }
                Err(e) => println!("{}", e),
            }
        }
    } else if args.iter().any(|a| a == "--str") {
        let program = fs::read_to_string(path)?;
        let std_ast = crate::baked::load_ast();

        if let crate::parser::Expression::Apply(items) = &std_ast {
            match crate::parser::merge_std_and_program(&program, items[1..].to_vec()) {
                Ok(wrapped_ast) => {
                    let mut code: Vec<crate::vm::Instruction> = Vec::new();
                    crate::vm::compile(&wrapped_ast, &mut code);
                    dump_raw_bytecode(code, &format!("{}/main.txt", dist));
                }
                Err(e) => println!("{}", e),
            }
        }
    } else if args.iter().any(|a| a == "--bit") {
        let program = fs::read_to_string(&format!("{}/main.txt", dist))?;
        println!(
            "{:?}",
            crate::vm::exe(parse_bitecode(&program).unwrap(), crate::vm::VM::new())
        );
    } else if args.iter().any(|a| a == "--doc") {
        let std_ast = crate::baked::load_ast();
        let mut names = Vec::new();
        "+ +# +. - -# -. / /# /. * *# *. mod mod. = =? =# =. < <# <. > ># >. <= <=# <=. >= >=# >=. not and or ^ >> << | & ~ true false Int->Float Float->Int"
            .split(" ")
            .for_each(|p| {
                let name = p.to_string();
                match crate::infer::infer_with_builtins(&crate::parser::Expression::Word(p.to_string()), 
                crate::infer::create_builtin_environment(crate::types::TypeEnv::new())
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
                                            crate::infer::create_builtin_environment(crate::types::TypeEnv::new())
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
    } else if args.iter().any(|a| a == "--sig") {
        let program = fs::read_to_string(path)?;
        let mut names = Vec::new();
        match crate::baked::load_ast() {
            crate::parser::Expression::Apply(items) => {
                match crate::parser::merge_std_and_program(&program, items.to_vec()) {
                    Ok(ast) => {
                        if let crate::parser::Expression::Apply(inner) = &ast {
                            for expr in inner[1..].to_vec() {
                                if let crate::parser::Expression::Apply(list) = expr {
                                    let a = &list[0];
                                    let b = &list[1];
                                    if let crate::parser::Expression::Word(kw) = a {
                                        if kw == "let" {
                                            if let crate::parser::Expression::Word(name) = b {
                                                let E = inner
                                                    .to_vec()
                                                    .iter()
                                                    .chain(
                                                        vec![crate::parser::Expression::Word(
                                                            name.to_string(),
                                                        )]
                                                        .iter(),
                                                    )
                                                    .into_iter()
                                                    .map(|e| e.clone())
                                                    .collect::<Vec<_>>();
                                                match crate::infer::infer_with_builtins(
                                                    &crate::parser::Expression::Apply(E),
                                                    crate::infer::create_builtin_environment(crate::types::TypeEnv::new())
                                                ) {
                                                    Ok(typ) => {
                                                        // TODO: use a regex to remove the T+\d+ noise of the files
                                                        names.push([
                                                            name.clone(),
                                                            format!("{}", typ),
                                                        ])
                                                    }
                                                    Err(e) => {}
                                                }
                                            }
                                        }
                                    }
                                }
                            }
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
    } else {
        println!("{}", run_code(fs::read_to_string(path)?))
    }

    Ok(())
}
