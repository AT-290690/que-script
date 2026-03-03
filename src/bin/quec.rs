use std::io::Write;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let Some(file_path) = args.get(1) else {
        eprintln!("missing file_path. Usage: que-compiler <script.que> > out.wasm");
        std::process::exit(1);
    };

    let program = std::fs::read_to_string(file_path).unwrap_or_else(|e| {
        eprintln!("\x1b[31mException: failed to read '{}': {}\x1b[0m", file_path, e);
        std::process::exit(1);
    });

    let std_ast = que::baked::load_ast();
    let wrapped_ast = match &std_ast {
        que::parser::Expression::Apply(items) => que::parser
            ::merge_std_and_program(&program, items[1..].to_vec())
            .unwrap_or_else(|e| {
                eprintln!("\x1b[31mException: {}\x1b[0m", e);
                std::process::exit(1);
            }),
        _ => {
            eprintln!("\x1b[31mException: failed to load standard library AST\x1b[0m");
            std::process::exit(1);
        }
    };

    let wat_src = que::wat::compile_program_to_wat(&wrapped_ast).unwrap_or_else(|e| {
        eprintln!("\x1b[31mException: {}\x1b[0m", e);
        std::process::exit(1);
    });

    let wasm_bytes = wat::parse_str(&wat_src).unwrap_or_else(|e| {
        eprintln!("\x1b[31mException: {}\x1b[0m", e);
        std::process::exit(1);
    });

    std::io::stdout().write_all(&wasm_bytes).unwrap_or_else(|e| {
        eprintln!("\x1b[31mException: failed to write wasm to stdout: {}\x1b[0m", e);
        std::process::exit(1);
    });
}
