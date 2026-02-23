#![allow(dead_code)]
#![allow(warnings)]

use crate::baked::load_ast;
use crate::format::format;
use crate::infer::infer_with_builtins_env;
use crate::parser::build;
#[cfg(feature = "repl")]
use crate::repl::repl;
use crate::vm::parse_bitecode;
use flate2::write::GzEncoder;
use flate2::Compression;
use std::cell::RefCell;
use std::env;
use std::fs;
use std::io::Read;
use std::io::{ self, Write };
use std::num::Wrapping;

#[cfg(feature = "deref-wasm")]
use wasmtime::{Engine, Instance, Memory, Module as WasmModule, Store};
#[cfg(feature = "deref-wasm")]
use wat as wat_crate;

thread_local! {
    static STD: RefCell<crate::parser::Expression> = RefCell::new(crate::baked::load_ast());
}
pub fn run_code(program: String) -> String {
    STD.with(|std| {
        let std_ast = std.borrow();
        if let crate::parser::Expression::Apply(items) = &*std_ast {
            match crate::parser::merge_std_and_program(&program, items[1..].to_vec()) {
                Ok(wrapped_ast) => {
                    match
                        crate::infer::infer_with_builtins(
                            &wrapped_ast,
                            crate::types::create_builtin_environment(crate::types::TypeEnv::new())
                        )
                    {
                        Ok(typ) =>
                            match crate::vm::run(&wrapped_ast, crate::vm::VM::new()) {
                                Ok(res) => {
                                    return format!("{}\n{:?}", typ, res);
                                }
                                Err(err) => {
                                    return err;
                                }
                            }
                        Err(err) => {
                            return err;
                        }
                    }
                }
                Err(err) => {
                    return err;
                }
            }
        }
        "No expressions...".to_string()
    })
}

pub fn run_code_report(program: String) -> String {
    STD.with(|std| {
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
    })
}

#[cfg(feature = "type-ast")]
fn format_typed_ast_node(node: &crate::infer::TypedExpression, indent: usize, out: &mut String) {
    let pad = "  ".repeat(indent);
    let typ = node.typ
        .as_ref()
        .map(|t| t.to_string())
        .unwrap_or_else(|| "_".to_string());
    out.push_str(&format!("{}{} :: {}\n", pad, node.expr.to_lisp(), typ));
    for child in &node.children {
        format_typed_ast_node(child, indent + 1, out);
    }
}

#[cfg(feature = "type-ast")]
pub fn run_code_typed_ast(program: String) -> String {
    STD.with(|std| {
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

            let (typ, typed_ast) = match
                crate::infer::infer_with_builtins_typed(
                    &wrapped_ast,
                    crate::types::create_builtin_environment(crate::types::TypeEnv::new())
                )
            {
                Ok(value) => value,
                Err(err) => {
                    return err;
                }
            };

            let mut output = String::new();
            output.push_str(&format!("program_type: {}\n", typ));
            output.push_str("typed_ast:\n");
            format_typed_ast_node(&typed_ast, 0, &mut output);
            return output;
        }
        "No expressions...".to_string()
    })
}

#[cfg(feature = "type-ast")]
fn typed_ast_for_expression(expr: &crate::parser::Expression) -> String {
    match
        crate::infer::infer_with_builtins_typed(
            expr,
            crate::types::create_builtin_environment(crate::types::TypeEnv::new())
        )
    {
        Ok((typ, typed_ast)) => {
            let mut output = String::new();
            output.push_str(&format!("program_type: {}\n", typ));
            output.push_str("typed_ast:\n");
            format_typed_ast_node(&typed_ast, 0, &mut output);
            output
        }
        Err(err) => err,
    }
}

fn compress(data: &str) -> Vec<u8> {
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(data.as_bytes()).expect("Failed to compress data");
    encoder.finish().expect("Failed to finish compression")
}

