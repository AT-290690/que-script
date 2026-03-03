mod baked;
mod infer;
mod parser;
mod shell;
mod tests;
mod types;
mod wat;

fn main() {
    if let Err(err) = crate::shell::run_native_shell() {
        eprintln!("\x1b[31mException: {}\x1b[0m", err);
        std::process::exit(1);
    }
}
