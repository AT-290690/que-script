# CLI

Run `que --help` for the installed command overview. The important commands
are below.

## Run and compile

```sh
que program.que --debug
que program.que --opt
que run-wasi program.que --runtime wasmtime --allow read print
que compile program.que --opt --out program.wasm
que wat program.que
```

`run-wasi` is the explicit spelling of normal execution; `que program.que`
uses it implicitly. `--runtime` accepts `wasmtime`, `wasmer`, `iwasm`/`wamr`,
or a path to one of those executables.

## Inspect and explain

```sh
que program.que --emit types
que program.que --emit source
que program.que --opt --emit opt-source
que explain program.que --opt
que explain program.que --opt --json --out report.json
```

## Format and browse the library

```sh
que fmt program.que
que fmt program.que --check
que fmt --stdin

que --lib names 'Table/*'
que --lib types 'map*'
que --lib source Table/get
```

Library patterns are passed as an argument—names containing `!`, `?`, or `*`
do not need shell escaping when quoted.

## Built-in guides and environment

```sh
que --learn
que --examples
que --style
que --pitfalls
que --env
```

`--env` lists compiler/runtime environment settings, including optimization,
debug, decimal-scale, and runtime selection settings.
