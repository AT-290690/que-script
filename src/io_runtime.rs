use super::*;

pub struct ShellStoreData {
    pub wasi_ctx: WasiCtx,
    pub resource_table: ResourceTable,
    wasi_p1_ctx: wasmtime_wasi::p1::WasiP1Ctx,
    pub script_cwd: Option<PathBuf>,
    pub shell_policy: ShellPolicy,
}

impl ShellStoreData {
    pub fn new_with_security(
        script_cwd: Option<PathBuf>,
        shell_policy: ShellPolicy,
    ) -> wasmtime::Result<Self> {
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
            shell_policy,
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
    Err(wasmtime::Error::msg(
        "guest export '$alloc'/'alloc' not found",
    ))
}

pub fn read_lisp_vector(
    caller: &mut Caller<'_, ShellStoreData>,
    vec_ptr: i32,
) -> wasmtime::Result<Vec<i32>> {
    let memory = memory_export(caller)?;
    let len = read_i32(&memory, &*caller, vec_ptr + VEC_LEN_OFFSET)?;
    let data_ptr = read_i32(&memory, &*caller, vec_ptr + VEC_DATA_PTR_OFFSET)?;
    if len < 0 {
        return Err(wasmtime::Error::msg(format!(
            "negative vector len: {}",
            len
        )));
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
        let offset =
            i32::try_from(i).map_err(|_| wasmtime::Error::msg("output index overflow"))? * 4;
        write_i32(&memory, caller, data_ptr + offset, value)?;
    }

    write_i32(&memory, caller, header_ptr + VEC_LEN_OFFSET, vec_len)?;
    write_i32(&memory, caller, header_ptr + VEC_CAP_OFFSET, vec_len)?;
    write_i32(&memory, caller, header_ptr + VEC_RC_OFFSET, 1)?;
    write_i32(&memory, caller, header_ptr + VEC_ELEM_REF_OFFSET, 0)?;
    write_i32(&memory, caller, header_ptr + VEC_DATA_PTR_OFFSET, data_ptr)?;
    write_i32(&memory, caller, header_ptr + VEC_MAGIC_OFFSET, VEC_MAGIC)?;
    Ok(header_ptr)
}

pub fn write_lisp_byte_vector(
    caller: &mut Caller<'_, ShellStoreData>,
    bytes: &[u8],
) -> wasmtime::Result<i32> {
    let alloc = guest_alloc(caller)?;
    let vec_len = i32::try_from(bytes.len())
        .map_err(|_| wasmtime::Error::msg("output too large for i32 vector length"))?;
    let data_bytes_len = vec_len
        .checked_mul(4)
        .ok_or_else(|| wasmtime::Error::msg("output byte length overflow"))?;
    let header_ptr = alloc.call(&mut *caller, VEC_HEADER_SIZE)?;
    let data_ptr = alloc.call(&mut *caller, data_bytes_len)?;
    let memory = memory_export(caller)?;

    let data_offset = usize::try_from(data_ptr)
        .map_err(|_| wasmtime::Error::msg(format!("invalid write address: {}", data_ptr)))?;
    let data_len = usize::try_from(data_bytes_len)
        .map_err(|_| wasmtime::Error::msg("output byte length overflow"))?;
    let data_end = data_offset
        .checked_add(data_len)
        .ok_or_else(|| wasmtime::Error::msg("output write address overflow"))?;

    {
        let data = memory.data_mut(&mut *caller);
        let output = data
            .get_mut(data_offset..data_end)
            .ok_or_else(|| wasmtime::Error::msg(format!("out of bounds write at {}", data_ptr)))?;
        for (i, byte) in bytes.iter().copied().enumerate() {
            let offset = i * 4;
            output[offset..offset + 4].copy_from_slice(&i32::from(byte).to_le_bytes());
        }
    }

    write_i32(&memory, caller, header_ptr + VEC_LEN_OFFSET, vec_len)?;
    write_i32(&memory, caller, header_ptr + VEC_CAP_OFFSET, vec_len)?;
    write_i32(&memory, caller, header_ptr + VEC_RC_OFFSET, 1)?;
    write_i32(&memory, caller, header_ptr + VEC_ELEM_REF_OFFSET, 0)?;
    write_i32(&memory, caller, header_ptr + VEC_DATA_PTR_OFFSET, data_ptr)?;
    write_i32(&memory, caller, header_ptr + VEC_MAGIC_OFFSET, VEC_MAGIC)?;
    Ok(header_ptr)
}

fn scalar_byte_vector_region(
    caller: &mut Caller<'_, ShellStoreData>,
    vec_ptr: i32,
) -> wasmtime::Result<(usize, usize)> {
    let memory = memory_export(caller)?;
    let len = read_i32(&memory, &*caller, vec_ptr + VEC_LEN_OFFSET)?;
    let elem_ref = read_i32(&memory, &*caller, vec_ptr + VEC_ELEM_REF_OFFSET)?;
    let data_ptr = read_i32(&memory, &*caller, vec_ptr + VEC_DATA_PTR_OFFSET)?;
    let magic = read_i32(&memory, &*caller, vec_ptr + VEC_MAGIC_OFFSET)?;
    if len <= 0 {
        return Err(wasmtime::Error::msg(format!(
            "read/buffer! buffer length must be positive, got {}",
            len
        )));
    }
    if elem_ref != 0 {
        return Err(wasmtime::Error::msg(
            "read/buffer! requires a scalar [Int] byte buffer",
        ));
    }
    if magic != VEC_MAGIC {
        return Err(wasmtime::Error::msg("read/buffer! received invalid vector"));
    }
    let len_usize = usize::try_from(len)
        .map_err(|_| wasmtime::Error::msg("read/buffer! buffer length overflow"))?;
    let data_offset = usize::try_from(data_ptr)
        .map_err(|_| wasmtime::Error::msg(format!("invalid write address: {}", data_ptr)))?;
    Ok((len_usize, data_offset))
}

pub fn read_lisp_string(
    caller: &mut Caller<'_, ShellStoreData>,
    vec_ptr: i32,
) -> wasmtime::Result<String> {
    let codes = read_lisp_vector(caller, vec_ptr)?;
    Ok(codes
        .into_iter()
        .map(|n| char::from_u32(n as u32).unwrap_or('\u{FFFD}'))
        .collect::<String>())
}

pub fn write_lisp_string(
    caller: &mut Caller<'_, ShellStoreData>,
    value: &str,
) -> wasmtime::Result<i32> {
    let codes = value
        .chars()
        .map(|c| i32::try_from(u32::from(c)).unwrap_or(0))
        .collect::<Vec<_>>();
    write_lisp_vector(caller, &codes)
}

fn normalize_lexical_path(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Prefix(prefix) => out.push(prefix.as_os_str()),
            Component::RootDir => out.push(component.as_os_str()),
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            Component::Normal(part) => out.push(part),
        }
    }
    out
}

