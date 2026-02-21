use crate::cli::cli;

mod baked;
mod cli;
mod format;
mod infer;
mod ir;
#[cfg(feature = "js-compiler")]
#[path = "compilers/js.rs"]
mod js;
#[cfg(feature = "ocaml-compiler")]
#[path = "compilers/ocaml.rs"]
mod ocaml;
#[cfg(feature = "python-compiler")]
#[path = "compilers/py.rs"]
mod py;
#[cfg(feature = "kotlin-compiler")]
#[path = "compilers/kotlin.rs"]
mod kotlin;
#[cfg(feature = "rust-compiler")]
#[path = "compilers/rs.rs"]
mod rs;
#[cfg(feature = "wasm-compiler")]
#[path = "compilers/wat.rs"]
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
