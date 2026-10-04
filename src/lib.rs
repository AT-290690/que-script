pub mod baked;
#[cfg(feature = "compiler")]
pub mod compiler_cli;
#[cfg(feature = "compiler")]
pub mod explain;
#[path = "../miscs/formatter.rs"]
pub mod formatter;
pub mod lsp_native_core;
pub mod op;
pub mod project;
pub mod static_analysis;
// The repository-wide integration suite exercises the WAT compiler and its
// optional `wat` dependency. Module-local parser, LSP, and analysis tests still
// run without default features; only this compiler-specific suite is gated.
#[cfg(all(test, feature = "compiler"))]
mod tests;
pub mod wasm_api;
#[cfg(feature = "compiler")]
pub mod wat;

pub use eclisp::{externals, infer, parser, types};
