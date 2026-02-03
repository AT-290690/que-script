use crate::cli::cli;

mod baked;
mod cli;
mod format;
mod infer;
mod ir;
mod js;
mod parser;
mod repl;
mod tests;
mod types;
mod vm;
fn main() {
    cli("./example").unwrap();
}
