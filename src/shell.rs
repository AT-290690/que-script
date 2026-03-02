use std::process::Command;
use std::collections::HashSet;
use std::path::PathBuf;
use wasmtime::{Caller, Extern, Linker, Memory, TypedFunc};
use wasmtime_wasi::{ResourceTable, WasiCtx, WasiCtxBuilder, WasiCtxView, WasiView};

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

    pub fn validate_command(&self, command: &str) -> Result<(), String> {
        if !self.shell_enabled {
            return Err(format!(
                "shell is disabled. pass --allow <read|write|delete|network> [...]. denied command: {}",
                command
            ));
        }

        if command.trim().is_empty() {
            return Err(format!("shell command is empty. denied command: {}", command));
        }

        if command.contains("$(") || command.contains('`') || command.contains("<(") || command.contains(">(") {
            return Err(format!(
                "shell command substitutions are blocked by policy. denied command: {}",
                command
            ));
        }

        let entries = parse_command_entries(command).map_err(|reason| {
            format!("unable to parse shell command ({}). denied command: {}", reason, command)
        })?;

        if entries.is_empty() {
            return Err(format!("no executable command found. denied command: {}", command));
        }

        for entry in entries {
            if entry.has_output_redirection && !self.allows(ShellPermission::Write) {
                return Err(format!(
                    "shell permission 'write' is required for output redirection in '{}'. denied command: {}",
                    entry.program, command
                ));
            }

            let needed = classify_program(&entry.program).ok_or_else(|| {
                format!(
                    "command '{}' is not in the shell allowlist. denied command: {}",
                    entry.program, command
                )
            })?;

            if !self.allows(needed) {
                return Err(format!(
                    "shell permission '{}' is required for '{}'. denied command: {}",
                    needed.as_str(),
                    entry.program,
                    command
                ));
            }
        }

        Ok(())
    }
}

#[derive(Debug)]
struct CommandEntry {
    program: String,
    has_output_redirection: bool,
}

const READ_COMMANDS: &[&str] = &[
    "[",
    "awk",
    "basename",
    "cat",
    "cut",
    "date",
    "df",
    "dirname",
    "du",
    "echo",
    "env",
    "fez-rs",
    "file",
    "find",
    "free",
    "grep",
    "head",
    "hostname",
    "id",
    "less",
    "ls",
    "lsof",
    "more",
    "nproc",
    "pgrep",
    "printenv",
    "ps",
    "pwd",
    "que",
    "readlink",
    "realpath",
    "sed",
    "sleep",
    "sort",
    "stat",
    "tail",
    "test",
    "top",
    "tr",
    "uname",
    "uniq",
    "wc",
    "which",
    "whoami",
];

const WRITE_COMMANDS: &[&str] = &[
    "chmod",
    "chgrp",
    "chown",
    "cp",
    "dd",
    "install",
    "ln",
    "mkdir",
    "mv",
    "tee",
    "touch",
    "truncate",
];

const DELETE_COMMANDS: &[&str] = &["rm", "rmdir", "shred", "unlink"];

const NETWORK_COMMANDS: &[&str] = &[
    "curl",
    "dig",
    "ftp",
    "host",
    "nc",
    "ncat",
    "netcat",
    "nslookup",
    "ping",
    "scp",
    "sftp",
    "ssh",
    "telnet",
    "wget",
];

fn classify_program(program: &str) -> Option<ShellPermission> {
    if READ_COMMANDS.contains(&program) {
        return Some(ShellPermission::Read);
    }
    if WRITE_COMMANDS.contains(&program) {
        return Some(ShellPermission::Write);
    }
    if DELETE_COMMANDS.contains(&program) {
        return Some(ShellPermission::Delete);
    }
    if NETWORK_COMMANDS.contains(&program) {
        return Some(ShellPermission::Network);
    }
    None
}

fn is_env_assignment(token: &str) -> bool {
    let Some(eq_idx) = token.find('=') else {
        return false;
    };
    if eq_idx == 0 {
        return false;
    }
    let key = &token[..eq_idx];
    let mut chars = key.chars();
    match chars.next() {
        Some(c) if c == '_' || c.is_ascii_alphabetic() => {}
        _ => return false,
    }
    chars.all(|c| c == '_' || c.is_ascii_alphanumeric())
}

fn normalize_program(raw: &str) -> String {
    raw.rsplit('/').next().unwrap_or(raw).to_ascii_lowercase()
}

fn extract_program(tokens: &[String]) -> Option<String> {
    let mut idx = 0usize;
    while idx < tokens.len() && is_env_assignment(&tokens[idx]) {
        idx += 1;
    }

    while idx < tokens.len() && tokens[idx] == "!" {
        idx += 1;
    }

    tokens.get(idx).map(|t| normalize_program(t))
}