fn sandbox_root(caller: &Caller<'_, ShellStoreData>) -> Result<PathBuf, String> {
    let root = caller
        .data()
        .script_cwd
        .clone()
        .unwrap_or_else(|| env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
    fs::canonicalize(&root).map_err(|e| {
        format!(
            "failed to resolve io sandbox root '{}': {}",
            root.display(),
            e
        )
    })
}

fn resolve_target_path(caller: &Caller<'_, ShellStoreData>, raw: &str) -> Result<PathBuf, String> {
    let candidate = Path::new(raw);
    let root = sandbox_root(caller)?;
    let target = if candidate.is_absolute() {
        normalize_lexical_path(candidate)
    } else {
        normalize_lexical_path(&root.join(candidate))
    };

    if !target.starts_with(&root) {
        return Err(format!(
            "path '{}' escapes io sandbox root '{}'",
            raw,
            root.display()
        ));
    }

    Ok(target)
}

fn ensure_existing_path_in_sandbox(
    caller: &Caller<'_, ShellStoreData>,
    raw: &str,
    target: &Path,
) -> Result<(), String> {
    let root = sandbox_root(caller)?;
    let real = fs::canonicalize(target)
        .map_err(|e| format!("failed to resolve '{}': {}", target.display(), e))?;
    if !real.starts_with(&root) {
        return Err(format!(
            "path '{}' resolves outside io sandbox root '{}'",
            raw,
            root.display()
        ));
    }
    Ok(())
}

fn ensure_parent_in_sandbox(
    caller: &Caller<'_, ShellStoreData>,
    raw: &str,
    target: &Path,
) -> Result<(), String> {
    let root = sandbox_root(caller)?;
    let parent = target.parent().unwrap_or(&root);
    let mut existing = parent;
    while !existing.exists() {
        existing = existing.parent().unwrap_or(&root);
    }
    let real = fs::canonicalize(existing)
        .map_err(|e| format!("failed to resolve '{}': {}", existing.display(), e))?;
    if !real.starts_with(&root) {
        return Err(format!(
            "path '{}' parent resolves outside io sandbox root '{}'",
            raw,
            root.display()
        ));
    }
    Ok(())
}

fn list_dir_text(path: &Path) -> Result<String, String> {
    let entries = fs::read_dir(path)
        .map_err(|e: io::Error| format!("failed to read directory '{}': {}", path.display(), e))?;
    let mut names = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|e: io::Error| format!("failed to read dir entry: {}", e))?;
        names.push(entry.file_name().to_string_lossy().into_owned());
    }
    names.sort();
    if names.is_empty() {
        Ok(String::new())
    } else {
        Ok(format!("{}\n", names.join("\n")))
    }
}

