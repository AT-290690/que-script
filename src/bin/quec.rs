fn main() {
    if let Err(error) = que::compiler_cli::run() {
        eprintln!("\x1b[31mException: {error}\x1b[0m");
        std::process::exit(1);
    }
}
