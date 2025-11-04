#![allow(dead_code)]
#![allow(warnings)]
mod infer;
mod parser;
mod types;
mod vm;
use std::fs;
use std::io::Write;
use std::num::Wrapping;
mod baked;
mod ir;
use crate::vm::parse_bitecode;
use baked::load_ast;
use ir::load_bytecode;
use std::env;
mod js;
mod tests;
fn run_code(program: String) -> String {
    let std_ast = baked::load_ast();
    if let parser::Expression::Apply(items) = &std_ast {
        match parser::merge_std_and_program(&program, items[1..].to_vec()) {
            Ok(wrapped_ast) => match infer::infer_with_builtins(&wrapped_ast) {
                Ok(typ) => {
                    return match vm::run(&wrapped_ast) {
                        Ok(res) => return format!("{}\n{:?}", typ, res),
                        Err(err) => return err,
                    }
                }
                Err(err) => return err,
            },
            Err(err) => return err,
        }
    }
    "No expressions...".to_string()
}
fn dump_wrapped_ast(expr: parser::Expression, path: &str) -> std::io::Result<()> {
    let mut file = fs::File::create(path)?;
    writeln!(file, "use crate::parser::Expression::*;")?;
    writeln!(file, "macro_rules! s {{($s:expr) => {{ $s.to_string() }}}}")?;
    writeln!(file, "pub fn load_ast() -> crate::parser::Expression {{")?;
    writeln!(file, "{}", expr.to_rust())?;
    writeln!(file, "}}")?;
    Ok(())
}

pub fn dump_wrapped_bytecode(code: Vec<vm::Instruction>, path: &str) -> std::io::Result<()> {
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

pub fn dump_raw_bytecode(code: Vec<vm::Instruction>, path: &str) -> std::io::Result<()> {
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
fn main() -> std::io::Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.iter().any(|a| a == "--std") {
        let std_src = fs::read_to_string("./lisp/std.lisp")?;
        let std_ast = parser::build(&std_src).unwrap();
        dump_wrapped_ast(std_ast, "./src/baked.rs");
    } else if args.iter().any(|a| a == "--comp") {
        let program = fs::read_to_string("./lisp/main.lisp")?;
        let std_ast = baked::load_ast();

        if let parser::Expression::Apply(items) = &std_ast {
            match parser::merge_std_and_program(&program, items[1..].to_vec()) {
                Ok(wrapped_ast) => {
                    let mut code: Vec<vm::Instruction> = Vec::new();
                    vm::compile(&wrapped_ast, &mut code);
                    dump_wrapped_bytecode(code, "./src/ir.rs");
                }
                Err(e) => println!("{}", e),
            }
        }
    } else if args.iter().any(|a| a == "--exec") {
        let bitecode = ir::load_bytecode();
        println!("{:?}", vm::exe(bitecode));
    } else if args.iter().any(|a| a == "--check") {
        let program = fs::read_to_string("./lisp/main.lisp")?;
        let std_ast = baked::load_ast();
        if let parser::Expression::Apply(items) = &std_ast {
            match parser::merge_std_and_program(&program, items[1..].to_vec()) {
                Ok(wrapped_ast) => match infer::infer_with_builtins(&wrapped_ast) {
                    Ok(typ) => println!("{}", typ),
                    Err(e) => println!("{}", e),
                },
                Err(e) => println!("{}", e),
            }
        }
    } else if args.iter().any(|a| a == "--js") {
        let program = fs::read_to_string("./lisp/main.lisp")?;
        let std_ast = baked::load_ast();
        if let parser::Expression::Apply(items) = &std_ast {
            match parser::merge_std_and_program(&program, items[1..].to_vec()) {
                Ok(wrapped_ast) => {
                    let mut code: Vec<vm::Instruction> = Vec::new();
                    let js = js::compile_program_to_js(&wrapped_ast);
                    dump_wrapped_js(js, "./dist/index.js");
                }
                Err(e) => println!("{}", e),
            }
        }
    } else if args.iter().any(|a| a == "--str") {
        let program = fs::read_to_string("./lisp/main.lisp")?;
        let std_ast = baked::load_ast();

        if let parser::Expression::Apply(items) = &std_ast {
            match parser::merge_std_and_program(&program, items[1..].to_vec()) {
                Ok(wrapped_ast) => {
                    let mut code: Vec<vm::Instruction> = Vec::new();
                    vm::compile(&wrapped_ast, &mut code);
                    dump_raw_bytecode(code, "./dist/ir.txt");
                }
                Err(e) => println!("{}", e),
            }
        }
    } else if args.iter().any(|a| a == "--bit") {
        let program = fs::read_to_string("./dist/ir.txt")?;
        println!("{:?}", vm::exe(parse_bitecode(&program).unwrap()));
    } else if args.iter().any(|a| a == "--doc") {
        let std_ast = baked::load_ast();
        let mut names = Vec::new();
        "+ +# - -# / /# * *# mod = =? =# < <# > ># <= <=# >= >=# not and or ^ >> << | & ~ true false"
            .split(" ")
            .for_each(|p| {
                let name = p.to_string();
                match infer::infer_with_builtins(&parser::Expression::Word(p.to_string())) {
                    Ok(typ) => names.push([name, format!("{}", typ)]),
                    Err(e) => println!("{}", e),
                }
            });
        if let parser::Expression::Apply(items) = &std_ast {
            let std_items = items[1..].to_vec();
            for expr in items[1..].to_vec() {
                if let parser::Expression::Apply(list) = expr {
                    let a = &list[0];
                    let b = &list[1];
                    if let parser::Expression::Word(kw) = a {
                        if kw == "let" {
                            if let parser::Expression::Word(name) = b {
                                if name.chars().next().unwrap() == '!' {
                                    names.push([name.clone(), "unsafe".to_string()]);
                                } else {
                                    match parser::merge_std_and_program(&name, items[1..].to_vec())
                                    {
                                        Ok(p) => match infer::infer_with_builtins(&p) {
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
    } else {
        let program = fs::read_to_string("./dist/ir.txt")?;
        println!("{}", run_code(fs::read_to_string("./lisp/main.lisp")?))
    }

    Ok(())
}
