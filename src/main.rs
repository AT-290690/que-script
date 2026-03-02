#[cfg(not(feature = "shell"))]
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
#[cfg(feature = "shell")]
mod shell;
mod tests;
mod types;
mod vm;
#[cfg(feature = "wasm-compiler")]
#[path = "compilers/wat.rs"]
mod wat;

#[cfg(feature = "shell")]
use std::env;
#[cfg(feature = "shell")]
use std::fs;
#[cfg(feature = "shell")]
use std::path::{ Path, PathBuf };
#[cfg(feature = "shell")]
use wasmtime::{ Engine, Linker, Memory, Module as WasmModule, Store };

#[cfg(feature = "shell")]
fn extract_type_from_wat(src: &str) -> Option<String> {
    src.lines()
        .next()
        .and_then(|line| line.strip_prefix(";; Type:"))
        .map(|rest| rest.trim().to_string())
}

#[cfg(feature = "shell")]
fn i32_at(memory: &Memory, store: &Store<shell::ShellStoreData>, addr: i32) -> Result<i32, String> {
    let offset = usize::try_from(addr).map_err(|_| format!("invalid memory address: {}", addr))?;
    let mut bytes = [0u8; 4];
    memory
        .read(store, offset, &mut bytes)
        .map_err(|_| format!("out of bounds memory read at {}", addr))?;
    Ok(i32::from_le_bytes(bytes))
}

#[cfg(feature = "shell")]
fn i32_to_f32(bits: i32) -> f32 {
    f32::from_bits(bits as u32)
}

#[cfg(feature = "shell")]
struct VecHeader {
    len: i32,
    data_ptr: i32,
}

#[cfg(feature = "shell")]
fn read_vec(
    memory: &Memory,
    store: &Store<shell::ShellStoreData>,
    vec_ptr: i32
) -> Result<VecHeader, String> {
    Ok(VecHeader {
        len: i32_at(memory, store, vec_ptr + 0)?,
        data_ptr: i32_at(memory, store, vec_ptr + 16)?,
    })
}

#[cfg(feature = "shell")]
fn read_vec_items(
    memory: &Memory,
    store: &Store<shell::ShellStoreData>,
    hdr: &VecHeader
) -> Result<Vec<i32>, String> {
    if hdr.len < 0 {
        return Err(format!("negative vector length: {}", hdr.len));
    }
    let mut out = Vec::with_capacity(hdr.len as usize);
    for i in 0..hdr.len {
        out.push(i32_at(memory, store, hdr.data_ptr + i * 4)?);
    }
    Ok(out)
}

#[cfg(feature = "shell")]
fn read_tuple(
    memory: &Memory,
    store: &Store<shell::ShellStoreData>,
    ptr: i32
) -> Result<Vec<i32>, String> {
    let hdr = read_vec(memory, store, ptr)?;
    let items = read_vec_items(memory, store, &hdr)?;
    if hdr.len < 2 {
        return Err(format!("tuple len != 2 ({})", hdr.len));
    }
    Ok(items)
}

#[cfg(feature = "shell")]
fn split_tuple_types(s: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut depth = 0;

    for c in s.chars() {
        match c {
            '[' | '{' => {
                depth += 1;
                current.push(c);
            }
            ']' | '}' => {
                depth -= 1;
                current.push(c);
            }
            '*' if depth == 0 => {
                parts.push(current.trim().to_string());
                current.clear();
            }
            _ => current.push(c),
        }
    }

    if !current.is_empty() {
        parts.push(current.trim().to_string());
    }

    parts
}

#[cfg(feature = "shell")]
fn decode_value(
    ptr: i32,
    typ: &str,
    memory: &Memory,
    store: &Store<shell::ShellStoreData>
) -> Result<String, String> {
    let t = typ.trim();

    if t == "Bool" {
        return Ok((ptr == 1).to_string());
    }
    if t == "Float" {
        return Ok(i32_to_f32(ptr).to_string());
    }
    if t == "Char" {
        let ch = char::from_u32(ptr as u32).unwrap_or('?');
        return Ok(ch.to_string());
    }
    if t == "[Char]" {
        let hdr = read_vec(memory, store, ptr)?;
        let items = read_vec_items(memory, store, &hdr)?;
        let s: String = items
            .into_iter()
            .map(|x| char::from_u32(x as u32).unwrap_or('?'))
            .collect();
        return Ok(s);
    }
    if t.starts_with('[') && t.ends_with(']') {
        let inner = t[1..t.len() - 1].trim();
        let hdr = read_vec(memory, store, ptr)?;
        let items = read_vec_items(memory, store, &hdr)?;
        let mut decoded = Vec::with_capacity(items.len());
        for item_ptr in items {
            decoded.push(decode_value(item_ptr, inner, memory, store)?);
        }
        return Ok(format!("[{}]", decoded.join(" ")));
    }
    if t.starts_with('{') && t.ends_with('}') {
        let content = t[1..t.len() - 1].trim();
        let parts = split_tuple_types(content);
        let raw_items = read_tuple(memory, store, ptr)?;
        let mut decoded = Vec::with_capacity(raw_items.len());
        for (i, item_ptr) in raw_items.into_iter().enumerate() {
            let typ = parts
                .get(i)
                .map(|s| s.as_str())
                .unwrap_or("Int");
            decoded.push(decode_value(item_ptr, typ, memory, store)?);
        }
        return Ok(format!("{{ {} }}", decoded.join(" ")));
    }

    Ok(ptr.to_string())
}

