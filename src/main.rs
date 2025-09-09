#![allow(dead_code)]
#![allow(warnings)]

mod infer;
mod lisp;
mod types;
mod vm;
use std::fs;
use std::io::Write;
mod ast;
use ast::load_ast;
use std::env;
mod tests;

fn dump_wrapped_ast(expr: lisp::Expression, path: &str) -> std::io::Result<()> {
    let mut file = fs::File::create(path)?;
    writeln!(file, "use crate::lisp::Expression;")?;
    writeln!(file, "pub fn load_ast() -> Expression {{")?;
    writeln!(file, "    {}", expr.to_rust())?;
    writeln!(file, "}}")?;
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

        let wrapped_ast = lisp::with_std(&program, &std_lib);

        match infer::infer_with_builtins(&wrapped_ast) {
            Ok(typ) => {
                println!("Type: {}", typ);
                dump_wrapped_ast(wrapped_ast, "./src/ast.rs");
            }
            Err(e) => println!("Error: {}", e),
        }
    } else {
        let wrapped_ast: lisp::Expression = load_ast();

        // match infer::infer_with_builtins(&wrapped_ast) {
        //     Ok(typ) => println!("Type: {}", typ),
        //     Err(e) => println!("Error: {}", e),
        // }
        // println!("{:?}", lisp::run(&wrapped_ast));
        let mut code = Vec::new();
        vm::compile(&wrapped_ast, &mut code);
        // Run
        let mut vm = vm::VM::new();
        vm.run(&code);
        println!(
            "{:?}",
            vm.result().unwrap_or(&vm::BiteCodeEvaluated::Int(0))
        );
    }

    Ok(())
}
