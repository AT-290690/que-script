use std::env;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};

fn runtime_path() -> Result<PathBuf, String> {
    if let Some(path) = env::var_os("QUE_RUNTIME") {
        return Ok(PathBuf::from(path));
    }
    let executable = env::current_exe()
        .map_err(|error| format!("failed to locate the que executable: {error}"))?;
    let directory = executable
        .parent()
        .ok_or_else(|| "failed to locate the que executable directory".to_string())?;
    let sibling = directory.join(if cfg!(windows) {
        "que-runtime.exe"
    } else {
        "que-runtime"
    });
    if sibling.is_file() {
        return Ok(sibling);
    }
    let development = Path::new(env!("CARGO_MANIFEST_DIR")).join(if cfg!(windows) {
        "target/debug/queio.exe"
    } else {
        "target/debug/queio"
    });
    if development.is_file() {
        return Ok(development);
    }
    Err(
        "Que runtime not found. Reinstall Que or set QUE_RUNTIME to the external runtime path."
            .to_string(),
    )
}

fn exit_code(status: ExitStatus) -> i32 {
    status.code().unwrap_or(1)
}

fn main() {
    if matches!(env::args().nth(1).as_deref(), Some("--version" | "-V")) {
        println!("que {}", env!("CARGO_PKG_VERSION"));
        return;
    }
    let runtime = runtime_path().unwrap_or_else(|error| {
        eprintln!("\x1b[31mException: {error}\x1b[0m");
        std::process::exit(1);
    });
    let status = Command::new(runtime)
        .args(env::args_os().skip(1))
        .env("QUE_FRONTEND_NAME", "que")
        .status()
        .unwrap_or_else(|error| {
            eprintln!("\x1b[31mException: failed to start the Que runtime: {error}\x1b[0m");
            std::process::exit(1);
        });
    std::process::exit(exit_code(status));
}