pub fn host_list_dir(
    mut caller: Caller<'_, ShellStoreData>,
    path_vec_ptr: i32,
) -> wasmtime::Result<i32> {
    let path = read_lisp_string(&mut caller, path_vec_ptr)?;
    caller
        .data()
        .shell_policy
        .require(ShellPermission::Read, "list-dir!", &path)
        .map_err(wasmtime::Error::msg)?;

    let target = resolve_target_path(&caller, &path).map_err(wasmtime::Error::msg)?;
    ensure_existing_path_in_sandbox(&caller, &path, &target).map_err(wasmtime::Error::msg)?;
    let output = list_dir_text(&target).map_err(wasmtime::Error::msg)?;
    write_lisp_string(&mut caller, &output)
}

pub fn host_read_file(
    mut caller: Caller<'_, ShellStoreData>,
    path_vec_ptr: i32,
) -> wasmtime::Result<i32> {
    let path = read_lisp_string(&mut caller, path_vec_ptr)?;
    caller
        .data()
        .shell_policy
        .require(ShellPermission::Read, "read!", &path)
        .map_err(wasmtime::Error::msg)?;

    let target = resolve_target_path(&caller, &path).map_err(wasmtime::Error::msg)?;
    ensure_existing_path_in_sandbox(&caller, &path, &target).map_err(wasmtime::Error::msg)?;
    let output = fs::read_to_string(&target).map_err(|e| {
        wasmtime::Error::msg(format!("failed to read '{}': {}", target.display(), e))
    })?;
    write_lisp_string(&mut caller, &output)
}

pub fn host_read_stdin(mut caller: Caller<'_, ShellStoreData>) -> wasmtime::Result<i32> {
    caller
        .data()
        .shell_policy
        .require(ShellPermission::Stdin, "stdin!", "<stdin>")
        .map_err(wasmtime::Error::msg)?;

    let mut output = String::new();
    io::stdin()
        .read_to_string(&mut output)
        .map_err(|e| wasmtime::Error::msg(format!("failed to read stdin: {}", e)))?;
    write_lisp_string(&mut caller, &output)
}

fn guest_apply1(
    caller: &mut Caller<'_, ShellStoreData>,
) -> wasmtime::Result<TypedFunc<(i32, i32), i32>> {
    for name in ["$apply1_i32", "apply1_i32"] {
        if let Some(func) = caller.get_export(name).and_then(Extern::into_func) {
            if let Ok(typed) = func.typed::<(i32, i32), i32>(&mut *caller) {
                return Ok(typed);
            }
        }
    }
    Err(wasmtime::Error::msg(
        "guest export '$apply1_i32'/'apply1_i32' not found",
    ))
}

fn guest_rc_release(
    caller: &mut Caller<'_, ShellStoreData>,
) -> wasmtime::Result<TypedFunc<i32, i32>> {
    for name in ["$rc_release", "rc_release", "release"] {
        if let Some(func) = caller.get_export(name).and_then(Extern::into_func) {
            if let Ok(typed) = func.typed::<i32, i32>(&mut *caller) {
                return Ok(typed);
            }
        }
    }
    Err(wasmtime::Error::msg(
        "guest export '$rc_release'/'rc_release' not found",
    ))
}

