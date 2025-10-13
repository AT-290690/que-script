#![allow(dead_code)]
#![allow(warnings)]
mod baked;
mod infer;
mod js;
mod parser;
mod types;
mod vm;
use crate::{baked::load_ast, vm::parse_bitecode};
use wasm_bindgen::prelude::wasm_bindgen;
#[wasm_bindgen]
pub fn cons(a: String, b: String) -> String {
    return format!(
        "{:?}",
        parse_bitecode(&a)
            .unwrap()
            .into_iter()
            .chain(parse_bitecode(&b).unwrap().into_iter())
            .collect::<Vec<_>>(),
    );
}
#[wasm_bindgen]
pub fn exec(program: String) -> String {
    match vm::exe(parse_bitecode(&program).unwrap()) {
        Ok(res) => return format!("{:?}", res),
        Err(err) => return err,
    }
}
#[wasm_bindgen]
pub fn comp(program: String) -> String {
    let std_ast = baked::load_ast();
    if let parser::Expression::Apply(items) = &std_ast {
        match parser::merge_std_and_program(&program, items[1..].to_vec()) {
            Ok(wrapped_ast) => {
                let mut code: Vec<vm::Instruction> = Vec::new();
                vm::compile(&wrapped_ast, &mut code);
                return format!("{:?}", code);
            }
            Err(err) => return err,
        }
    }
    "No expressions...".to_string()
}
#[wasm_bindgen]
pub fn run(program: String) -> String {
    let std_ast = baked::load_ast();
    if let parser::Expression::Apply(items) = &std_ast {
        match parser::merge_std_and_program(&program, items[1..].to_vec()) {
            Ok(wrapped_ast) => match infer::infer_with_builtins(&wrapped_ast) {
                Ok(typ) => {
                    return match vm::run(&wrapped_ast) {
                        Ok(res) => return format!("{}\n{:?}", typ, res),
                        Err(err) => return err,
                    }
                }
                Err(err) => return err,
            },
            Err(err) => return err,
        }
    }
    "No expressions...".to_string()
}
#[wasm_bindgen]
pub fn js(program: String) -> String {
    let std_ast = baked::load_ast();
    if let parser::Expression::Apply(items) = &std_ast {
        match parser::merge_std_and_program(&program, items[1..].to_vec()) {
            Ok(wrapped_ast) => {
                let mut code: Vec<vm::Instruction> = Vec::new();
                return js::compile_program_to_js(&wrapped_ast);
            }
            Err(e) => println!("{:?}", e),
        }
    }
    "No expressions...".to_string()
}

#[wasm_bindgen]
pub fn check(program: String) -> String {
    let std_ast = baked::load_ast();
    if let parser::Expression::Apply(items) = &std_ast {
        match parser::merge_std_and_program(&program, items[1..].to_vec()) {
            Ok(wrapped_ast) => match infer::infer_with_builtins(&wrapped_ast) {
                Ok(typ) => return format!("{}", typ),
                Err(e) => return format!("{:?}", e),
            },
            Err(e) => return e,
        }
    }
    "No expressions...".to_string()
}
