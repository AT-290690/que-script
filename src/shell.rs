use std::process::Command;
use std::path::PathBuf;
use wasmtime::{Caller, Extern, Linker, Memory, TypedFunc};
use wasmtime_wasi::{ResourceTable, WasiCtx, WasiCtxBuilder, WasiCtxView, WasiView};

const VEC_LEN_OFFSET: i32 = 0;
const VEC_CAP_OFFSET: i32 = 4;
const VEC_RC_OFFSET: i32 = 8;
const VEC_ELEM_REF_OFFSET: i32 = 12;
const VEC_DATA_PTR_OFFSET: i32 = 16;
const VEC_HEADER_SIZE: i32 = 20;

pub struct ShellStoreData {
    pub wasi_ctx: WasiCtx,
    pub resource_table: ResourceTable,
    wasi_p1_ctx: wasmtime_wasi::p1::WasiP1Ctx,
    pub script_cwd: Option<PathBuf>,
}

impl ShellStoreData {
    pub fn new() -> wasmtime::Result<Self> {
        Self::new_with_script_cwd(None)
    }

    pub fn new_with_script_cwd(script_cwd: Option<PathBuf>) -> wasmtime::Result<Self> {
        let mut p2_builder = WasiCtxBuilder::new();
        p2_builder.inherit_stdio();
        p2_builder.inherit_args();
        p2_builder.inherit_env();

        let mut p1_builder = WasiCtxBuilder::new();
        p1_builder.inherit_stdio();
        p1_builder.inherit_args();
        p1_builder.inherit_env();

        Ok(Self {
            wasi_ctx: p2_builder.build(),
            resource_table: ResourceTable::new(),
            wasi_p1_ctx: p1_builder.build_p1(),
            script_cwd,
        })
    }
}

impl WasiView for ShellStoreData {
    fn ctx(&mut self) -> WasiCtxView<'_> {
        WasiCtxView {
            ctx: &mut self.wasi_ctx,
            table: &mut self.resource_table,
        }
    }
}

fn memory_export(caller: &mut Caller<'_, ShellStoreData>) -> wasmtime::Result<Memory> {
    caller
        .get_export("memory")
        .and_then(Extern::into_memory)
        .ok_or_else(|| wasmtime::Error::msg("guest export 'memory' not found"))
}

fn read_i32(
    memory: &Memory,
    caller: &Caller<'_, ShellStoreData>,
    addr: i32,
) -> wasmtime::Result<i32> {
    let offset = usize::try_from(addr)
        .map_err(|_| wasmtime::Error::msg(format!("invalid read address: {}", addr)))?;
    let mut bytes = [0u8; 4];
    memory
        .read(caller, offset, &mut bytes)
        .map_err(|_| wasmtime::Error::msg(format!("out of bounds read at {}", addr)))?;
    Ok(i32::from_le_bytes(bytes))
}

fn write_i32(
    memory: &Memory,
    caller: &mut Caller<'_, ShellStoreData>,
    addr: i32,
    value: i32,
) -> wasmtime::Result<()> {
    let offset = usize::try_from(addr)
        .map_err(|_| wasmtime::Error::msg(format!("invalid write address: {}", addr)))?;
    memory
        .write(caller, offset, &value.to_le_bytes())
        .map_err(|_| wasmtime::Error::msg(format!("out of bounds write at {}", addr)))
}

fn guest_alloc(caller: &mut Caller<'_, ShellStoreData>) -> wasmtime::Result<TypedFunc<i32, i32>> {
    for name in ["$alloc", "alloc"] {
        if let Some(func) = caller.get_export(name).and_then(Extern::into_func) {
            if let Ok(typed) = func.typed::<i32, i32>(&mut *caller) {
                return Ok(typed);
            }
        }
    }
    Err(wasmtime::Error::msg("guest export '$alloc'/'alloc' not found"))
}

pub fn read_lisp_vector(
    caller: &mut Caller<'_, ShellStoreData>,
    vec_ptr: i32,
) -> wasmtime::Result<Vec<i32>> {
    let memory = memory_export(caller)?;
    let len = read_i32(&memory, &*caller, vec_ptr + VEC_LEN_OFFSET)?;
    let data_ptr = read_i32(&memory, &*caller, vec_ptr + VEC_DATA_PTR_OFFSET)?;
    if len < 0 {
        return Err(wasmtime::Error::msg(format!("negative vector len: {}", len)));
    }

    let mut values = Vec::with_capacity(len as usize);
    for i in 0..len {
        values.push(read_i32(&memory, &*caller, data_ptr + i * 4)?);
    }
    Ok(values)
}

pub fn write_lisp_vector(
    caller: &mut Caller<'_, ShellStoreData>,
    values: &[i32],
) -> wasmtime::Result<i32> {
    let alloc = guest_alloc(caller)?;
    let vec_len = i32::try_from(values.len())
        .map_err(|_| wasmtime::Error::msg("output too large for i32 vector length"))?;
    let header_ptr = alloc.call(&mut *caller, VEC_HEADER_SIZE)?;
    let data_ptr = alloc.call(&mut *caller, vec_len * 4)?;
    let memory = memory_export(caller)?;

    for (i, value) in values.iter().copied().enumerate() {
        let offset = i32::try_from(i)
            .map_err(|_| wasmtime::Error::msg("output index overflow"))?
            * 4;
        write_i32(&memory, caller, data_ptr + offset, value)?;
    }

    write_i32(&memory, caller, header_ptr + VEC_LEN_OFFSET, vec_len)?;
    write_i32(&memory, caller, header_ptr + VEC_CAP_OFFSET, vec_len)?;
    write_i32(&memory, caller, header_ptr + VEC_RC_OFFSET, 1)?;
    write_i32(&memory, caller, header_ptr + VEC_ELEM_REF_OFFSET, 0)?;
    write_i32(&memory, caller, header_ptr + VEC_DATA_PTR_OFFSET, data_ptr)?;
    Ok(header_ptr)
}

pub fn host_run_shell(
    mut caller: Caller<'_, ShellStoreData>,
    cmd_vec_ptr: i32,
) -> wasmtime::Result<i32> {
    let cmd_codes = read_lisp_vector(&mut caller, cmd_vec_ptr)?;
    let cmd_str: String = cmd_codes
        .into_iter()
        .map(|n| char::from_u32(n as u32).unwrap_or('\u{FFFD}'))
        .collect();

    let mut command = Command::new("sh");
    command.arg("-c").arg(&cmd_str);
    if let Some(script_cwd) = caller.data().script_cwd.as_ref() {
        command.current_dir(script_cwd);
    }
    let output = command
        .output()
        .map_err(|e| wasmtime::Error::msg(format!("failed to run shell command: {}", e)))?;

    let mut combined = String::new();
    combined.push_str(&String::from_utf8_lossy(&output.stdout));
    combined.push_str(&String::from_utf8_lossy(&output.stderr));

    let output_codes = combined
        .chars()
        .map(|c| i32::try_from(u32::from(c)).unwrap_or(0))
        .collect::<Vec<_>>();
    write_lisp_vector(&mut caller, &output_codes)
}

pub fn add_shell_to_linker(linker: &mut Linker<ShellStoreData>) -> wasmtime::Result<()> {
    // Core wasm modules (like this backend) use WASIp1 imports.
    wasmtime_wasi::p1::add_to_linker_sync(linker, |state| &mut state.wasi_p1_ctx)?;
    linker.func_wrap("host", "run_shell", host_run_shell)?;
    Ok(())
}
