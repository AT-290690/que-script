# Projects, composition, tests, and editors

## `que.toml`

When compiling a `.que` file, Calque searches upward from that file for a
`que.toml`. The configuration can provide source bundles and environment
settings:

```toml
entry = "main.que"
deps = [
  "./lib/math.que",
  "./lib/text.que",
]

[env]
QUE_WASM_OPT = "speed"
QUE_DEVIRTUALIZE = "aggressive"
QUE_TCO = "aggressive"
```

`deps` paths are resolved from the project root. Each dependency must be a
`.que` definition bundle: its top-level forms may be `sig`, `let`, `letrec`,
`mut`, or `extern`, but not arbitrary run-time expressions. Calque merges those
definitions with the standard library and the compiled source file.

At present, invoke the source entry explicitly:

```sh
que main.que --debug
```

The `entry` field records the intended entrypoint for tooling and project
conventions; compilation is driven by the source file passed to the current
CLI command.

## Composition, not modules

Today Calque projects use **ordered source composition**, not an import/module
system or package registry. A dependency bundle contributes its top-level
names to the program that is compiled. This has useful consequences:

- dependencies are ordinary `.que` source, versioned with the project;
- definitions must appear before uses, so dependency order in `deps` matters;
- a dependency cannot perform top-level I/O or other run-time work;
- duplicate top-level names are an error rather than an override mechanism;
- recursive calls use `letrec` inside one definition; mutually recursive
  top-level bundles are not currently a project feature.

A small project can therefore use a simple layout:

```text
report/
  que.toml
  lib/parse.que
  lib/stats.que
  main.que
  tests/report.test.que
```

Keep reusable `let`/`letrec` definitions in `lib/*.que`, list them in
dependency order in `que.toml`, and keep execution and host I/O in `main.que`.
This is deliberately explicit until Calque grows a real module/package model.

## Tests and CI

Calque test files are ordinary source. A useful small test file returns data
that a shell runner, CI job, or person can inspect:

```que
; tests/math.test.que
(let square (lambda (n) (* n n)))

[(test "square of zero" (= (square 0) 0))
 (test "square of nine" (= (square 9) 81))]
; [{"square of zero" true} {"square of nine" true}]
```

`test` is intentionally a tiny macro that creates `{name outcome}` data. It
does not provide a hidden test runner or magical assertion behavior. Run test
sources in debug mode first:

```sh
que tests/math.test.que --debug
```

For a project, make CI run formatting, debug tests, optimized tests where
relevant, and the compiler’s own Rust suite from a checkout:

```sh
que fmt main.que --check
que tests/math.test.que --debug
que tests/math.test.que --opt
cargo test
```

Use a real assertion runner around the printed result if a nonzero process exit
on `false` is required today. A debug/optimized or WASI/native output mismatch
for the same valid test input is a compiler bug; preserve the smallest
reproducer and the commands used. See the [debugging cookbook](debugging.md).

## Formatter

```sh
que fmt program.que
que fmt program.que --check
que fmt --stdin
```

The formatter keeps compact short forms where possible, uses two-space-style
nesting, preserves comments, and deliberately keeps runs of closing delimiters
together rather than spreading them across lines.

## Language server and editors

`quelsp` provides diagnostics, inferred-type hover, completion, signatures,
and project-aware definitions. The repository includes:

- a thin Neovim plugin in `miscs/neovim`
- a VS Code extension in `miscs/extension/que-lang`

The Neovim integration provides `:QueFormat` / `<Space>f` and `:QueLib` /
`<Space>g`. In `que nvim`, scratch commands include optimized run, debug run,
WAT, inferred types, `explain`, expanded source, and library browsing.

See the integration READMEs for installation details and configuration
examples.