#[cfg(feature = "shell")]
fn set_argv_i32(
    store: &mut Store<shell::ShellStoreData>,
    instance: &wasmtime::Instance,
    argv: &[i32]
) -> wasmtime::Result<()> {
    let make_vec = instance.get_typed_func::<i32, i32>(&mut *store, "make_vec")?;
    let vec_push = instance.get_typed_func::<(i32, i32), i32>(&mut *store, "vec_push")?;
    let set_argv = instance.get_typed_func::<i32, i32>(&mut *store, "set_argv")?;

    let vec_ptr = make_vec.call(&mut *store, 0)?;
    for &value in argv {
        let _ = vec_push.call(&mut *store, (vec_ptr, value))?;
    }
    let _ = set_argv.call(&mut *store, vec_ptr)?;

    if let Ok(release) = instance.get_typed_func::<i32, i32>(&mut *store, "release") {
        let _ = release.call(&mut *store, vec_ptr)?;
    }
    Ok(())
}

#[cfg(feature = "shell")]
fn run_native_shell() -> Result<(), String> {
    let args: Vec<String> = env::args().collect();
    let Some(file_path) = args.get(1) else {
        return Err("missing file_path. Usage: fez-rs <script.fez> [i32 ...]".to_string());
    };
    let mut argv = Vec::new();
    for raw in args.iter().skip(2) {
        let parsed = raw
            .parse::<i32>()
            .map_err(|_| format!("invalid ARGV item '{}': expected i32", raw))?;
        argv.push(parsed);
    }

    let program = fs
        ::read_to_string(&file_path)
        .map_err(|e| format!("failed to read '{}': {}", file_path, e))?;
    let script_cwd = fs
        ::canonicalize(file_path)
        .ok()
        .and_then(|path| path.parent().map(Path::to_path_buf))
        .or_else(|| Path::new(file_path).parent().map(Path::to_path_buf))
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or_else(|| PathBuf::from("."));
    let std_ast = crate::baked::load_ast();
    let wrapped_ast = match &std_ast {
        crate::parser::Expression::Apply(items) => {
            crate::parser
                ::merge_std_and_program(&program, items[1..].to_vec())
                .map_err(|e| e.to_string())?
        }
        _ => {
            return Err("failed to load standard library AST".to_string());
        }
    };

    let wat_src = crate::wat::compile_program_to_wat(&wrapped_ast)?;
    let ret_type = extract_type_from_wat(&wat_src).unwrap_or_else(|| "Int".to_string());
    let wasm_bytes = ::wat::parse_str(&wat_src).map_err(|e| e.to_string())?;

    let engine = Engine::default();
    let module = WasmModule::new(&engine, wasm_bytes).map_err(|e| e.to_string())?;
    let mut linker = Linker::new(&engine);
    crate::shell::add_shell_to_linker(&mut linker).map_err(|e| e.to_string())?;
    let mut store = Store::new(
        &engine,
        crate::shell::ShellStoreData
            ::new_with_script_cwd(Some(script_cwd))
            .map_err(|e| e.to_string())?
    );
    let instance = linker.instantiate(&mut store, &module).map_err(|e| e.to_string())?;

    set_argv_i32(&mut store, &instance, &argv).map_err(|e| e.to_string())?;

    let main = instance.get_typed_func::<(), i32>(&mut store, "main").map_err(|e| e.to_string())?;
    let result = main.call(&mut store, ()).map_err(|e| e.to_string())?;

    let memory = instance
        .get_memory(&mut store, "memory")
        .ok_or_else(|| "no exported memory".to_string())?;
    let decoded = decode_value(result, &ret_type, &memory, &store)?;
    println!("{}", decoded);

    Ok(())
}

#[cfg(feature = "shell")]
fn main() {
    if let Err(err) = run_native_shell() {
        eprintln!("\x1b[31mException: {}\x1b[0m", err);
        std::process::exit(1);
    }
}

#[cfg(not(feature = "shell"))]
fn main() {
    cli("./example").unwrap();
}
