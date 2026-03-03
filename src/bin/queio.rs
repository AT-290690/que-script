fn main() {
    if let Err(err) = que::io::run_native_shell() {
        eprintln!("\x1b[31mException: {}\x1b[0m", err);
        std::process::exit(1);
    }
}