fn dump_wrapped_libs(expr: &str, path: &str) -> io::Result<()> {
    let mut file = fs::File::create(path)?;
    writeln!(
        file,
        "use std::io::Read;fn decompress(compressed: &[u8]) -> crate::parser::Expression {{use flate2::read::GzDecoder;let mut decoder = GzDecoder::new(compressed);let mut decompressed_data = String::new();decoder.read_to_string(&mut decompressed_data).expect(\"Failed to decompress data\");let expressions =crate::parser::parse(&decompressed_data).expect(\"Failed to parse decompressed data\");expressions.first().expect(\"No expressions returned\").clone()}}"
    );
    let compressed_code = compress(&expr);
    writeln!(file, "pub fn load_ast() -> crate::parser::Expression {{")?;
    writeln!(file, "decompress(&{:?})", compressed_code)?;
    writeln!(file, "}}")?;
    Ok(())
}

pub fn dump_wrapped_bytecode(code: Vec<crate::vm::Instruction>, path: &str) -> std::io::Result<()> {
    let mut file: fs::File = fs::File::create(path)?;
    writeln!(file, "use crate::vm::Instruction::*;")?;
    // writeln!(
    //     file,
    //     "macro_rules! bytecode {{ ( $($instr:expr),* ) => {{ vec![$( $instr ),*] }};}}"
    // );
    writeln!(file, "macro_rules! s {{($s:expr) => {{ $s.to_string() }}}}")?;
    writeln!(file, "pub fn load_bytecode() -> Vec<crate::vm::Instruction> {{")?;
    write!(file, "vec![")?;

    for (i, instr) in code.iter().enumerate() {
        write!(file, "{}", instr.to_rust())?;
        if i < code.len() - 1 {
            write!(file, ",")?;
        }
    }

    writeln!(file, "]")?;
    write!(file, "}}")?;
    Ok(())
}

pub fn dump_raw_bytecode(code: Vec<crate::vm::Instruction>, path: &str) -> std::io::Result<()> {
    std::fs::create_dir_all(std::path::Path::new(path).parent().unwrap()).unwrap();
    let mut file = fs::File::create(path)?;
    write!(file, "[")?;
    for (i, instr) in code.iter().enumerate() {
        write!(file, "{}", instr.serialise())?;
        if i < code.len() - 1 {
            write!(file, ",")?;
        }
    }
    writeln!(file, "]")?;
    Ok(())
}

pub fn dump_wrapped_js(src: String, path: &str) -> std::io::Result<()> {
    std::fs::create_dir_all(std::path::Path::new(path).parent().unwrap()).unwrap();
    let mut file = fs::File::create(path)?;
    writeln!(
        file,
        "const _prettyResult = (v) => {{ if (typeof v === 'boolean') return v ? 'true' : 'false'; if (v === null || v === undefined) return '()'; if (Array.isArray(v)) return `[${{v.map(_prettyResult).join(' ')}}]`; return String(v); }};"
    )?;
    writeln!(file, "const _ =(")?;
    writeln!(file, "{}", src)?;
    writeln!(file, ");console.log(_prettyResult(_))")?;

    Ok(())
}
pub fn cons(a: String, b: String) -> String {
    return format!(
        "{:?}",
        parse_bitecode(&a)
            .unwrap()
            .into_iter()
            .chain(parse_bitecode(&b).unwrap().into_iter())
            .collect::<Vec<_>>()
    );
}

#[cfg(feature = "deref-wasm")]
fn extract_type_from_wat(src: &str) -> Option<String> {
    // Convention: first line is `;; Type: <typ>`
    src.lines()
        .next()
        .and_then(|line| line.strip_prefix(";; Type:"))
        .map(|rest| rest.trim().to_string())
}

#[cfg(feature = "deref-wasm")]
fn i32_at(mem: &Memory, store: &mut Store<()>, addr: i32) -> i32 {
    let data = mem.data(store);
    let i = addr as usize;
    let bytes = &data[i..i + 4];
    i32::from_le_bytes(bytes.try_into().unwrap())
}

#[cfg(feature = "deref-wasm")]
struct VecHeader {
    len: i32,
    _cap: i32,
    _rc: i32,
    _elem_ref: i32,
    data_ptr: i32,
}

