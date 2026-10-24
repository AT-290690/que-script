# Getting started

Install Calque, a WASI runtime, the language server, and optional editor
integration by following [Installation](installation.md). Once `que` is on
your `PATH`, continue here.

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
See [Debug, optimized, and native execution](compiler/release-safety.md) for
the precise safety and deployment contract.

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
