use crate::cli::cli;

mod baked;
mod cli;
mod infer;
mod ir;
mod js;
mod parser;
mod tests;
mod types;
mod vm;

fn main() {
    cli("./example").unwrap();
}
