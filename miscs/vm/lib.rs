#![allow(dead_code)]
#![allow(warnings)]
mod parser;
mod vm;
use crate::vm::parse_bitecode;
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
pub fn exec(program: String) -> *const u8 {
    let result = match vm::exe(parse_bitecode(&program).unwrap(), crate::vm::VM::new()) {
        Ok(res) => format!("0\n{:?}", res),
        Err(err) => format!("2\n{}", err),
    };

    write_to_output(&result)
}
