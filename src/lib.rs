#![allow(dead_code)]
#![allow(warnings)]
#[cfg(feature = "parser")]
mod baked;
// mod format;
#[cfg(feature = "type-checker")]
mod infer;
#[cfg(feature = "js-compiler")]
#[path = "compilers/js.rs"]
mod js;
#[cfg(feature = "parser")]
mod parser;
#[cfg(feature = "report")]
mod report;
#[cfg(feature = "type-checker")]
mod types;
#[cfg(feature = "vm")]
mod vm;
#[cfg(feature = "wasm-compiler")]
#[path = "compilers/wat.rs"]
mod wat;
#[cfg(feature = "vm")]
use crate::vm::parse_bitecode;
use std::cell::RefCell;
use wasm_bindgen::prelude::wasm_bindgen;

use std::io::Write;

thread_local! {
    static OUTPUT: RefCell<Vec<u8>> = RefCell::new(Vec::new());
    #[cfg(feature = "parser")]
    static STD: RefCell<parser::Expression> = RefCell::new(baked::load_ast());
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
#[cfg(all(feature = "parser", feature = "type-checker", feature = "vm"))]
pub fn evaluate(program: String) -> *const u8 {
    let result = STD.with(|std| {
        let std_ast = std.borrow();
        if let parser::Expression::Apply(items) = &*std_ast {
            match parser::merge_std_and_program(&program, items[1..].to_vec()) {
                Ok(wrapped_ast) => {
                    match
                        infer::infer_with_builtins(
                            &wrapped_ast,
                            types::create_builtin_environment(types::TypeEnv::new())
                        )
                    {
                        Ok(typ) =>
                            match vm::run(&wrapped_ast, crate::vm::VM::new()) {
                                Ok(res) => format!("0\n{}\n{:?}", typ, res),
                                Err(err) => format!("2\n{}", err),
                            }
                        Err(err) => format!("1\n{}", err),
                    }
                }
                Err(err) => format!("3\n{}", err),
            }
        } else {
            "No expressions...".to_string()
        }
    });

    write_to_output(&result)
}

#[wasm_bindgen]
#[cfg(all(feature = "parser", feature = "vm"))]
pub fn run(program: String) -> *const u8 {
    let result = STD.with(|std| {
        let std_ast = std.borrow();
        if let parser::Expression::Apply(items) = &*std_ast {
            match parser::merge_std_and_program(&program, items[1..].to_vec()) {
                Ok(wrapped_ast) =>
                    match vm::run(&wrapped_ast, crate::vm::VM::new()) {
                        Ok(res) => format!("0\n{:?}", res),
                        Err(err) => format!("2\n{}", err),
                    }
                Err(err) => format!("1\n{}", err),
            }
        } else {
            "No expressions...".to_string()
        }
    });
    write_to_output(&result)
}

#[wasm_bindgen]
#[cfg(all(feature = "parser", feature = "js-compiler"))]
pub fn js(program: String) -> *const u8 {
    let result = STD.with(|std| {
        let std_ast = std.borrow();
        if let parser::Expression::Apply(items) = &*std_ast {
            match parser::merge_std_and_program(&program, items[1..].to_vec()) {
                Ok(wrapped_ast) => js::compile_program_to_js(&wrapped_ast),
                Err(err) => format!("3\n{:?}", err),
            }
        } else {
            "No expressions...".to_string()
        }
    });
    write_to_output(&result)
}

#[wasm_bindgen]
#[cfg(all(feature = "parser", feature = "wasm-compiler"))]
pub fn wat(program: String) -> *const u8 {
    let result = STD.with(|std| {
        let std_ast = std.borrow();
        if let parser::Expression::Apply(items) = &*std_ast {
            match parser::merge_std_and_program(&program, items[1..].to_vec()) {
                Ok(wrapped_ast) =>
                    match wat::compile_program_to_wat(&wrapped_ast) {
                        Ok(wat_src) => wat_src,
                        Err(err) => format!("3\n{}", err),
                    }
                Err(err) => format!("2\n{}", err),
            }
        } else {
            "1\nNo expressions...".to_string()
        }
    });
    write_to_output(&result)
}

#[wasm_bindgen]
#[cfg(all(feature = "parser", feature = "wat-compiler"))]
pub fn wat(program: String) -> *const u8 {
    let result = STD.with(|std| {
        let std_ast = std.borrow();
        if let parser::Expression::Apply(items) = &*std_ast {
            match parser::merge_std_and_program(&program, items[1..].to_vec()) {
                Ok(wrapped_ast) =>
                    match wat::compile_program_to_wat(&wrapped_ast) {
                        Ok(wat_src) => wat_src,
                        Err(err) => format!("3\n{}", err),
                    }
                Err(err) => format!("2\n{}", err),
            }
        } else {
            "1\nNo expressions...".to_string()
        }
    });
    write_to_output(&result)
}

#[wasm_bindgen]
#[cfg(all(feature = "parser", feature = "type-checker"))]
pub fn check(program: String) -> *const u8 {
    let result = STD.with(|std| {
        let std_ast = std.borrow();
        if let parser::Expression::Apply(items) = &*std_ast {
            match parser::merge_std_and_program(&program, items[1..].to_vec()) {
                Ok(wrapped_ast) => {
                    match
                        infer::infer_with_builtins(
                            &wrapped_ast,
                            types::create_builtin_environment(types::TypeEnv::new())
                        )
                    {
                        Ok(typ) => format!("0\n{}", typ),
                        Err(err) => format!("1\n{}", err),
                    }
                }
                Err(err) => format!("3\n{}", err),
            }
        } else {
            "No expressions...".to_string()
        }
    });
    write_to_output(&result)
}

#[wasm_bindgen]
#[cfg(feature = "vm")]
pub fn exec(program: String) -> *const u8 {
    let result = match vm::exe(parse_bitecode(&program).unwrap(), crate::vm::VM::new()) {
        Ok(res) => format!("0\n{:?}", res),
        Err(err) => format!("2\n{}", err),
    };

    write_to_output(&result)
}

#[wasm_bindgen]
#[cfg(all(feature = "parser", feature = "vm"))]
pub fn comp(program: String) -> *const u8 {
    let result = STD.with(|std| {
        let std_ast = std.borrow();
        if let parser::Expression::Apply(items) = &*std_ast {
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
        }
    });

    write_to_output(&result)
}

#[wasm_bindgen]
#[cfg(feature = "vm")]
pub fn cons(a: String, b: String) -> *const u8 {
    let merged = {
        let a_bc = parse_bitecode(&a).unwrap();
        let b_bc = parse_bitecode(&b).unwrap();
        let combined = a_bc.into_iter().chain(b_bc.into_iter()).collect::<Vec<_>>();
        format!("{:?}", combined)
    };

    write_to_output(&merged)
}
// #[wasm_bindgen]
// pub fn format(program: String) -> *const u8 {
//     write_to_output(&format::format_source(&program))
// }
#[wasm_bindgen]
#[cfg(all(feature = "parser", feature = "type-checker"))]
pub fn signatures(program: String) -> *const u8 {
    STD.with(|std| {
        let mut names = Vec::new();
        let std_ast = std.borrow();
        match &*std_ast {
            parser::Expression::Apply(items) => {
                match parser::merge_std_and_program(&program, items.to_vec()) {
                    Ok(ast) => {
                        match
                            infer::infer_with_builtins_env(
                                &ast,
                                crate::types::create_builtin_environment(
                                    crate::types::TypeEnv::new()
                                )
                            )
                        {
                            Ok(env) =>
                                env.scopes.iter().for_each(|x| {
                                    for key in x.keys().into_iter().collect::<Vec<_>>() {
                                        names.push([
                                            key.to_string(),
                                            x.get(key).unwrap().to_string(),
                                        ]);
                                    }
                                }),
                            Err(_) => {}
                        }

                        let res = format!(
                            "{}",
                            names
                                .iter()
                                .map(|s| s.join(","))
                                .collect::<Vec<_>>()
                                .join("\n")
                        );

                        write_to_output(&res)
                    }
                    Err(err) => write_to_output(&format!("2\n{:?}", err)),
                }
            }
            err => write_to_output(&format!("2\n{:?}", err)),
        }
    })
}

#[wasm_bindgen]
#[cfg(all(feature = "parser", feature = "type-checker", feature = "report"))]
pub fn report(program: String) -> *const u8 {
    let result = STD.with(|std| {
        let std_ast = std.borrow();
        if let crate::parser::Expression::Apply(items) = &*std_ast {
            let wrapped_ast = match
                crate::parser::merge_std_and_program(&program, items[1..].to_vec())
            {
                Ok(ast) => ast,
                Err(err) => {
                    return err;
                }
            };

            if
                let Err(err) = crate::infer::infer_with_builtins(
                    &wrapped_ast,
                    crate::types::create_builtin_environment(crate::types::TypeEnv::new())
                )
            {
                return err;
            }

            let (result, report) = crate::report::run_with_report(
                &wrapped_ast,
                crate::report::VM::new()
            );
            let (final_result, runtime_error) = match result {
                Ok(value) => (Some(format!("{:?}", value)), None),
                Err(err) => (None, Some(err)),
            };
            let anomaly_report = report.llm_anomaly_report();
            return crate::report::format_anomaly_report_text(
                final_result.as_deref(),
                runtime_error.as_deref(),
                &anomaly_report
            );
        }
        "No expressions...".to_string()
    });
    write_to_output(&result)
}