#[cfg(feature = "deref-wasm")]
fn read_vec_header(mem: &Memory, store: &mut Store<()>, ptr: i32) -> VecHeader {
    VecHeader {
        len: i32_at(mem, store, ptr + 0),
        _cap: i32_at(mem, store, ptr + 4),
        _rc: i32_at(mem, store, ptr + 8),
        _elem_ref: i32_at(mem, store, ptr + 12),
        data_ptr: i32_at(mem, store, ptr + 16),
    }
}

#[cfg(feature = "deref-wasm")]
fn read_vec_items(mem: &Memory, store: &mut Store<()>, hdr: &VecHeader) -> Vec<i32> {
    let mut out = Vec::with_capacity(hdr.len as usize);
    for i in 0..hdr.len {
        out.push(i32_at(mem, store, hdr.data_ptr + i * 4));
    }
    out
}

#[cfg(feature = "deref-wasm")]
fn read_tuple2(mem: &Memory, store: &mut Store<()>, ptr: i32) -> (i32, i32) {
    let hdr = read_vec_header(mem, store, ptr);
    let items = read_vec_items(mem, store, &hdr);
    assert_eq!(items.len(), 2, "tuple len != 2");
    (items[0], items[1])
}

/// Very small subset of JS getValue: Int, [Int], { [Int] * [Int] }
#[cfg(feature = "deref-wasm")]
fn decode_value(ptr: i32, typ: &str, mem: &Memory, store: &mut Store<()>) -> String {
    let t = typ.trim();
    if t == "Int" {
        return format!("{}", ptr);
    }
    if t.starts_with('[') && t.ends_with(']') {
        let inner = t[1..t.len() - 1].trim();
        if inner == "Int" {
            let hdr = read_vec_header(mem, store, ptr);
            let items = read_vec_items(mem, store, &hdr);
            return format!(
                "[{}]",
                items
                    .iter()
                    .map(|x| x.to_string())
                    .collect::<Vec<_>>()
                    .join(" ")
            );
        }
    }
    if t.starts_with('{') && t.ends_with('}') {
        let content = &t[1..t.len() - 1];
        let parts: Vec<_> = content.split('*').map(|s| s.trim()).collect();
        if parts.len() == 2 && parts[0] == "[Int]" && parts[1] == "[Int]" {
            let (a_ptr, b_ptr) = read_tuple2(mem, store, ptr);
            let a = decode_value(a_ptr, "[Int]", mem, store);
            let b = decode_value(b_ptr, "[Int]", mem, store);
            return format!("{{ {} {} }}", a, b);
        }
    }
    format!("ptr({})", ptr)
}

