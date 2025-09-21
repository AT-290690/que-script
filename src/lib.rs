#![allow(dead_code)]
#![allow(warnings)]
mod baked;
mod infer;
mod parser;
mod types;
mod vm;
use crate::baked::load_ast;
mod tests;
use wasm_bindgen::prelude::wasm_bindgen;
#[wasm_bindgen]
pub fn run(program: &str) -> String {
    let std_ast = baked::load_ast();
    if let parser::Expression::Apply(items) = &std_ast {
        let wrapped_ast = parser::merge_std_and_program(&program, items[1..].to_vec());
        match infer::infer_with_builtins(&wrapped_ast) {
            Ok(typ) => return format!("{}\n{:?}", typ, vm::run(&wrapped_ast)),
            Err(e) => return format!("{:?}", e),
        }
    }
    "No expressions...".to_string()
}
