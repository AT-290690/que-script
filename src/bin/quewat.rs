fn main() {
    let args: Vec<String> = std::env::args().collect();
    let Some(first_arg) = args.get(1) else {
        eprintln!("missing input. Usage: quewat <script.que> > out.wat");
        eprintln!("or:    quewat --eval '<source>' > out.wat");
        std::process::exit(1);
    };

    let program = if first_arg == "--eval" || first_arg == "-e" {
        let Some(source) = args.get(2) else {
            eprintln!("\x1b[31mException: --eval requires source text\x1b[0m");
            std::process::exit(1);
        };
        source.clone()
    } else {
        std::fs::read_to_string(first_arg).unwrap_or_else(|e| {
            eprintln!("\x1b[31mException: failed to read '{}': {}\x1b[0m", first_arg, e);
            std::process::exit(1);
        })
    };

    let std_ast = que::baked::load_ast();
    let lib_defs = que::baked::ast_to_definitions(std_ast, "active library").unwrap_or_else(|e| {
        eprintln!("\x1b[31mException: {}\x1b[0m", e);
        std::process::exit(1);
    });
    let wrapped_ast = que::parser::merge_std_and_program(&program, lib_defs).unwrap_or_else(|e| {
        eprintln!("\x1b[31mException: {}\x1b[0m", e);
        std::process::exit(1);
    });

    let wat_src = que::wat::compile_program_to_wat(&wrapped_ast).unwrap_or_else(|e| {
        eprintln!("\x1b[31mException: {}\x1b[0m", e);
        std::process::exit(1);
    });

    println!("{}", wat_src);
}
