#![allow(dead_code)]
#![allow(warnings)]
mod infer;
mod parser;
mod types;
mod vm;
use std::fs;
use std::io::Write;
use std::num::Wrapping;
mod ast;
mod ir;
use ast::load_ast;
use ir::load_bytecode;
use std::env;
mod tests;

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
    writeln!(file, "macro_rules! s {{($s:expr) => {{ $s.to_string() }}}}")?;
    writeln!(
        file,
        "pub fn load_bytecode() -> Vec<crate::vm::Instruction> {{"
    )?;
    write!(file, "vec![")?;

    for instr in code {
        write!(file, "{},", instr.to_rust())?;
    }

    writeln!(file, "]")?;
    write!(file, "}}")?;
    Ok(())
}

fn main() -> std::io::Result<()> {
    // let lambda_exprs =
    //     lisp::parse("(do (let fn (lambda xs (do (let x (get xs 0)) x))) (fn (array (= 1 1) )))")
    //         .unwrap();
    // if let Some(lambda_expr) = lambda_exprs.first() {
    //     match infer::infer_with_builtins(lambda_expr) {
    //         Ok(typ) => println!("Type of : {}", typ),
    //         Err(e) => println!("Type error : {}", e),
    //     }
    // }

    let args: Vec<String> = env::args().collect();
    if args.iter().any(|a| a == "--dump") {
        let program = fs::read_to_string("./lisp/main.lisp")?;
        let std_lib = fs::read_to_string("./lisp/std.lisp")?;

        let wrapped_ast = parser::with_std(&program, &std_lib);

        match infer::infer_with_builtins(&wrapped_ast) {
            Ok(typ) => {
                println!("{}", typ);
                dump_wrapped_ast(wrapped_ast, "./src/ast.rs");
            }
            Err(e) => println!("Error: {}", e),
        }
    } else if args.iter().any(|a| a == "--comp") {
        let program = fs::read_to_string("./lisp/main.lisp")?;
        let std_lib = fs::read_to_string("./lisp/std.lisp")?;
        let wrapped_ast = parser::with_std(&program, &std_lib);
        let mut code: Vec<vm::Instruction> = Vec::new();
        vm::compile(&wrapped_ast, &mut code);
        dump_wrapped_bytecode(code, "./src/ir.rs");
    } else if args.iter().any(|a| a == "--exec") {
        let bitecode = ir::load_bytecode();
        println!("{:?}", vm::exe(bitecode));
    } else {
        let program = fs::read_to_string("./lisp/main.lisp")?;
        let std_lib = fs::read_to_string("./lisp/std.lisp")?;

        let wrapped_ast = parser::with_std(&program, &std_lib);

        match infer::infer_with_builtins(&wrapped_ast) {
            Ok(typ) => {
                println!("{}", typ);
                println!("{:?}", vm::run(&wrapped_ast))
            }
            Err(e) => println!("Error: {}", e),
        }
    }

    Ok(())
}
