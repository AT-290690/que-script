use std::process::Command;

fn main() {
    println!("cargo:rerun-if-env-changed=CALQUE_BUILD_ID");
    println!("cargo:rerun-if-changed=.git/HEAD");

    let build_id = std::env::var("CALQUE_BUILD_ID")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .or_else(git_revision)
        .unwrap_or_else(|| "source".to_string());
    println!("cargo:rustc-env=CALQUE_BUILD_ID={build_id}");
}

fn git_revision() -> Option<String> {
    let output = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }

    let revision = String::from_utf8(output.stdout).ok()?;
    let revision = revision.trim();
    (!revision.is_empty()).then(|| revision.to_string())
}
