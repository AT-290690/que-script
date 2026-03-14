use wasmtime::{ Config, Engine, Linker, Memory, Module as WasmModule, OptLevel, Store, Strategy };
use wat as wat_crate;

fn extract_type_from_wat(src: &str) -> Option<String> {
    src.lines()
        .next()
        .and_then(|line| line.strip_prefix(";; Type:"))
        .map(|rest| rest.trim().to_string())
}

fn i32_at<T>(memory: &Memory, store: &Store<T>, addr: i32) -> Result<i32, String> {
    let offset = usize::try_from(addr).map_err(|_| format!("invalid memory address: {}", addr))?;
    let mut bytes = [0u8; 4];
    memory
        .read(store, offset, &mut bytes)
        .map_err(|_| format!("out of bounds memory read at {}", addr))?;
    Ok(i32::from_le_bytes(bytes))
}

fn i32_to_f32(bits: i32) -> f32 {
    f32::from_bits(bits as u32)
}

struct VecHeader {
    len: i32,
    data_ptr: i32,
}

fn read_vec<T>(memory: &Memory, store: &Store<T>, vec_ptr: i32) -> Result<VecHeader, String> {
    Ok(VecHeader {
        len: i32_at(memory, store, vec_ptr + 0)?,
        data_ptr: i32_at(memory, store, vec_ptr + 16)?,
    })
}

