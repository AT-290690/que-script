use crate::cli::cli;

mod baked;
mod cli;
mod format;
mod infer;
mod ir;
#[cfg(feature = "js-compiler")]
mod js;
#[cfg(feature = "ocaml-compiler")]
mod ocaml;
#[cfg(feature = "python-compiler")]
mod py;
#[cfg(feature = "kotlin-compiler")]
mod kotlin;
#[cfg(feature = "rust-compiler")]
mod rs;
#[cfg(feature = "wasm-compiler")]
mod wat;
mod parser;
mod repl;
mod report;
mod tests;
mod types;
mod vm;
fn main() {
    cli("./example").unwrap();
}
