# Getting started

## Install the compiler and a runtime

Calque emits WASI WebAssembly. The `que` command needs a compatible runtime on
`PATH` to execute a program: `wasmtime` is the default; `wasmer` and
`iwasm`/WAMR are also supported.

Install the CLI and bundled library on Linux or macOS:

```sh
curl -fsSL https://raw.githubusercontent.com/AT-290690/que-script/refs/heads/main/scripts/install.sh | bash
```

Then install your chosen WASI runtime. Select a non-default runtime per run:

```sh
que hello.que --runtime wasmer
que hello.que --runtime /path/to/iwasm
```

## First program

Create `hello.que`:

```que
(let greeting "Hello, Calque!")
(println! greeting)
```

Run while developing:

```sh
que hello.que --debug --allow print
```

Use `--opt` for release-style execution after checking the program’s
invariants:

```sh
que hello.que --opt --allow print
```

`--debug` keeps runtime guards for checked integer arithmetic, division by
zero, and vector access where appropriate, and reports static-analysis
findings. `--opt` enables optimization and removes those optional runtime
guards; it is not a substitute for establishing correctness.

## Learn from the installed compiler

```sh
que --help
que --learn
que --examples
que --style
que --pitfalls
```

Evaluate a small source expression without making a file:

```sh
que --eval '(+ 20 22)' --debug
```

## A project

A project uses `que.toml` to supply optional source dependencies and compiler
environment settings:

```toml
entry = "main.que"
deps = ["./lib/math.que"]

[env]
QUE_TCO = "aggressive"
```

Run the entry file explicitly from the project folder:

```sh
que main.que --debug
```

Calque discovers the nearest `que.toml`, loads `deps` as definition bundles, and
applies its `[env]` settings. See [Projects, tests, and
editors](compiler/projects-and-editors.md) for the exact rules.