pub fn host_read_chunks(
    mut caller: Caller<'_, ShellStoreData>,
    path_vec_ptr: i32,
    chunk_size: i32,
    callback: i32,
) -> wasmtime::Result<i32> {
    if chunk_size <= 0 {
        return Err(wasmtime::Error::msg(format!(
            "read/chunks! chunk size must be positive, got {}",
            chunk_size
        )));
    }

    let path = read_lisp_string(&mut caller, path_vec_ptr)?;
    caller
        .data()
        .shell_policy
        .require(ShellPermission::Read, "read/chunks!", &path)
        .map_err(wasmtime::Error::msg)?;

    let target = resolve_target_path(&caller, &path).map_err(wasmtime::Error::msg)?;
    ensure_existing_path_in_sandbox(&caller, &path, &target).map_err(wasmtime::Error::msg)?;
    let mut file = fs::File::open(&target).map_err(|e| {
        wasmtime::Error::msg(format!("failed to open '{}': {}", target.display(), e))
    })?;
    let apply1 = guest_apply1(&mut caller)?;
    let rc_release = guest_rc_release(&mut caller)?;
    let mut buffer = vec![0u8; chunk_size as usize];

    loop {
        let n = file.read(&mut buffer).map_err(|e| {
            wasmtime::Error::msg(format!("failed to read '{}': {}", target.display(), e))
        })?;
        if n == 0 {
            break;
        }

        let chunk_ptr = write_lisp_byte_vector(&mut caller, &buffer[..n])?;
        let should_stop = apply1.call(&mut caller, (callback, chunk_ptr))?;
        let _ = rc_release.call(&mut caller, chunk_ptr)?;
        if should_stop != 0 {
            return Ok(1);
        }
    }

    Ok(0)
}

pub fn host_read_buffer(
    mut caller: Caller<'_, ShellStoreData>,
    buffer_vec_ptr: i32,
    path_vec_ptr: i32,
    callback: i32,
) -> wasmtime::Result<i32> {
    let path = read_lisp_string(&mut caller, path_vec_ptr)?;
    caller
        .data()
        .shell_policy
        .require(ShellPermission::Read, "read/buffer!", &path)
        .map_err(wasmtime::Error::msg)?;

    let target = resolve_target_path(&caller, &path).map_err(wasmtime::Error::msg)?;
    ensure_existing_path_in_sandbox(&caller, &path, &target).map_err(wasmtime::Error::msg)?;
    let mut file = fs::File::open(&target).map_err(|e| {
        wasmtime::Error::msg(format!("failed to open '{}': {}", target.display(), e))
    })?;
    let apply1 = guest_apply1(&mut caller)?;

    let (buffer_len, data_offset) = scalar_byte_vector_region(&mut caller, buffer_vec_ptr)?;
    let byte_width = buffer_len
        .checked_mul(4)
        .ok_or_else(|| wasmtime::Error::msg("read/buffer! buffer size overflow"))?;
    let mut host_buffer = vec![0u8; buffer_len];
    let memory = memory_export(&mut caller)?;

    loop {
        let n = file.read(&mut host_buffer).map_err(|e| {
            wasmtime::Error::msg(format!("failed to read '{}': {}", target.display(), e))
        })?;
        if n == 0 {
            break;
        }

        {
            let end = data_offset
                .checked_add(byte_width)
                .ok_or_else(|| wasmtime::Error::msg("read/buffer! write address overflow"))?;
            let data = memory.data_mut(&mut caller);
            let output = data.get_mut(data_offset..end).ok_or_else(|| {
                wasmtime::Error::msg(format!("out of bounds write at {}", data_offset))
            })?;
            for (i, byte) in host_buffer[..n].iter().copied().enumerate() {
                let offset = i * 4;
                output[offset..offset + 4].copy_from_slice(&i32::from(byte).to_le_bytes());
            }
        }

        let should_stop = apply1.call(
            &mut caller,
            (
                callback,
                i32::try_from(n)
                    .map_err(|_| wasmtime::Error::msg("read/buffer! byte count overflow"))?,
            ),
        )?;
        if should_stop != 0 {
            return Ok(1);
        }
    }

    Ok(0)
}

