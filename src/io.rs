use std::collections::HashSet;
use std::env;
use std::fs;
use std::io;
use std::path::{ Path, PathBuf };
use std::process::Command;
use wasmtime::{ Caller, Extern, Memory, TypedFunc };
use wasmtime::Linker;
use wasmtime_wasi::{ ResourceTable, WasiCtx, WasiCtxBuilder, WasiCtxView, WasiView };

const VEC_LEN_OFFSET: i32 = 0;
const VEC_CAP_OFFSET: i32 = 4;
const VEC_RC_OFFSET: i32 = 8;
const VEC_ELEM_REF_OFFSET: i32 = 12;
const VEC_DATA_PTR_OFFSET: i32 = 16;
const VEC_HEADER_SIZE: i32 = 20;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ShellPermission {
    Read,
    Write,
    Delete,
    Network,
}

impl ShellPermission {
    fn as_str(&self) -> &'static str {
        match self {
            ShellPermission::Read => "read",
            ShellPermission::Write => "write",
            ShellPermission::Delete => "delete",
            ShellPermission::Network => "network",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShellPolicy {
    shell_enabled: bool,
    permissions: HashSet<ShellPermission>,
}

impl ShellPolicy {
    pub fn disabled() -> Self {
        Self {
            shell_enabled: false,
            permissions: HashSet::new(),
        }
    }

    fn enabled(permissions: HashSet<ShellPermission>) -> Self {
        Self {
            shell_enabled: true,
            permissions,
        }
    }

    fn allows(&self, permission: ShellPermission) -> bool {
        self.permissions.contains(&permission)
    }

    pub fn require(
        &self,
        permission: ShellPermission,
        operation: &str,
        target: &str
    ) -> Result<(), String> {
        if !self.shell_enabled {
            return Err(
                format!(
                    "host io is disabled. pass --allow <read|write|delete|network> [...]. denied operation '{}' for '{}'",
                    operation,
                    target
                )
            );
        }

        if !self.allows(permission) {
            return Err(
                format!(
                    "permission '{}' is required for operation '{}'. denied target: {}",
                    permission.as_str(),
                    operation,
                    target
                )
            );
        }

        Ok(())
    }
}

fn parse_shell_policy_permissions(parts: &[String]) -> Result<ShellPolicy, String> {
    let mut permissions = HashSet::new();
    let mut grant_all = false;

    for part in parts {
        for fragment in part.split(',') {
            let token = fragment.trim().trim_matches('"').trim_matches('\'').to_ascii_lowercase();
            if token.is_empty() {
                continue;
            }
            match token.as_str() {
                "read" => {
                    permissions.insert(ShellPermission::Read);
                }
                "write" => {
                    permissions.insert(ShellPermission::Write);
                }
                "delete" => {
                    permissions.insert(ShellPermission::Delete);
                }
                "network" => {
                    permissions.insert(ShellPermission::Network);
                }
                "all" | "*" => {
                    grant_all = true;
                }
                _ => {
                    return Err(
                        format!("unknown shell permission '{}'. expected one of: read, write, delete, network", token)
                    );
                }
            }
        }
    }

    if grant_all {
        permissions.insert(ShellPermission::Read);
        permissions.insert(ShellPermission::Write);
        permissions.insert(ShellPermission::Delete);
        permissions.insert(ShellPermission::Network);
    }

    Ok(ShellPolicy::enabled(permissions))
}

pub fn take_shell_policy_from_argv(argv: &mut Vec<String>) -> Result<ShellPolicy, String> {
    if let Some(pos) = argv.iter().position(|arg| arg == "--allow") {
        let permissions = argv[pos + 1..].to_vec();
        argv.truncate(pos);
        return parse_shell_policy_permissions(&permissions);
    }

    Ok(ShellPolicy::disabled())
}

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
        shell_policy: ShellPolicy
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
    addr: i32
) -> wasmtime::Result<i32> {
    let offset = usize
        ::try_from(addr)
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
    value: i32
) -> wasmtime::Result<()> {
    let offset = usize
        ::try_from(addr)
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
    vec_ptr: i32
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
    values: &[i32]
) -> wasmtime::Result<i32> {
    let alloc = guest_alloc(caller)?;
    let vec_len = i32
        ::try_from(values.len())
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
    Ok(header_ptr)
}

fn read_lisp_string(
    caller: &mut Caller<'_, ShellStoreData>,
    vec_ptr: i32
) -> wasmtime::Result<String> {
    let codes = read_lisp_vector(caller, vec_ptr)?;
    Ok(
        codes
            .into_iter()
            .map(|n| char::from_u32(n as u32).unwrap_or('\u{FFFD}'))
            .collect::<String>()
    )
}

fn write_lisp_string(
    caller: &mut Caller<'_, ShellStoreData>,
    value: &str
) -> wasmtime::Result<i32> {
    let codes = value
        .chars()
        .map(|c| i32::try_from(u32::from(c)).unwrap_or(0))
        .collect::<Vec<_>>();
    write_lisp_vector(caller, &codes)
}

fn resolve_target_path(caller: &Caller<'_, ShellStoreData>, raw: &str) -> PathBuf {
    let candidate = Path::new(raw);
    if candidate.is_absolute() {
        return candidate.to_path_buf();
    }

    if let Some(script_cwd) = caller.data().script_cwd.as_ref() {
        return script_cwd.join(candidate);
    }

    candidate.to_path_buf()
}

fn split_shell_like_args(input: &str) -> Result<Vec<String>, String> {
    let mut args = Vec::new();
    let mut token = String::new();
    let mut in_single = false;
    let mut in_double = false;
    let mut escaped = false;

    let push_token = |token: &mut String, args: &mut Vec<String>| {
        if !token.is_empty() {
            args.push(std::mem::take(token));
        }
    };

    for ch in input.chars() {
        if escaped {
            token.push(ch);
            escaped = false;
            continue;
        }

        if in_single {
            if ch == '\'' {
                in_single = false;
            } else {
                token.push(ch);
            }
            continue;
        }

        if in_double {
            if ch == '"' {
                in_double = false;
            } else if ch == '\\' {
                escaped = true;
            } else {
                token.push(ch);
            }
            continue;
        }

        match ch {
            '\\' => {
                escaped = true;
            }
            '\'' => {
                in_single = true;
            }
            '"' => {
                in_double = true;
            }
            c if c.is_whitespace() => push_token(&mut token, &mut args),
            _ => token.push(ch),
        }
    }

    if in_single || in_double {
        return Err("unterminated quoted string in curl args".to_string());
    }

    if escaped {
        token.push('\\');
    }

    push_token(&mut token, &mut args);
    Ok(args)
}

fn list_dir_text(path: &Path) -> Result<String, String> {
    let entries = fs
        ::read_dir(path)
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
    path_vec_ptr: i32
) -> wasmtime::Result<i32> {
    let path = read_lisp_string(&mut caller, path_vec_ptr)?;
    caller
        .data()
        .shell_policy.require(ShellPermission::Read, "list-dir!", &path)
        .map_err(wasmtime::Error::msg)?;

    let target = resolve_target_path(&caller, &path);
    let output = list_dir_text(&target).map_err(wasmtime::Error::msg)?;
    write_lisp_string(&mut caller, &output)
}

pub fn host_read_file(
    mut caller: Caller<'_, ShellStoreData>,
    path_vec_ptr: i32
) -> wasmtime::Result<i32> {
    let path = read_lisp_string(&mut caller, path_vec_ptr)?;
    caller
        .data()
        .shell_policy.require(ShellPermission::Read, "read!", &path)
        .map_err(wasmtime::Error::msg)?;

    let target = resolve_target_path(&caller, &path);
    let output = fs
        ::read_to_string(&target)
        .map_err(|e|
            wasmtime::Error::msg(format!("failed to read '{}': {}", target.display(), e))
        )?;
    write_lisp_string(&mut caller, &output)
}

pub fn host_write_file(
    mut caller: Caller<'_, ShellStoreData>,
    path_vec_ptr: i32,
    data_vec_ptr: i32
) -> wasmtime::Result<i32> {
    let path = read_lisp_string(&mut caller, path_vec_ptr)?;
    let data = read_lisp_string(&mut caller, data_vec_ptr)?;
    caller
        .data()
        .shell_policy.require(ShellPermission::Write, "write!", &path)
        .map_err(wasmtime::Error::msg)?;

    let target = resolve_target_path(&caller, &path);
    if let Some(parent) = target.parent() {
        if !parent.as_os_str().is_empty() {
            fs
                ::create_dir_all(parent)
                .map_err(|e| {
                    wasmtime::Error::msg(
                        format!("failed to create parent dirs '{}': {}", parent.display(), e)
                    )
                })?;
        }
    }
    fs
        ::write(&target, data.as_bytes())
        .map_err(|e| {
            wasmtime::Error::msg(format!("failed to write '{}': {}", target.display(), e))
        })?;

    Ok(0)
}

pub fn host_mkdir_p(
    mut caller: Caller<'_, ShellStoreData>,
    path_vec_ptr: i32
) -> wasmtime::Result<i32> {
    let path = read_lisp_string(&mut caller, path_vec_ptr)?;
    caller
        .data()
        .shell_policy.require(ShellPermission::Write, "mkdir!", &path)
        .map_err(wasmtime::Error::msg)?;

    let target = resolve_target_path(&caller, &path);
    fs
        ::create_dir_all(&target)
        .map_err(|e|
            wasmtime::Error::msg(format!("failed to mkdir '{}': {}", target.display(), e))
        )?;
    Ok(0)
}

pub fn host_delete(
    mut caller: Caller<'_, ShellStoreData>,
    path_vec_ptr: i32
) -> wasmtime::Result<i32> {
    let path = read_lisp_string(&mut caller, path_vec_ptr)?;
    caller
        .data()
        .shell_policy.require(ShellPermission::Delete, "delete!", &path)
        .map_err(wasmtime::Error::msg)?;

    let target = resolve_target_path(&caller, &path);
    let meta = fs
        ::symlink_metadata(&target)
        .map_err(|e|
            wasmtime::Error::msg(
                format!("failed to inspect path '{}' for delete: {}", target.display(), e)
            )
        )?;
    if meta.is_dir() {
        fs
            ::remove_dir_all(&target)
            .map_err(|e|
                wasmtime::Error::msg(
                    format!("failed to delete directory '{}': {}", target.display(), e)
                )
            )?;
    } else {
        fs
            ::remove_file(&target)
            .map_err(|e|
                wasmtime::Error::msg(format!("failed to delete file '{}': {}", target.display(), e))
            )?;
    }
    Ok(0)
}

pub fn host_move(
    mut caller: Caller<'_, ShellStoreData>,
    src_vec_ptr: i32,
    dst_vec_ptr: i32
) -> wasmtime::Result<i32> {
    let src = read_lisp_string(&mut caller, src_vec_ptr)?;
    let dst = read_lisp_string(&mut caller, dst_vec_ptr)?;
    caller
        .data()
        .shell_policy.require(ShellPermission::Write, "move!", &format!("{} -> {}", src, dst))
        .map_err(wasmtime::Error::msg)?;

    let src_path = resolve_target_path(&caller, &src);
    let dst_path = resolve_target_path(&caller, &dst);
    if let Some(parent) = dst_path.parent() {
        if !parent.as_os_str().is_empty() {
            fs
                ::create_dir_all(parent)
                .map_err(|e|
                    wasmtime::Error::msg(
                        format!("failed to create destination dirs '{}': {}", parent.display(), e)
                    )
                )?;
        }
    }
    fs
        ::rename(&src_path, &dst_path)
        .map_err(|e|
            wasmtime::Error::msg(
                format!(
                    "failed to move '{}' to '{}': {}",
                    src_path.display(),
                    dst_path.display(),
                    e
                )
            )
        )?;

    Ok(0)
}

pub fn host_curl(
    mut caller: Caller<'_, ShellStoreData>,
    args_vec_ptr: i32
) -> wasmtime::Result<i32> {
    let args_text = read_lisp_string(&mut caller, args_vec_ptr)?;
    caller
        .data()
        .shell_policy.require(ShellPermission::Network, "curl!", &args_text)
        .map_err(wasmtime::Error::msg)?;

    let mut args = split_shell_like_args(&args_text).map_err(wasmtime::Error::msg)?;
    if args.first().map(|x| x.to_ascii_lowercase()) == Some("curl".to_string()) {
        args.remove(0);
    }

    let mut command = Command::new("curl");
    command.args(&args);
    if let Some(script_cwd) = caller.data().script_cwd.as_ref() {
        command.current_dir(script_cwd);
    }
    let output = command
        .output()
        .map_err(|e| wasmtime::Error::msg(format!("failed to run curl: {}", e)))?;

    let mut combined = String::new();
    combined.push_str(&String::from_utf8_lossy(&output.stdout));
    combined.push_str(&String::from_utf8_lossy(&output.stderr));
    if combined.is_empty() && !output.status.success() {
        combined = format!("curl failed with status {}", output.status);
    }

    write_lisp_string(&mut caller, &combined)
}

pub fn add_shell_to_linker(linker: &mut Linker<ShellStoreData>) -> wasmtime::Result<()> {
    // Core wasm modules (like this backend) use WASIp1 imports.
    wasmtime_wasi::p1::add_to_linker_sync(linker, |state| &mut state.wasi_p1_ctx)?;
    linker.func_wrap("host", "list_dir", host_list_dir)?;
    linker.func_wrap("host", "read_file", host_read_file)?;
    linker.func_wrap("host", "write_file", host_write_file)?;
    linker.func_wrap("host", "mkdir_p", host_mkdir_p)?;
    linker.func_wrap("host", "delete", host_delete)?;
    linker.func_wrap("host", "move", host_move)?;
    linker.func_wrap("host", "curl", host_curl)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{ take_shell_policy_from_argv, ShellPermission, ShellPolicy };
    use std::collections::HashSet;

    #[test]
    fn parse_policy_empty_permissions() {
        let mut args = vec!["alpha".to_string(), "--allow".to_string()];
        let policy = take_shell_policy_from_argv(&mut args).unwrap();
        assert_eq!(args, vec!["alpha".to_string()]);
        assert!(policy.require(ShellPermission::Read, "read", "./x").is_err());
    }

    #[test]
    fn parse_policy_with_permissions() {
        let mut args = vec![
            "main.fez".to_string(),
            "--allow".to_string(),
            "read".to_string(),
            "write".to_string()
        ];
        let policy = take_shell_policy_from_argv(&mut args).unwrap();
        assert_eq!(args, vec!["main.fez".to_string()]);
        assert!(policy.require(ShellPermission::Read, "read", "./x").is_ok());
        assert!(policy.require(ShellPermission::Write, "mkdir", "./x").is_ok());
        assert!(policy.require(ShellPermission::Delete, "delete", "./x").is_err());
    }

    #[test]
    fn parse_policy_rejects_unknown_permission() {
        let mut args = vec!["main.fez".to_string(), "--allow".to_string(), "foo".to_string()];
        let err = take_shell_policy_from_argv(&mut args).unwrap_err();
        assert!(err.contains("unknown shell permission 'foo'"));
    }

    #[test]
    fn disabled_policy_blocks_operations() {
        let policy = ShellPolicy::disabled();
        let err = policy.require(ShellPermission::Read, "read", "./x").unwrap_err();
        assert!(err.contains("host io is disabled"));
    }

    #[test]
    fn policy_requires_specific_permission() {
        let mut perms = HashSet::new();
        perms.insert(ShellPermission::Read);
        let policy = ShellPolicy::enabled(perms);
        assert!(policy.require(ShellPermission::Read, "list-dir", ".").is_ok());
        assert!(policy.require(ShellPermission::Write, "mkdir", "./x").is_err());
        assert!(policy.require(ShellPermission::Network, "curl", "-sL").is_err());
    }
}

pub fn run_native_shell() -> Result<(), String> {
    let args: Vec<String> = env::args().collect();
    let Some(file_path) = args.get(1) else {
        return Err("missing file_path. Usage: fez-rs <script.fez> [arg ...]".to_string());
    };
    let mut argv: Vec<String> = args.iter().skip(2).cloned().collect();
    let shell_policy = crate::io
        ::take_shell_policy_from_argv(&mut argv)
        .map_err(|e| format!("invalid shell policy: {}", e))?;

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
    let store_data = ShellStoreData::new_with_security(Some(script_cwd), shell_policy).map_err(|e|
        e.to_string()
    )?;
    let decoded = crate::runtime::run_wat_text(&wat_src, store_data, &argv, |linker|
        add_shell_to_linker(linker).map_err(|e| e.to_string())
    )?;
    println!("\x1b[32m{}\x1b[0m", decoded);

    Ok(())
}
