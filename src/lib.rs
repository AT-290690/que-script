#![allow(dead_code)]
#![allow(warnings)]
mod baked;
mod infer;
mod js;
mod parser;
mod types;
mod vm;
use crate::{baked::load_ast, vm::parse_bitecode};
use std::cell::RefCell;
use wasm_bindgen::prelude::wasm_bindgen;

use std::io::Write;

thread_local! {
    static OUTPUT: RefCell<Vec<u8>> = RefCell::new(Vec::new());
}

fn write_to_output(s: &str) -> *const u8 {
    OUTPUT.with(|buf| {
        let mut buf = buf.borrow_mut();
        buf.clear();
        buf.write_all(s.as_bytes()).unwrap();
        buf.as_ptr()
    })
}

#[wasm_bindgen]
pub fn get_output_ptr() -> *const u8 {
    OUTPUT.with(|buf| buf.borrow().as_ptr())
}

#[wasm_bindgen]
pub fn get_output_len() -> usize {
    OUTPUT.with(|buf| buf.borrow().len())
}

#[wasm_bindgen]
pub fn evaluate(program: String) -> *const u8 {
    let std_ast = baked::load_ast();
    let result = if let parser::Expression::Apply(items) = &std_ast {
        match parser::merge_std_and_program(&program, items[1..].to_vec()) {
            Ok(wrapped_ast) => match infer::infer_with_builtins(&wrapped_ast) {
                Ok(typ) => match vm::run(&wrapped_ast) {
                    Ok(res) => format!("0\n{}\n{:?}", typ, res),
                    Err(err) => format!("2\n{}", err),
                },
                Err(err) => format!("1\n{}", err),
            },
            Err(err) => format!("3\n{}", err),
        }
    } else {
        "0 No expressions...".to_string()
    };

    write_to_output(&result)
}

#[wasm_bindgen]
pub fn run(program: String) -> *const u8 {
    let std_ast = baked::load_ast();
    let result = if let parser::Expression::Apply(items) = &std_ast {
        match parser::merge_std_and_program(&program, items[1..].to_vec()) {
            Ok(wrapped_ast) => match vm::run(&wrapped_ast) {
                Ok(res) => format!("0\n{:?}", res),
                Err(err) => format!("2\n{}", err),
            },
            Err(err) => format!("1\n{}", err),
        }
    } else {
        "No expressions...".to_string()
    };

    write_to_output(&result)
}

#[wasm_bindgen]
pub fn js(program: String) -> *const u8 {
    let std_ast = baked::load_ast();
    let result = if let parser::Expression::Apply(items) = &std_ast {
        match parser::merge_std_and_program(&program, items[1..].to_vec()) {
            Ok(wrapped_ast) => js::compile_program_to_js(&wrapped_ast),
            Err(err) => format!("3\n{:?}", err),
        }
    } else {
        "No expressions...".to_string()
    };

    write_to_output(&result)
}

#[wasm_bindgen]
pub fn check(program: String) -> *const u8 {
    let std_ast = baked::load_ast();
    let result = if let parser::Expression::Apply(items) = &std_ast {
        match parser::merge_std_and_program(&program, items[1..].to_vec()) {
            Ok(wrapped_ast) => match infer::infer_with_builtins(&wrapped_ast) {
                Ok(typ) => format!("0\n{}", typ),
                Err(err) => format!("1\n{}", err),
            },
            Err(err) => format!("3\n{}", err),
        }
    } else {
        "No expressions...".to_string()
    };

    write_to_output(&result)
}

#[wasm_bindgen]
pub fn exec(program: String) -> *const u8 {
    let result = match vm::exe(parse_bitecode(&program).unwrap()) {
        Ok(res) => format!("0\n{:?}", res),
        Err(err) => format!("2\n{}", err),
    };

    write_to_output(&result)
}

#[wasm_bindgen]
pub fn comp(program: String) -> *const u8 {
    let std_ast = baked::load_ast();
    let result = if let parser::Expression::Apply(items) = &std_ast {
        match parser::merge_std_and_program(&program, items[1..].to_vec()) {
            Ok(wrapped_ast) => {
                let mut code: Vec<vm::Instruction> = Vec::new();
                vm::compile(&wrapped_ast, &mut code);
                format!("{:?}", code)
            }
            Err(err) => format!("3\n{:?}", err),
        }
    } else {
        "No expressions...".to_string()
    };

    write_to_output(&result)
}

#[wasm_bindgen]
pub fn cons(a: String, b: String) -> *const u8 {
    let merged = {
        let a_bc = parse_bitecode(&a).unwrap();
        let b_bc = parse_bitecode(&b).unwrap();
        let combined = a_bc.into_iter().chain(b_bc.into_iter()).collect::<Vec<_>>();
        format!("{:?}", combined)
    };

    write_to_output(&merged)
}