pub fn host_read_stdin_chunks(
    mut caller: Caller<'_, ShellStoreData>,
    chunk_size: i32,
    callback: i32,
) -> wasmtime::Result<i32> {
    if chunk_size <= 0 {
        return Err(wasmtime::Error::msg(format!(
            "stdin/chunks! chunk size must be positive, got {}",
            chunk_size
        )));
    }

    caller
        .data()
        .shell_policy
        .require(ShellPermission::Stdin, "stdin/chunks!", "<stdin>")
        .map_err(wasmtime::Error::msg)?;

    let apply1 = guest_apply1(&mut caller)?;
    let rc_release = guest_rc_release(&mut caller)?;
    let mut input = io::stdin().lock();
    let mut buffer = vec![0u8; chunk_size as usize];

    loop {
        let n = input
            .read(&mut buffer)
            .map_err(|e| wasmtime::Error::msg(format!("failed to read stdin: {}", e)))?;
        if n == 0 {
            break;
        }

        let chunk_ptr = write_lisp_byte_vector(&mut caller, &buffer[..n])?;
        let should_stop = apply1.call(&mut caller, (callback, chunk_ptr))?;
        let _ = rc_release.call(&mut caller, chunk_ptr)?;
        if should_stop != 0 {
            return Ok(1);
        }
    }

    Ok(0)
}

pub fn host_read_lines(
    mut caller: Caller<'_, ShellStoreData>,
    path_vec_ptr: i32,
    callback: i32,
) -> wasmtime::Result<i32> {
    let path = read_lisp_string(&mut caller, path_vec_ptr)?;
    caller
        .data()
        .shell_policy
        .require(ShellPermission::Read, "read/lines!", &path)
        .map_err(wasmtime::Error::msg)?;

    let target = resolve_target_path(&caller, &path).map_err(wasmtime::Error::msg)?;
    ensure_existing_path_in_sandbox(&caller, &path, &target).map_err(wasmtime::Error::msg)?;
    let file = fs::File::open(&target).map_err(|e| {
        wasmtime::Error::msg(format!("failed to open '{}': {}", target.display(), e))
    })?;
    let mut reader = io::BufReader::new(file);
    let apply1 = guest_apply1(&mut caller)?;
    let rc_release = guest_rc_release(&mut caller)?;
    let mut line = Vec::new();

    loop {
        line.clear();
        let n = reader.read_until(b'\n', &mut line).map_err(|e| {
            wasmtime::Error::msg(format!("failed to read '{}': {}", target.display(), e))
        })?;
        if n == 0 {
            break;
        }

        if line.last() == Some(&b'\n') {
            line.pop();
            if line.last() == Some(&b'\r') {
                line.pop();
            }
        }

        let line_ptr = write_lisp_byte_vector(&mut caller, &line)?;
        let should_stop = apply1.call(&mut caller, (callback, line_ptr))?;
        let _ = rc_release.call(&mut caller, line_ptr)?;
        if should_stop != 0 {
            return Ok(1);
        }
    }

    Ok(0)
}

pub fn host_write_file(
    mut caller: Caller<'_, ShellStoreData>,
    path_vec_ptr: i32,
    data_vec_ptr: i32,
) -> wasmtime::Result<i32> {
    let path = read_lisp_string(&mut caller, path_vec_ptr)?;
    let data = read_lisp_string(&mut caller, data_vec_ptr)?;
    caller
        .data()
        .shell_policy
        .require(ShellPermission::Write, "write!", &path)
        .map_err(wasmtime::Error::msg)?;

    let target = resolve_target_path(&caller, &path).map_err(wasmtime::Error::msg)?;
    if target.exists() {
        ensure_existing_path_in_sandbox(&caller, &path, &target).map_err(wasmtime::Error::msg)?;
    } else {
        ensure_parent_in_sandbox(&caller, &path, &target).map_err(wasmtime::Error::msg)?;
    }
    if let Some(parent) = target.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).map_err(|e| {
                wasmtime::Error::msg(format!(
                    "failed to create parent dirs '{}': {}",
                    parent.display(),
                    e
                ))
            })?;
        }
    }
    fs::write(&target, data.as_bytes()).map_err(|e| {
        wasmtime::Error::msg(format!("failed to write '{}': {}", target.display(), e))
    })?;

    Ok(0)
}

