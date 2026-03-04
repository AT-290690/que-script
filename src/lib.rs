pub mod baked;
pub mod infer;
pub mod parser;
#[cfg(feature = "runtime")]
pub mod runtime;
#[cfg(feature = "io")]
pub mod io;
#[cfg(test)]
mod tests;
pub mod types;
#[cfg(feature = "compiler")]
pub mod wat;
pub mod wasm_api;