fn parse_command_entries(command: &str) -> Result<Vec<CommandEntry>, String> {
    let mut entries = Vec::new();
    let mut tokens = Vec::<String>::new();
    let mut token = String::new();
    let mut in_single = false;
    let mut in_double = false;
    let mut escaped = false;
    let mut has_output_redirection = false;

    let push_token = |token: &mut String, tokens: &mut Vec<String>| {
        if !token.is_empty() {
            tokens.push(std::mem::take(token));
        }
    };

    let flush_entry = |tokens: &mut Vec<String>,
                       has_output_redirection: &mut bool,
                       entries: &mut Vec<CommandEntry>| {
        if let Some(program) = extract_program(tokens) {
            entries.push(CommandEntry {
                program,
                has_output_redirection: *has_output_redirection,
            });
        }
        tokens.clear();
        *has_output_redirection = false;
    };

    for ch in command.chars() {
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
            '\\' => escaped = true,
            '\'' => in_single = true,
            '"' => in_double = true,
            '>' => {
                has_output_redirection = true;
                push_token(&mut token, &mut tokens);
            }
            ';' | '\n' | '|' | '&' => {
                push_token(&mut token, &mut tokens);
                flush_entry(&mut tokens, &mut has_output_redirection, &mut entries);
            }
            c if c.is_whitespace() => {
                push_token(&mut token, &mut tokens);
            }
            _ => token.push(ch),
        }
    }

    if in_single || in_double {
        return Err("unterminated string quote".to_string());
    }

    if escaped {
        token.push('\\');
    }

    push_token(&mut token, &mut tokens);
    flush_entry(&mut tokens, &mut has_output_redirection, &mut entries);

    Ok(entries)
}

fn parse_shell_policy_permissions(parts: &[String]) -> Result<ShellPolicy, String> {
    let mut permissions = HashSet::new();
    let mut grant_all = false;

    for part in parts {
        for fragment in part.split(',') {
            let token = fragment
                .trim()
                .trim_matches('"')
                .trim_matches('\'')
                .to_ascii_lowercase();
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
                "all" | "*" => grant_all = true,
                _ => {
                    return Err(format!(
                        "unknown shell permission '{}'. expected one of: read, write, delete, network",
                        token
                    ));
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

pub fn shell_policy_from_process_args() -> Result<ShellPolicy, String> {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    take_shell_policy_from_argv(&mut args)
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
    caller
        .data()
        .shell_policy
        .validate_command(&cmd_str)
        .map_err(wasmtime::Error::msg)?;

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

#[cfg(test)]
mod tests {
    use super::{ take_shell_policy_from_argv, ShellPermission, ShellPolicy };
    use std::collections::HashSet;

    #[test]
    fn parse_policy_empty_permissions() {
        let mut args = vec!["alpha".to_string(), "--allow".to_string()];
        let policy = take_shell_policy_from_argv(&mut args).unwrap();
        assert_eq!(args, vec!["alpha".to_string()]);
        assert!(policy.validate_command("ls").is_err());
    }

    #[test]
    fn parse_policy_with_permissions() {
        let mut args = vec![
            "main.fez".to_string(),
            "--allow".to_string(),
            "read".to_string(),
            "write".to_string(),
        ];
        let policy = take_shell_policy_from_argv(&mut args).unwrap();
        assert_eq!(args, vec!["main.fez".to_string()]);
        assert!(policy.validate_command("ls").is_ok());
        assert!(policy.validate_command("mkdir demo").is_ok());
        assert!(policy.validate_command("rm demo").is_err());
    }

    #[test]
    fn parse_policy_rejects_unknown_permission() {
        let mut args = vec![
            "main.fez".to_string(),
            "--allow".to_string(),
            "foo".to_string(),
        ];
        let err = take_shell_policy_from_argv(&mut args).unwrap_err();
        assert!(err.contains("unknown shell permission 'foo'"));
    }

    #[test]
    fn disabled_policy_blocks_shell() {
        let policy = ShellPolicy::disabled();
        let err = policy.validate_command("ls").unwrap_err();
        assert!(err.contains("denied command: ls"));
    }

    #[test]
    fn policy_requires_specific_permission() {
        let mut perms = HashSet::new();
        perms.insert(ShellPermission::Read);
        let policy = ShellPolicy::enabled(perms);
        assert!(policy.validate_command("ls -la").is_ok());
        assert!(policy.validate_command("mkdir a").is_err());
        assert!(policy.validate_command("curl https://example.com").is_err());
    }

    #[test]
    fn write_redirection_needs_write_permission() {
        let mut perms = HashSet::new();
        perms.insert(ShellPermission::Read);
        let policy = ShellPolicy::enabled(perms);
        assert!(policy.validate_command("echo hi > out.txt").is_err());
    }
}
