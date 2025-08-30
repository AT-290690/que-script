#![allow(dead_code)]
#![allow(warnings)]

mod infer;
mod lisp;
mod types;
use std::fs;
use std::io::Write;
mod ast;
use ast::load_ast;
use std::env;

fn test_type_inference() {
    // Test type inference
    println!("\n=== Type Inference Examples ===");

    // Test simple expressions
    let simple_exprs = lisp::parse("(+ 1 2)").unwrap();
    if let Some(simple_expr) = simple_exprs.first() {
        match infer::infer_with_builtins(simple_expr) {
            Ok(typ) => println!("Type of (+ 1 2): {}", typ),
            Err(e) => println!("Type error in '(+ 1 2)': {}", e),
        }
    }

    // Test lambda
    let lambda_exprs = lisp::parse("(lambda x (+ x 1))").unwrap();
    if let Some(lambda_expr) = lambda_exprs.first() {
        match infer::infer_with_builtins(lambda_expr) {
            Ok(typ) => println!("Type of (lambda x (+ x 1)): {}", typ),
            Err(e) => println!("Type error in '(lambda x (+ x 1))': {}", e),
        }
    }

    let lambda_exprs = lisp::parse("(lambda x (and x 42))").unwrap();
    if let Some(lambda_expr) = lambda_exprs.first() {
        match infer::infer_with_builtins(lambda_expr) {
            Ok(typ) => println!("Type of (lambda x (and x 42)): {}", typ),
            Err(e) => println!("Type error in '(lambda x (and x 42))': {}", e),
        }
    }

    let lambda_exprs = lisp::parse("(lambda x (and x (or x x)))").unwrap();
    if let Some(lambda_expr) = lambda_exprs.first() {
        match infer::infer_with_builtins(lambda_expr) {
            Ok(typ) => println!("Type of (lambda x (and x (or x x))): {}", typ),
            Err(e) => println!("Type error in '(lambda x (and x (or x x)))': {}", e),
        }
    }

    let lambda_exprs = lisp::parse("(let fn (lambda a b (and a b))) (fn 42 69)").unwrap();
    if let Some(lambda_expr) = lambda_exprs.first() {
        match infer::infer_with_builtins(lambda_expr) {
            Ok(typ) => println!(
                "Type of (let fn (lambda a b (and a b))) (fn 42 69): {}",
                typ
            ),
            Err(e) => println!(
                "Type error in '(let fn (lambda a b (and a b))) (fn 42 69)': {}",
                e
            ),
        }
    }

    let lambda_exprs =
        lisp::parse("(let process (lambda xs (get xs 0))) (process [ 1 2 3 ])").unwrap();
    if let Some(lambda_expr) = lambda_exprs.first() {
        match infer::infer_with_builtins(lambda_expr) {
            Ok(typ) => println!(
                "Type of (let process (lambda xs (get xs 0))) (process [ 1 2 3 ]): {}",
                typ
            ),
            Err(e) => println!(
                "Type error in '(let process (lambda xs (get xs 0))) (process [ 1 2 3 ])': {}",
                e
            ),
        }
    }
    // Test your range function (without pipe operator)
    let range_exprs = lisp::parse("(summation (range 1 10))").unwrap();
    if let Some(range_expr) = range_exprs.first() {
        match infer::infer_with_builtins(range_expr) {
            Ok(typ) => println!("Type of (summation (range 1 10)): {}", typ),
            Err(e) => println!("Type error in '(summation (range 1 10))': {}", e),
        }
    }

    // Test array creation
    let array_exprs = lisp::parse("(array 1 2 3)").unwrap();
    if let Some(array_expr) = array_exprs.first() {
        match infer::infer_with_builtins(array_expr) {
            Ok(typ) => println!("Type of (array 1 2 3): {}", typ),
            Err(e) => println!("Type error in '(array 1 2 3)': {}", e),
        }
    }

    // Test array creation
    let array_exprs = lisp::parse("(array (array (array 1)))").unwrap();
    if let Some(array_expr) = array_exprs.first() {
        match infer::infer_with_builtins(array_expr) {
            Ok(typ) => println!("Type of (array (array (array 1))): {}", typ),
            Err(e) => println!("Type error in '(array (array (array 1)))': {}", e),
        }
    }

    // Test if expression
    let if_expression = lisp::parse("(if (= 1 2) 10 (= 0 1))").unwrap();
    if let Some(if_expr) = if_expression.first() {
        match infer::infer_with_builtins(if_expr) {
            Ok(typ) => println!("Type of (if (= 1 2) 10 (= 0 1)): {}", typ),
            Err(e) => println!("Type error in '(if (= 1 2) 10 (= 0 1))': {}", e),
        }
    }
}

fn dump_wrapped_ast(expr: lisp::Expression, path: &str) -> std::io::Result<()> {
    let mut file = fs::File::create(path)?;
    writeln!(file, "use crate::lisp::Expression;")?;
    writeln!(file, "pub fn load_ast() -> Expression {{")?;
    writeln!(file, "    {}", expr.to_rust())?;
    writeln!(file, "}}")?;
    Ok(())
}

fn main() -> std::io::Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.iter().any(|a| a == "--dump") {
        let program = fs::read_to_string("./lisp/main.lisp")?;
        let std_lib = fs::read_to_string("./lisp/std.lisp")?;
        let _ = dump_wrapped_ast(lisp::with_std(&program, &std_lib), "./src/ast.rs");
    } else {
        let wrapped_ast: lisp::Expression = load_ast();
        match infer::infer_with_builtins(&wrapped_ast) {
            Ok(typ) => println!("Type: {}", typ),
            Err(e) => println!("Error: {}", e),
        }
        println!("{:?}", lisp::run(&wrapped_ast));

        // test_type_inference();
    }

    Ok(())
}