pub fn host_mkdir_p(
    mut caller: Caller<'_, ShellStoreData>,
    path_vec_ptr: i32,
) -> wasmtime::Result<i32> {
    let path = read_lisp_string(&mut caller, path_vec_ptr)?;
    caller
        .data()
        .shell_policy
        .require(ShellPermission::Write, "mkdir!", &path)
        .map_err(wasmtime::Error::msg)?;

    let target = resolve_target_path(&caller, &path).map_err(wasmtime::Error::msg)?;
    if target.exists() {
        ensure_existing_path_in_sandbox(&caller, &path, &target).map_err(wasmtime::Error::msg)?;
    } else {
        ensure_parent_in_sandbox(&caller, &path, &target).map_err(wasmtime::Error::msg)?;
    }
    fs::create_dir_all(&target).map_err(|e| {
        wasmtime::Error::msg(format!("failed to mkdir '{}': {}", target.display(), e))
    })?;
    Ok(0)
}

pub fn host_delete(
    mut caller: Caller<'_, ShellStoreData>,
    path_vec_ptr: i32,
) -> wasmtime::Result<i32> {
    let path = read_lisp_string(&mut caller, path_vec_ptr)?;
    caller
        .data()
        .shell_policy
        .require(ShellPermission::Delete, "delete!", &path)
        .map_err(wasmtime::Error::msg)?;

    let target = resolve_target_path(&caller, &path).map_err(wasmtime::Error::msg)?;
    ensure_existing_path_in_sandbox(&caller, &path, &target).map_err(wasmtime::Error::msg)?;
    let meta = fs::symlink_metadata(&target).map_err(|e| {
        wasmtime::Error::msg(format!(
            "failed to inspect path '{}' for delete: {}",
            target.display(),
            e
        ))
    })?;
    if meta.is_dir() {
        fs::remove_dir_all(&target).map_err(|e| {
            wasmtime::Error::msg(format!(
                "failed to delete directory '{}': {}",
                target.display(),
                e
            ))
        })?;
    } else {
        fs::remove_file(&target).map_err(|e| {
            wasmtime::Error::msg(format!(
                "failed to delete file '{}': {}",
                target.display(),
                e
            ))
        })?;
    }
    Ok(0)
}

pub fn host_move(
    mut caller: Caller<'_, ShellStoreData>,
    src_vec_ptr: i32,
    dst_vec_ptr: i32,
) -> wasmtime::Result<i32> {
    let src = read_lisp_string(&mut caller, src_vec_ptr)?;
    let dst = read_lisp_string(&mut caller, dst_vec_ptr)?;
    caller
        .data()
        .shell_policy
        .require(
            ShellPermission::Write,
            "move!",
            &format!("{} -> {}", src, dst),
        )
        .map_err(wasmtime::Error::msg)?;

    let src_path = resolve_target_path(&caller, &src).map_err(wasmtime::Error::msg)?;
    ensure_existing_path_in_sandbox(&caller, &src, &src_path).map_err(wasmtime::Error::msg)?;
    let dst_path = resolve_target_path(&caller, &dst).map_err(wasmtime::Error::msg)?;
    ensure_parent_in_sandbox(&caller, &dst, &dst_path).map_err(wasmtime::Error::msg)?;
    if let Some(parent) = dst_path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).map_err(|e| {
                wasmtime::Error::msg(format!(
                    "failed to create destination dirs '{}': {}",
                    parent.display(),
                    e
                ))
            })?;
        }
    }
    fs::rename(&src_path, &dst_path).map_err(|e| {
        wasmtime::Error::msg(format!(
            "failed to move '{}' to '{}': {}",
            src_path.display(),
            dst_path.display(),
            e
        ))
    })?;

    Ok(0)
}

pub fn host_print(
    mut caller: Caller<'_, ShellStoreData>,
    text_vec_ptr: i32,
) -> wasmtime::Result<i32> {
    let text = read_lisp_string(&mut caller, text_vec_ptr)?;
    caller
        .data()
        .shell_policy
        .require(ShellPermission::Print, "print!", "<stdout>")
        .map_err(wasmtime::Error::msg)?;

    let mut out = io::stdout();
    out.write_all(text.as_bytes())
        .map_err(|e| wasmtime::Error::msg(format!("failed to write stdout: {}", e)))?;
    out.flush()
        .map_err(|e| wasmtime::Error::msg(format!("failed to flush stdout: {}", e)))?;
    Ok(0)
}

