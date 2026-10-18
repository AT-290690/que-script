//! Cross-target semantic regression testing.
//!
//! This deliberately lives outside the normal unit suite: it drives external
//! Wasmtime, WABT's wasm2c, and a native C compiler. Run it through
//! `scripts/test-differential.sh`, which supplies the checked compiler binary
//! and verifies those dependencies first.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

struct TempDir(PathBuf);

impl TempDir {
    fn new(label: &str) -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be after the epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "que-differential-{label}-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(&path).expect("differential temp directory should exist");
        Self(path)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[derive(Debug, PartialEq, Eq)]
enum Outcome {
    Value(String),
    Failure(FailureKind),
}

#[derive(Debug, PartialEq, Eq)]
enum FailureKind {
    /// Debug guards deliberately use Wasm `unreachable`, while optimized WAT
    /// and wasm2c may expose a more specific engine trap. The portable
    /// contract here is that all three reject execution, not their host's
    /// wording for the trap.
    RuntimeTrap,
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).replace("\r\n", "\n")
}

fn normalize_failure(_output: &Output) -> FailureKind {
    FailureKind::RuntimeTrap
}

fn outcome(output: Output) -> Outcome {
    if output.status.success() {
        Outcome::Value(text(&output.stdout).trim().to_string())
    } else {
        Outcome::Failure(normalize_failure(&output))
    }
}

fn executable(name: &str) -> bool {
    Command::new(name).arg("--version").output().is_ok()
}

fn compiler() -> PathBuf {
    PathBuf::from(
        std::env::var("QUE_DIFFERENTIAL_QUE")
            .expect("QUE_DIFFERENTIAL_QUE must name the checked que compiler"),
    )
}

fn run_wasm(compiler: &Path, source: &Path, mode: &str) -> Outcome {
    outcome(
        Command::new(compiler)
            .arg(source)
            .arg(mode)
            .env("QUE_WASM_RUNTIME", "wasmtime")
            .output()
            .expect("Calque should start Wasmtime"),
    )
}

fn run_native(compiler: &Path, source: &Path, output_dir: &Path) -> Outcome {
    let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/compile-native-c.sh");
    let compiled = Command::new(script)
        .arg(source)
        .arg(output_dir)
        .env("QUE_COMPILER", compiler)
        .output()
        .expect("native-C compiler script should start");
    assert!(
        compiled.status.success(),
        "native-C compilation failed for {}:\nstdout:\n{}\nstderr:\n{}",
        source.display(),
        text(&compiled.stdout),
        text(&compiled.stderr)
    );
    let executable = text(&compiled.stdout)
        .lines()
        .last()
        .expect("native-C compiler should print executable path")
        .trim()
        .to_string();
    outcome(
        Command::new(executable)
            .output()
            .expect("native-C executable should start"),
    )
}

fn next(seed: &mut u32) -> i32 {
    *seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    ((*seed >> 8) % 97 + 1) as i32
}

fn generated_sources() -> Vec<(String, String)> {
    let mut seed = 0x51a7_1c5eu32;
    let mut definitions = Vec::new();
    let mut calls = Vec::new();
    for case in 0..12 {
        let a = next(&mut seed);
        let b = next(&mut seed);
        let c = next(&mut seed);
        let d = next(&mut seed);
        let factor = next(&mut seed) % 11 + 1;
        let depth = next(&mut seed) % 10 + 1;
        definitions.push(format!(
            "(let calc-{case} (lambda ()\n\
              (let f (lambda x (if (= (% x 2) 0) (+ (* x {factor}) {a}) (- (* x {factor}) {b}))))\n\
              (let xs [{a} {b} {c} {d}])\n\
              (mut i 0)\n\
              (mut total 0)\n\
              (while (< i (length xs))\n\
                (let x (get xs i))\n\
                (if (= (% (+ x i) 2) 0) (alter! total (+ total x)) (alter! total (- total x)))\n\
                (++ i))\n\
              (let make-adder (lambda n (lambda x (+ n x))))\n\
              (let add (make-adder {a}))\n\
              (let pair {{{a} {b}}})\n\
              (let {{x y}} pair)\n\
              (letrec sum-to (lambda n (if (= n 0) 0 (+ n (sum-to (- n 1))))))\n\
              (let base (+ (f {c}) total))\n\
              (let closure-value (add {d}))\n\
              (let pair-value (+ (* x {factor}) y))\n\
              (let answer (+ base (+ closure-value (+ pair-value (sum-to {depth})))))\n\
              answer))"
        ));
        calls.push(format!("(calc-{case})"));
    }
    vec![(
        "generated-mixed-corpus".to_string(),
        format!("{}\n[{}]", definitions.join("\n"), calls.join(" ")),
    )]
}

fn assert_same(name: &str, source: &str, debug: &Outcome, optimized: &Outcome, native: &Outcome) {
    assert_eq!(
        optimized, debug,
        "optimized Wasm diverged from debug Wasm for generated case '{name}':\n{source}\ndebug: {debug:?}\nopt: {optimized:?}"
    );
    assert_eq!(
        native, debug,
        "wasm2c/native-C diverged from debug Wasm for generated case '{name}':\n{source}\ndebug: {debug:?}\nnative: {native:?}"
    );
}

#[test]
#[ignore = "requires wasmtime, wasm2c, and a C compiler; run scripts/test-differential.sh"]
fn generated_programs_match_across_debug_opt_and_native_c() {
    assert!(
        executable("wasmtime") && executable("wasm2c") && (executable("clang") || executable("cc")),
        "requires wasmtime, wasm2c, and clang or cc"
    );
    let compiler = compiler();
    assert!(compiler.is_file(), "missing Calque compiler: {}", compiler.display());
    let temp = TempDir::new("targets");

    for (name, source) in generated_sources() {
        let source_path = temp.0.join(format!("{name}.que"));
        fs::write(&source_path, &source).expect("generated source should write");
        let debug = run_wasm(&compiler, &source_path, "--debug");
        let optimized = run_wasm(&compiler, &source_path, "--opt");
        let native = run_native(&compiler, &source_path, &temp.0.join(format!("native-{name}")));
        assert_same(&name, &source, &debug, &optimized, &native);
    }

    let source = "(/ 7 0)";
    let source_path = temp.0.join("division-by-zero.que");
    fs::write(&source_path, source).expect("failure source should write");
    let debug = run_wasm(&compiler, &source_path, "--debug");
    let optimized = run_wasm(&compiler, &source_path, "--opt");
    let native = run_native(&compiler, &source_path, &temp.0.join("native-division-by-zero"));
    assert_same("division-by-zero", source, &debug, &optimized, &native);
    assert_eq!(debug, Outcome::Failure(FailureKind::RuntimeTrap));
}
