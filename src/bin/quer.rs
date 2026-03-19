fn main() {
    let args: Vec<String> = std::env::args().collect();
    let Some(file_path) = args.get(1) else {
        eprintln!("missing file_path. Usage: que-runtime <script.que> [arg ...]");
        std::process::exit(1);
    };
    let argv: Vec<String> = args.iter().skip(2).cloned().collect();

    let program = std::fs::read_to_string(file_path).unwrap_or_else(|e| {
        eprintln!(
            "\x1b[31mException: failed to read '{}': {}\x1b[0m",
            file_path, e
        );
        std::process::exit(1);
    });

    let std_ast = que::baked::load_ast();
    let wrapped_ast = match &std_ast {
        que::parser::Expression::Apply(items) => {
            que::parser::merge_std_and_program(&program, items[1..].to_vec()).unwrap_or_else(|e| {
                eprintln!("\x1b[31mException: {}\x1b[0m", e);
                std::process::exit(1);
            })
        }
        _ => {
            eprintln!("\x1b[31mException: failed to load standard library AST\x1b[0m");
            std::process::exit(1);
        }
    };

    let wat_src = que::wat::compile_program_to_wat(&wrapped_ast).unwrap_or_else(|e| {
        eprintln!("\x1b[31mException: {}\x1b[0m", e);
        std::process::exit(1);
    });

    if wat_requires_host_io(&wat_src) {
        eprintln!(
            "\x1b[31mException: this program requires host io functions. Use 'queio' (with --allow ...) or build/run 'quer' without the 'io' feature.\x1b[0m"
        );
        std::process::exit(1);
    }

    let decoded =
        que::runtime::run_wat_text(&wat_src, (), &argv, |_linker| Ok(())).unwrap_or_else(|e| {
            eprintln!("\x1b[31mException: {}\x1b[0m", e);
            std::process::exit(1);
        });

    println!("\x1b[32m{}\x1b[0m", decoded);
}

fn wat_requires_host_io(wat_src: &str) -> bool {
    wat_src.contains("(import \"host\" \"")
}
