mod lisp;
use std::fs;
use std::io::Write;
mod ast;
use ast::load_ast;
use std::env;

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
        println!("{:?}", lisp::run(&wrapped_ast));
    }

    Ok(())
}
