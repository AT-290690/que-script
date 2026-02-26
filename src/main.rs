use crate::cli::cli;

mod baked;
mod cli;
mod format;
mod infer;
#[cfg(feature = "js-compiler")]
#[path = "compilers/js.rs"]
mod js;
#[cfg(feature = "kotlin-compiler")]
#[path = "compilers/kotlin.rs"]
mod kotlin;
#[cfg(feature = "ocaml-compiler")]
#[path = "compilers/ocaml.rs"]
mod ocaml;
mod parser;
#[cfg(feature = "python-compiler")]
#[path = "compilers/py.rs"]
mod py;
#[cfg(feature = "qir")]
#[path = "compilers/qir.rs"]
mod qir;
mod repl;
mod report;
#[cfg(feature = "rust-compiler")]
#[path = "compilers/rs.rs"]
mod rs;
mod tests;
mod types;
mod vm;
#[cfg(feature = "wasm-compiler")]
#[path = "compilers/wat.rs"]
mod wat;
fn main() {
    cli("./example").unwrap();
}