fn read_vec_items<T>(
    memory: &Memory,
    store: &Store<T>,
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

fn read_tuple<T>(memory: &Memory, store: &Store<T>, ptr: i32) -> Result<Vec<i32>, String> {
    let hdr = read_vec(memory, store, ptr)?;
    let items = read_vec_items(memory, store, &hdr)?;
    if hdr.len < 2 {
        return Err(format!("tuple len != 2 ({})", hdr.len));
    }
    Ok(items)
}

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

fn strip_outer_type(t: &str, open: char, close: char) -> Option<String> {
    let s = t.trim();
    if !s.starts_with(open) || !s.ends_with(close) {
        return None;
    }

    let mut depth = 0i32;
    let mut saw_outer_close_at_end = false;
    for (idx, ch) in s.char_indices() {
        if ch == open {
            depth += 1;
        } else if ch == close {
            depth -= 1;
            if depth == 0 {
                saw_outer_close_at_end = idx + ch.len_utf8() == s.len();
                if !saw_outer_close_at_end {
                    return None;
                }
            } else if depth < 0 {
                return None;
            }
        }
    }

    if depth != 0 || !saw_outer_close_at_end {
        return None;
    }

    let inner_start = open.len_utf8();
    let inner_end = s.len() - close.len_utf8();
    Some(s[inner_start..inner_end].trim().to_string())
}

pub fn decode_value<T>(
    ptr: i32,
    typ: &str,
    memory: &Memory,
    store: &Store<T>
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
    if let Some(inner) = strip_outer_type(t, '[', ']') {
        let inner = inner.trim();
        let hdr = read_vec(memory, store, ptr)?;
        let items = read_vec_items(memory, store, &hdr)?;
        let mut decoded = Vec::with_capacity(items.len());
        for item_ptr in items {
            decoded.push(decode_value(item_ptr, inner, memory, store)?);
        }
        return Ok(format!("[{}]", decoded.join(" ")));
    }
    if let Some(content) = strip_outer_type(t, '{', '}') {
        let content = content.trim();
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

fn set_argv_strings<T>(
    store: &mut Store<T>,
    instance: &wasmtime::Instance,
    argv: &[String]
) -> wasmtime::Result<()> {
    let make_vec = instance.get_typed_func::<i32, i32>(&mut *store, "make_vec")?;
    let vec_push = instance.get_typed_func::<(i32, i32), i32>(&mut *store, "vec_push")?;
    let set_argv = instance.get_typed_func::<i32, i32>(&mut *store, "set_argv")?;
    let release = instance.get_typed_func::<i32, i32>(&mut *store, "release").ok();

    let vec_ptr = make_vec.call(&mut *store, 1)?;
    for raw in argv {
        let arg_ptr = make_vec.call(&mut *store, 0)?;
        for ch in raw.chars() {
            let code = i32::try_from(u32::from(ch)).unwrap_or(0);
            let _ = vec_push.call(&mut *store, (arg_ptr, code))?;
        }
        let _ = vec_push.call(&mut *store, (vec_ptr, arg_ptr))?;
        if let Some(release_fn) = &release {
            let _ = release_fn.call(&mut *store, arg_ptr)?;
        }
    }
    let _ = set_argv.call(&mut *store, vec_ptr)?;

    if let Some(release_fn) = &release {
        let _ = release_fn.call(&mut *store, vec_ptr)?;
    }
    Ok(())
}

fn read_debug_global_i64<T>(
    instance: &wasmtime::Instance,
    store: &mut Store<T>,
    name: &str
) -> Option<i64> {
    let g = instance.get_global(&mut *store, name)?;
    match g.get(&mut *store) {
        wasmtime::Val::I64(v) => Some(v),
        wasmtime::Val::I32(v) => Some(v as i64),
        _ => None,
    }
}

fn debug_rc_snapshot<T>(instance: &wasmtime::Instance, store: &mut Store<T>) -> Option<String> {
    let alloc = read_debug_global_i64(instance, store, "dbg_alloc_count")?;
    let free = read_debug_global_i64(instance, store, "dbg_free_count").unwrap_or(0);
    let retain = read_debug_global_i64(instance, store, "dbg_retain_count").unwrap_or(0);
    let release = read_debug_global_i64(instance, store, "dbg_release_count").unwrap_or(0);
    let vec_new = read_debug_global_i64(instance, store, "dbg_vec_new_count").unwrap_or(0);
    let vec_set = read_debug_global_i64(instance, store, "dbg_vec_set_count").unwrap_or(0);
    let bad_ptr = read_debug_global_i64(instance, store, "dbg_bad_vec_set_ptr").unwrap_or(0);
    let bad_val = read_debug_global_i64(instance, store, "dbg_bad_ref_value").unwrap_or(0);
    let bad_old = read_debug_global_i64(instance, store, "dbg_bad_ref_old").unwrap_or(0);
    let elem_ref_0 = read_debug_global_i64(instance, store, "dbg_vec_set_elem_ref_0").unwrap_or(0);
    let elem_ref_1 = read_debug_global_i64(instance, store, "dbg_vec_set_elem_ref_1").unwrap_or(0);
    let rel_vec_gt0 = read_debug_global_i64(instance, store, "dbg_rc_release_vec_gt0").unwrap_or(0);
    let rel_vec_free = read_debug_global_i64(instance, store, "dbg_rc_release_vec_free").unwrap_or(
        0
    );
    let set_append = read_debug_global_i64(instance, store, "dbg_vec_set_append_path").unwrap_or(0);
    let set_replace = read_debug_global_i64(instance, store, "dbg_vec_set_replace_path").unwrap_or(
        0
    );
    let rel_rc_eq1 = read_debug_global_i64(instance, store, "dbg_rc_release_vec_rc_eq_1").unwrap_or(
        0
    );
    let rel_rc_ge2 = read_debug_global_i64(instance, store, "dbg_rc_release_vec_rc_ge_2").unwrap_or(
        0
    );
    let old_rc_eq1 = read_debug_global_i64(instance, store, "dbg_vec_set_old_rc_eq_1").unwrap_or(0);
    let old_rc_ge2 = read_debug_global_i64(instance, store, "dbg_vec_set_old_rc_ge_2").unwrap_or(0);
    let old_not_vec = read_debug_global_i64(instance, store, "dbg_vec_set_old_not_vec").unwrap_or(
        0
    );
    let tmp_rel_exec = read_debug_global_i64(instance, store, "dbg_tmp_release_exec").unwrap_or(0);
    let tmp_rel_skip = read_debug_global_i64(instance, store, "dbg_tmp_release_skip").unwrap_or(0);
    let v_rc_eq1 = read_debug_global_i64(instance, store, "dbg_vec_set_v_rc_eq_1").unwrap_or(0);
    let v_rc_ge2 = read_debug_global_i64(instance, store, "dbg_vec_set_v_rc_ge_2").unwrap_or(0);
    let v_not_vec = read_debug_global_i64(instance, store, "dbg_vec_set_v_not_vec").unwrap_or(0);
    let tmp_post_eq1 = read_debug_global_i64(
        instance,
        store,
        "dbg_tmp_release_post_rc_eq_1"
    ).unwrap_or(0);
    let tmp_post_other = read_debug_global_i64(
        instance,
        store,
        "dbg_tmp_release_post_rc_other"
    ).unwrap_or(0);
    let tmp_post_not_vec = read_debug_global_i64(
        instance,
        store,
        "dbg_tmp_release_post_not_vec"
    ).unwrap_or(0);
    let rel_reject_not_vec = read_debug_global_i64(
        instance,
        store,
        "dbg_rc_release_reject_not_vec"
    ).unwrap_or(0);
    let rel_take_vec = read_debug_global_i64(
        instance,
        store,
        "dbg_rc_release_take_vec_path"
    ).unwrap_or(0);

    Some(
        format!(
            "debug-rc: alloc={alloc} free={free} retain={retain} release={release} vec_new={vec_new} vec_set={vec_set} bad_vec_set_ptr={bad_ptr} bad_ref_value={bad_val} bad_ref_old={bad_old} vec_set_elem_ref_0={elem_ref_0} vec_set_elem_ref_1={elem_ref_1} rc_release_vec_gt0={rel_vec_gt0} rc_release_vec_free={rel_vec_free} vec_set_append={set_append} vec_set_replace={set_replace} rc_release_vec_rc_eq1={rel_rc_eq1} rc_release_vec_rc_ge2={rel_rc_ge2} vec_set_old_rc_eq1={old_rc_eq1} vec_set_old_rc_ge2={old_rc_ge2} vec_set_old_not_vec={old_not_vec} tmp_release_exec={tmp_rel_exec} tmp_release_skip={tmp_rel_skip} vec_set_v_rc_eq1={v_rc_eq1} vec_set_v_rc_ge2={v_rc_ge2} vec_set_v_not_vec={v_not_vec} tmp_release_post_eq1={tmp_post_eq1} tmp_release_post_other={tmp_post_other} tmp_release_post_not_vec={tmp_post_not_vec} rc_release_reject_not_vec={rel_reject_not_vec} rc_release_take_vec={rel_take_vec}"
        )
    )
}

fn debug_guard_trap_message<T>(
    instance: &wasmtime::Instance,
    store: &mut Store<T>
) -> Option<String> {
    let code = read_debug_global_i64(instance, store, "dbg_guard_trap_code").unwrap_or(0);
    if code == 0 {
        return None;
    }
    let msg = match code {
        1 => "debug.guard_trap: integer divide/modulo by zero (QUE_DIV_ZERO_CHECK)",
        2 => "debug.guard_trap: float divide by zero (QUE_DIV_ZERO_CHECK)",
        3 => "debug.guard_trap: integer overflow on add/inc (QUE_INT_OVERFLOW_CHECK)",
        4 => "debug.guard_trap: integer overflow on sub/dec (QUE_INT_OVERFLOW_CHECK)",
        5 => "debug.guard_trap: integer overflow on mul/square (QUE_INT_OVERFLOW_CHECK)",
        6 => "debug.guard_trap: float overflow or NaN/Inf (QUE_FLOAT_OVERFLOW_CHECK)",
        _ => "debug.guard_trap: unknown guard trap",
    };
    Some(msg.to_string())
}

fn configured_engine() -> Result<Engine, String> {
    let mut config = Config::new();
    config.strategy(Strategy::Cranelift);
    config.parallel_compilation(true);

    let opt = std::env::var("QUE_WASM_OPT").unwrap_or_else(|_| "speed".to_string());
    let opt_level = match opt.trim().to_ascii_lowercase().as_str() {
        "none" => OptLevel::None,
        "speed" => OptLevel::Speed,
        "speed_and_size" | "speed-size" | "size" => OptLevel::SpeedAndSize,
        other => {
            return Err(
                format!("invalid QUE_WASM_OPT='{}'. expected one of: none, speed, speed_and_size", other)
            );
        }
    };
    config.cranelift_opt_level(opt_level);

    Engine::new(&config).map_err(|e| format!("engine error: {}", e))
}

pub fn run_wat_text<T: 'static, F>(
    wat_src: &str,
    store_data: T,
    argv: &[String],
    link_imports: F
) -> Result<String, String>
    where F: FnOnce(&mut Linker<T>) -> Result<(), String>
{
    let typ = extract_type_from_wat(wat_src).unwrap_or_else(|| "Int".to_string());
    let wasm_bytes = wat_crate::parse_str(wat_src).map_err(|e| e.to_string())?;
    let engine = configured_engine()?;
    let module = WasmModule::new(&engine, &wasm_bytes).map_err(|e| format!("module error: {}", e))?;
    let mut linker = Linker::new(&engine);
    link_imports(&mut linker)?;
    let mut store = Store::new(&engine, store_data);
    let instance = linker
        .instantiate(&mut store, &module)
        .map_err(|e| format!("inst error: {:#}", e))?;

    set_argv_strings(&mut store, &instance, argv).map_err(|e| e.to_string())?;

    let memory = instance
        .get_memory(&mut store, "memory")
        .ok_or_else(|| "no exported memory".to_string())?;

    let main = instance
        .get_typed_func::<(), i32>(&mut store, "main")
        .map_err(|e| format!("main func error: {:#}", e))?;

    let ptr = match main.call(&mut store, ()) {
        Ok(v) => v,
        Err(e) => {
            let mut msg = format!("call error: {:#}", e);
            if let Some(guard_msg) = debug_guard_trap_message(&instance, &mut store) {
                msg.push('\n');
                msg.push_str(&guard_msg);
            }
            if let Some(debug) = debug_rc_snapshot(&instance, &mut store) {
                msg.push('\n');
                msg.push_str(&debug);
            }
            return Err(msg);
        }
    };
    decode_value(ptr, &typ, &memory, &store)
}

pub fn run_wat_text_no_result<T: 'static, F>(
    wat_src: &str,
    store_data: T,
    argv: &[String],
    link_imports: F
) -> Result<(), String>
    where F: FnOnce(&mut Linker<T>) -> Result<(), String>
{
    let wasm_bytes = wat_crate::parse_str(wat_src).map_err(|e| e.to_string())?;
    let engine = configured_engine()?;
    let module = WasmModule::new(&engine, &wasm_bytes).map_err(|e| format!("module error: {}", e))?;
    let mut linker = Linker::new(&engine);
    link_imports(&mut linker)?;
    let mut store = Store::new(&engine, store_data);
    let instance = linker
        .instantiate(&mut store, &module)
        .map_err(|e| format!("inst error: {:#}", e))?;

    set_argv_strings(&mut store, &instance, argv).map_err(|e| e.to_string())?;

    let main = instance
        .get_typed_func::<(), i32>(&mut store, "main")
        .map_err(|e| format!("main func error: {:#}", e))?;

    if let Err(e) = main.call(&mut store, ()) {
        let mut msg = format!("call error: {:#}", e);
        if let Some(guard_msg) = debug_guard_trap_message(&instance, &mut store) {
            msg.push('\n');
            msg.push_str(&guard_msg);
        }
        if let Some(debug) = debug_rc_snapshot(&instance, &mut store) {
            msg.push('\n');
            msg.push_str(&debug);
        }
        return Err(msg);
    }

    Ok(())
}