pub fn host_sleep(caller: Caller<'_, ShellStoreData>, millis: i32) -> wasmtime::Result<i32> {
    caller
        .data()
        .shell_policy
        .require(ShellPermission::Clock, "sleep!", "<clock>")
        .map_err(wasmtime::Error::msg)?;

    if millis < 0 {
        return Err(wasmtime::Error::msg(format!(
            "sleep! expects non-negative ms, got {}",
            millis
        )));
    }
    thread::sleep(Duration::from_millis(millis as u64));
    Ok(0)
}

pub fn host_time(caller: Caller<'_, ShellStoreData>) -> wasmtime::Result<i32> {
    caller
        .data()
        .shell_policy
        .require(ShellPermission::Clock, "time!", "<clock>")
        .map_err(wasmtime::Error::msg)?;

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| wasmtime::Error::msg(format!("clock error: {}", e)))?;
    i32::try_from(now.as_secs()).map_err(|_| wasmtime::Error::msg("time! overflowed i32"))
}

pub fn host_clear(caller: Caller<'_, ShellStoreData>) -> wasmtime::Result<i32> {
    caller
        .data()
        .shell_policy
        .require(ShellPermission::Print, "clear!", "<stdout>")
        .map_err(wasmtime::Error::msg)?;

    let mut out = io::stdout();
    out.write_all(b"\x1b[2J\x1b[H")
        .map_err(|e| wasmtime::Error::msg(format!("failed to clear stdout: {}", e)))?;
    out.flush()
        .map_err(|e| wasmtime::Error::msg(format!("failed to flush stdout: {}", e)))?;
    Ok(0)
}

fn register_builtin_host_import(
    linker: &mut Linker<ShellStoreData>,
    spec: &crate::externals::BuiltinHostExternSpec,
) -> wasmtime::Result<()> {
    match spec.import {
        "list_dir" => {
            linker.func_wrap(spec.module, spec.import, host_list_dir)?;
        }
        "read_file" => {
            linker.func_wrap(spec.module, spec.import, host_read_file)?;
        }
        "read_stdin" => {
            linker.func_wrap(spec.module, spec.import, host_read_stdin)?;
        }
        "read_chunks" => {
            linker.func_wrap(spec.module, spec.import, host_read_chunks)?;
        }
        "read_buffer" => {
            linker.func_wrap(spec.module, spec.import, host_read_buffer)?;
        }
        "read_stdin_chunks" => {
            linker.func_wrap(spec.module, spec.import, host_read_stdin_chunks)?;
        }
        "read_lines" => {
            linker.func_wrap(spec.module, spec.import, host_read_lines)?;
        }
        "write_file" => {
            linker.func_wrap(spec.module, spec.import, host_write_file)?;
        }
        "mkdir_p" => {
            linker.func_wrap(spec.module, spec.import, host_mkdir_p)?;
        }
        "delete" => {
            linker.func_wrap(spec.module, spec.import, host_delete)?;
        }
        "move" => {
            linker.func_wrap(spec.module, spec.import, host_move)?;
        }
        "print" => {
            linker.func_wrap(spec.module, spec.import, host_print)?;
        }
        "sleep" => {
            linker.func_wrap(spec.module, spec.import, host_sleep)?;
        }
        "time" => {
            linker.func_wrap(spec.module, spec.import, host_time)?;
        }
        "clear" => {
            linker.func_wrap(spec.module, spec.import, host_clear)?;
        }
        other => {
            return Err(wasmtime::Error::msg(format!(
                "unsupported builtin host extern registration: {}::{}",
                spec.module, other
            )));
        }
    }
    Ok(())
}

pub fn add_shell_to_linker(linker: &mut Linker<ShellStoreData>) -> wasmtime::Result<()> {
    // Core wasm modules (like this backend) use WASIp1 imports.
    wasmtime_wasi::p1::add_to_linker_sync(linker, |state| &mut state.wasi_p1_ctx)?;
    for spec in crate::externals::BUILTIN_HOST_EXTERNS {
        register_builtin_host_import(linker, spec)?;
    }
    Ok(())
}
