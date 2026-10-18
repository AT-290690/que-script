# Projects, tests, and editors

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

## Tests

Calque code is ordinary source, so a small test file can call project definitions
and end by returning or printing a useful value. The bundled `test` macro is
also available for assertion-shaped values. Run test sources with the same
debug or optimized workflow you use for programs:

```sh
que tests/math.test.que --debug
```

Compiler regression tests live in the Rust workspace; run them from a checkout
with `cargo test`.

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
