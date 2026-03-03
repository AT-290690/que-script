mod baked;
mod infer;
mod parser;
#[cfg(feature = "runtime")]
mod runtime;
#[cfg(feature = "io")]
mod shell;
mod tests;
mod types;
#[cfg(feature = "compiler")]
mod wat;

#[cfg(feature = "io")]
fn main() {
    if let Err(err) = crate::shell::run_native_shell() {
        eprintln!("\x1b[31mException: {}\x1b[0m", err);
        std::process::exit(1);
    }
}

#[cfg(not(feature = "io"))]
#[cfg(feature = "compiler")]
fn main() {
    let args: Vec<String> = std::env::args().collect();
    let Some(file_path) = args.get(1) else {
        eprintln!("missing file_path. Usage: que <script.que> [out.wasm]");
        std::process::exit(1);
    };
    let out_path = args
        .get(2)
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| {
            let path = std::path::Path::new(file_path);
            match path.file_stem() {
                Some(stem) => {
                    let mut out = path.with_file_name(stem);
                    out.set_extension("wasm");
                    out
                }
                None => std::path::PathBuf::from("out.wasm"),
            }
        });
    let program = std::fs::read_to_string(file_path).unwrap_or_else(|e| {
        eprintln!("\x1b[31mException: failed to read '{}': {}\x1b[0m", file_path, e);
        std::process::exit(1);
    });

    let std_ast = crate::baked::load_ast();
    let wrapped_ast = match &std_ast {
        crate::parser::Expression::Apply(items) =>
            crate::parser::merge_std_and_program(&program, items[1..].to_vec()).unwrap_or_else(|e| {
                eprintln!("\x1b[31mException: {}\x1b[0m", e);
                std::process::exit(1);
            }),
        _ => {
            eprintln!("\x1b[31mException: failed to load standard library AST\x1b[0m");
            std::process::exit(1);
        }
    };

    let wat_src = crate::wat::compile_program_to_wat(&wrapped_ast).unwrap_or_else(|e| {
        eprintln!("\x1b[31mException: {}\x1b[0m", e);
        std::process::exit(1);
    });
    let wasm_bytes = ::wat::parse_str(&wat_src).unwrap_or_else(|e| {
        eprintln!("\x1b[31mException: {}\x1b[0m", e);
        std::process::exit(1);
    });
    std::fs::write(&out_path, wasm_bytes).unwrap_or_else(|e| {
        eprintln!("\x1b[31mException: failed to write '{}': {}\x1b[0m", out_path.display(), e);
        std::process::exit(1);
    });
    println!("\x1b[32mwrote {}\x1b[0m", out_path.display());
}

#[cfg(not(feature = "io"))]
#[cfg(not(feature = "compiler"))]
fn main() {
    eprintln!("no executable mode enabled. Use --features compiler or --features io");
    std::process::exit(1);
}