#[cfg(feature = "deref-wasm")]
fn deref_wat_text(wat_src: &str) -> Result<String, String> {
    let typ = extract_type_from_wat(wat_src).unwrap_or_else(|| "Int".to_string());
    let wasm_bytes = wat_crate::parse_str(wat_src).map_err(|e| e.to_string())?;
    let engine = Engine::default();
    let module =
        WasmModule::new(&engine, &wasm_bytes).map_err(|e| format!("module error: {}", e))?;
    let mut store = Store::new(&engine, ());
    let instance =
        Instance::new(&mut store, &module, &[]).map_err(|e| format!("inst error: {}", e))?;

    let memory = instance
        .get_memory(&mut store, "memory")
        .ok_or_else(|| "no exported memory".to_string())?;

    let main = instance
        .get_typed_func::<(), i32>(&mut store, "main")
        .map_err(|e| format!("main func error: {}", e))?;

    let ptr = main.call(&mut store, ()).map_err(|e| format!("call error: {}", e))?;
    Ok(decode_value(ptr, &typ, &memory, &mut store))
}
pub fn cli(dir: &str) -> std::io::Result<()> {
    let default_source = format!("{}/{}", dir, "main.lisp");
    let default_dist = format!("{}/dist", dir);

    let args: Vec<String> = env::args().collect();
    let get_opt = |name: &str| -> Option<String> {
        args.windows(2)
            .find(|w| w[0] == name)
            .map(|w| w[1].clone())
    };
    let source_path = get_opt("--s").unwrap_or(default_source);
    let dist = get_opt("--d").unwrap_or(default_dist);
    let known_cmds = [
        "--std",
        "--check",
        "--js",
        "--ml",
        "--py",
        "--kt",
        "--rs",
        "--wat",
        "--comp",
        "--deref-wasm",
        "--exec",
        "--str",
        "--bit",
        "--doc",
        "--sig",
        "--fmt",
        "--repl",
        "--report",
        "--type-ast",
    ];
    let cmd = args
        .iter()
        .find(|a| known_cmds.contains(&a.as_str()))
        .map(|s| s.as_str())
        .unwrap_or("");
    let resolve_output_path = |dest: &str, default_file: &str| -> String {
        let p = std::path::Path::new(dest);
        if p.extension().is_some() {
            dest.to_string()
        } else {
            format!("{}/{}", dest, default_file)
        }
    };
    if cmd == "--std" {
        let combined = format!(
            "{}\n{}\n{}\n{}",
            fs::read_to_string("./lisp/const.lisp")?,
            fs::read_to_string("./lisp/std.lisp")?,
            fs::read_to_string("./lisp/fp.lisp")?,
            fs::read_to_string("./lisp/ds.lisp")?
        );
        // let mut file = fs::File::create("./combined.lisp")?;
        // writeln!(file, "{}", combined)?;
        dump_wrapped_libs(&build(&combined).unwrap().to_lisp(), "./src/baked.rs");
    } else if cmd == "--check" {
        let program = fs::read_to_string(&source_path)?;
        STD.with(|std| {
            let std_ast = std.borrow();
            if let crate::parser::Expression::Apply(items) = &*std_ast {
                match crate::parser::merge_std_and_program(&program, items[1..].to_vec()) {
                    Ok(wrapped_ast) =>
                        match
                            crate::infer::infer_with_builtins(
                                &wrapped_ast,
                                crate::types::create_builtin_environment(
                                    crate::types::TypeEnv::new()
                                )
                            )
                        {
                            Ok(typ) => println!("{}", typ),
                            Err(e) => println!("{}", e),
                        }
                    Err(e) => println!("{}", e),
                }
            }
        });
    } else if cmd == "--js" {
        #[cfg(feature = "js-compiler")]
        {
            let program = fs::read_to_string(&source_path)?;
            let std_ast = crate::baked::load_ast();
            STD.with(|std| {
                let std_ast = std.borrow();
                if let crate::parser::Expression::Apply(items) = &*std_ast {
                    match crate::parser::merge_std_and_program(&program, items[1..].to_vec()) {
                        Ok(wrapped_ast) => {
                            let mut code: Vec<crate::vm::Instruction> = Vec::new();
                            let js = crate::js::compile_program_to_js(&wrapped_ast);
                            let target = resolve_output_path(&dist, "main.js");
                            dump_wrapped_js(js, &target);
                        }
                        Err(e) => println!("{}", e),
                    }
                }
            });
        }
        #[cfg(not(feature = "js-compiler"))]
        {
            println!("Error! JS compiler is disabled. Rebuild with --features js-compiler");
        }
    } else if cmd == "--ml" {
        #[cfg(feature = "ocaml-compiler")]
        {
            let program = fs::read_to_string(&source_path)?;
            STD.with(|std| {
                let std_ast = std.borrow();
                if let crate::parser::Expression::Apply(items) = &*std_ast {
                    match crate::parser::merge_std_and_program(&program, items[1..].to_vec()) {
                        Ok(wrapped_ast) => {
                            let out = match crate::ocaml::compile_program_to_ocaml(&wrapped_ast) {
                                Ok(src) => src,
                                Err(err) => err,
                            };
                            let target = resolve_output_path(&dist, "main.ml");
                            std::fs
                                ::create_dir_all(std::path::Path::new(&target).parent().unwrap())
                                .unwrap();
                            let mut out_file = fs::File::create(target).unwrap();
                            writeln!(out_file, "{}", out).unwrap();
                        }
                        Err(e) => println!("{}", e),
                    }
                }
            });
        }
        #[cfg(not(feature = "ocaml-compiler"))]
        {
            println!("Error! OCaml compiler is disabled. Rebuild with --features ocaml-compiler");
        }
    } else if cmd == "--py" {
        #[cfg(feature = "python-compiler")]
        {
            let program = fs::read_to_string(&source_path)?;
            STD.with(|std| {
                let std_ast = std.borrow();
                if let crate::parser::Expression::Apply(items) = &*std_ast {
                    match crate::parser::merge_std_and_program(&program, items[1..].to_vec()) {
                        Ok(wrapped_ast) => {
                            let out = crate::py::compile_program_to_python(&wrapped_ast);
                            let target = resolve_output_path(&dist, "main.py");
                            std::fs
                                ::create_dir_all(std::path::Path::new(&target).parent().unwrap())
                                .unwrap();
                            let mut out_file = fs::File::create(target).unwrap();
                            writeln!(out_file, "{}", out).unwrap();
                        }
                        Err(e) => println!("{}", e),
                    }
                }
            });
        }
        #[cfg(not(feature = "python-compiler"))]
        {
            println!("Error! Python compiler is disabled. Rebuild with --features python-compiler");
        }
    } else if cmd == "--kt" {
        #[cfg(feature = "kotlin-compiler")]
        {
            let program = fs::read_to_string(&source_path)?;
            STD.with(|std| {
                let std_ast = std.borrow();
                if let crate::parser::Expression::Apply(items) = &*std_ast {
                    match crate::parser::merge_std_and_program(&program, items[1..].to_vec()) {
                        Ok(wrapped_ast) => {
                            let out = match crate::kotlin::compile_program_to_kotlin(&wrapped_ast) {
                                Ok(src) => src,
                                Err(err) => err,
                            };
                            let target = resolve_output_path(&dist, "main.kt");
                            std::fs
                                ::create_dir_all(std::path::Path::new(&target).parent().unwrap())
                                .unwrap();
                            let mut out_file = fs::File::create(target).unwrap();
                            writeln!(out_file, "{}", out).unwrap();
                        }
                        Err(e) => println!("{}", e),
                    }
                }
            });
        }
        #[cfg(not(feature = "kotlin-compiler"))]
        {
            println!("Error! Kotlin compiler is disabled. Rebuild with --features kotlin-compiler");
        }
    } else if cmd == "--rs" {
        #[cfg(feature = "rust-compiler")]
        {
            let program = fs::read_to_string(&source_path)?;
            STD.with(|std| {
                let std_ast = std.borrow();
                if let crate::parser::Expression::Apply(items) = &*std_ast {
                    match crate::parser::merge_std_and_program(&program, items[1..].to_vec()) {
                        Ok(wrapped_ast) => {
                            let out = match crate::rs::compile_program_to_rust(&wrapped_ast) {
                                Ok(src) => src,
                                Err(err) => err,
                            };
                            let target = resolve_output_path(&dist, "main.rs");
                            std::fs
                                ::create_dir_all(std::path::Path::new(&target).parent().unwrap())
                                .unwrap();
                            let mut out_file = fs::File::create(target).unwrap();
                            writeln!(out_file, "{}", out).unwrap();
                        }
                        Err(e) => println!("{}", e),
                    }
                }
            });
        }
        #[cfg(not(feature = "rust-compiler"))]
        {
            println!("Error! Rust compiler is disabled. Rebuild with --features rust-compiler");
        }
    } else if cmd == "--wat" {
        #[cfg(feature = "wasm-compiler")]
        {
            let program = fs::read_to_string(&source_path)?;
            STD.with(|std| {
                let std_ast = std.borrow();
                if let crate::parser::Expression::Apply(items) = &*std_ast {
                    match crate::parser::merge_std_and_program(&program, items[1..].to_vec()) {
                        Ok(wrapped_ast) =>
                            match crate::wat::compile_program_to_wat(&wrapped_ast) {
                                Ok(out) => {
                                    let target = resolve_output_path(&dist, "main.wat");
                                    std::fs
                                        ::create_dir_all(
                                            std::path::Path::new(&target).parent().unwrap()
                                        )
                                        .unwrap();
                                    let mut out_file = fs::File::create(target).unwrap();
                                    writeln!(out_file, "{}", out).unwrap();
                                }
                                Err(err) => println!("{}", err),
                            }
                        Err(e) => println!("{}", e),
                    }
                }
            });
        }
        #[cfg(not(feature = "wasm-compiler"))]
        {
            println!("Error! Wasm compiler is disabled. Rebuild with --features wasm-compiler");
        }
    } else if cmd == "--deref-wasm" {
        #[cfg(feature = "deref-wasm")]
        {
            let program = fs::read_to_string(&source_path)?;
            STD.with(|std| {
                let std_ast = std.borrow();
                if let crate::parser::Expression::Apply(items) = &*std_ast {
                    match crate::parser::merge_std_and_program(&program, items[1..].to_vec()) {
                        Ok(wrapped_ast) => match crate::wat::compile_program_to_wat(&wrapped_ast) {
                            Ok(wat_src) => match deref_wat_text(&wat_src) {
                                Ok(value) => println!("{}", value),
                                Err(err) => println!("{}", err),
                            },
                            Err(err) => println!("{}", err),
                        },
                        Err(e) => println!("{}", e),
                    }
                }
            });
        }
        #[cfg(not(feature = "deref-wasm"))]
        {
            println!("Error! deref-wasm is disabled. Rebuild with --features deref-wasm");
        }
    } else if cmd == "--comp" {
        let program = fs::read_to_string(&source_path)?;
        STD.with(|std| {
            let std_ast = std.borrow();
            if let crate::parser::Expression::Apply(items) = &*std_ast {
                match crate::parser::merge_std_and_program(&program, items[1..].to_vec()) {
                    Ok(wrapped_ast) => {
                        let mut code: Vec<crate::vm::Instruction> = Vec::new();
                        crate::vm::compile(&wrapped_ast, &mut code);
                        let target = resolve_output_path(&dist, "main.txt");
                        dump_raw_bytecode(code, &target);
                    }
                    Err(e) => println!("{}", e),
                }
            }
        });
    } else if cmd == "--exec" {
        let path = args
            .get(1)
            .filter(|x| !x.starts_with("--"))
            .cloned()
            .unwrap_or_else(|| format!("{}/main.txt", dist));
        let program = fs::read_to_string(path)?;
        match crate::vm::exe(parse_bitecode(&program).unwrap(), crate::vm::VM::new()) {
            Ok(e) => println!("{:?}", e),
            Err(e) => println!("{:?}", e),
        }
    } else if cmd == "--str" {
        let program = fs::read_to_string(&source_path)?;
        STD.with(|std| {
            let std_ast = std.borrow();
            if let crate::parser::Expression::Apply(items) = &*std_ast {
                match crate::parser::merge_std_and_program(&program, items[1..].to_vec()) {
                    Ok(wrapped_ast) => {
                        let mut code: Vec<crate::vm::Instruction> = Vec::new();
                        crate::vm::compile(&wrapped_ast, &mut code);
                        dump_raw_bytecode(code, &format!("{}/main.txt", dist));
                    }
                    Err(e) => println!("{}", e),
                }
            }
        });
    } else if cmd == "--bit" {
        let program = fs::read_to_string(&format!("{}/main.txt", dist))?;
        match crate::vm::exe(parse_bitecode(&program).unwrap(), crate::vm::VM::new()) {
            Ok(e) => println!("{:?}", e),
            Err(e) => println!("{:?}", e),
        }
    } else if cmd == "--doc" {
        let std_ast = crate::baked::load_ast();
        let mut names = Vec::new();
        "+ +# +. - -# -. / /# /. * *# *. mod mod. = =? =# =. < <# <. > ># >. <= <=# <=. >= >=# >=. not and or ^ >> << | & ~ true false car cdr Int->Float Float->Int"
            .split(" ")
            .for_each(|p| {
                let name = p.to_string();
                match
                    crate::infer::infer_with_builtins(
                        &crate::parser::Expression::Word(p.to_string()),
                        crate::types::create_builtin_environment(crate::types::TypeEnv::new())
                    )
                {
                    Ok(typ) => names.push([name, format!("{}", typ)]),
                    Err(e) => println!("{}", e),
                }
            });
        if let crate::parser::Expression::Apply(items) = &std_ast {
            for expr in items[1..].to_vec() {
                if let crate::parser::Expression::Apply(list) = expr {
                    let a = &list[0];
                    let b = &list[1];
                    if let crate::parser::Expression::Word(kw) = a {
                        if kw == "let" {
                            if let crate::parser::Expression::Word(name) = b {
                                if name.chars().next().unwrap() == '!' {
                                    names.push([name.clone(), "unsafe".to_string()]);
                                } else {
                                    match
                                        crate::parser::merge_std_and_program(
                                            &name,
                                            items[1..].to_vec()
                                        )
                                    {
                                        Ok(p) =>
                                            match
                                                crate::infer::infer_with_builtins(
                                                    &p,
                                                    crate::types::create_builtin_environment(
                                                        crate::types::TypeEnv::new()
                                                    )
                                                )
                                            {
                                                Ok(typ) => {
                                                    // TODO: use a regex to remove the T+\d+ noise of the files
                                                    names.push([name.clone(), format!("{}", typ)]);
                                                }
                                                Err(e) => println!("{}", e),
                                            }
                                        Err(e) => println!("{}", e),
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        let path = "./example/dist/lib.json";
        std::fs::create_dir_all(std::path::Path::new(path).parent().unwrap()).unwrap();
        let mut file = fs::File::create(path)?;
        write!(file, "{:?}", names)?;

        // let std_typed_ast_path = "./example/dist/std-typed-ast.txt";
        // std::fs
        //     ::create_dir_all(std::path::Path::new(std_typed_ast_path).parent().unwrap())
        //     .unwrap();
        // let mut typed_ast_file = fs::File::create(std_typed_ast_path)?;
        // writeln!(typed_ast_file, "{}", typed_ast_for_expression(&std_ast))?;
    } else if cmd == "--sig" {
        let program = fs::read_to_string(&source_path)?;
        let mut names = Vec::new();
        match crate::baked::load_ast() {
            crate::parser::Expression::Apply(items) => {
                match crate::parser::merge_std_and_program(&program, items.to_vec()) {
                    Ok(ast) => {
                        match
                            infer_with_builtins_env(
                                &ast,
                                crate::types::create_builtin_environment(
                                    crate::types::TypeEnv::new()
                                )
                            )
                        {
                            Ok(env) => {
                                env.scopes.iter().for_each(|x| {
                                    for key in x.keys().into_iter().collect::<Vec<_>>() {
                                        names.push([
                                            key.to_string(),
                                            x.get(key).unwrap().to_string(),
                                        ]);
                                    }
                                });
                            }
                            Err(_) => {}
                        }
                        let path = "./sig.json";
                        std::fs
                            ::create_dir_all(std::path::Path::new(path).parent().unwrap())
                            .unwrap();
                        let mut file = fs::File::create(path)?;
                        write!(file, "{:?}", names)?;
                    }
                    Err(_) => {}
                }
            }
            _ => {}
        }
    } else if cmd == "--fmt" {
        println!("{}", crate::format::format_source(&fs::read_to_string(&source_path)?));
    } else if cmd == "--repl" {
        if args.len() == 6 {
            let path = &args[4];
            #[cfg(feature = "repl")]
            repl(fs::read_to_string(path)?);
        } else {
            #[cfg(feature = "repl")]
            repl(String::new());
        }
    } else if cmd == "--report" {
        let file = fs::read_to_string(&source_path)?;
        let report = "./example/dist/report.txt";
        std::fs::create_dir_all(std::path::Path::new(report).parent().unwrap()).unwrap();
        let mut fileOut = fs::File::create(report)?;
        writeln!(fileOut, "{}", run_code_report(file))?;
    } else if cmd == "--type-ast" {
        #[cfg(feature = "type-ast")]
        {
            let file = fs::read_to_string(&source_path)?;
            let typed_ast = "./example/dist/typed-ast.txt";
            std::fs::create_dir_all(std::path::Path::new(typed_ast).parent().unwrap()).unwrap();
            let mut fileOut = fs::File::create(typed_ast)?;
            writeln!(fileOut, "{}", run_code_typed_ast(file))?;
        }
        #[cfg(not(feature = "type-ast"))]
        {
            println!("Error! typed-ast is disabled. Rebuild with --features type-ast");
        }
    } else {
        let file = fs::read_to_string(&source_path)?;
        println!("{}", run_code(file));
    }

    Ok(())
}
