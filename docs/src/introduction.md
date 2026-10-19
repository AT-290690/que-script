# Introduction

Calque is a small, statically typed, expression-oriented Lisp and its
WebAssembly toolchain. It parses and type-checks a program, optimizes the typed
AST, lowers it to WebAssembly, and ships the `que` CLI, formatter, language
servers, runtime integration, and editor integrations.

> **Why `que`?** Calque is the language and toolchain name. `que` is its
> compact technical spelling: the command is `que`, source files end in
> `.que`, project configuration is `que.toml`, and the language server is
> `quelsp`.

Calque is designed for a particular trade-off: write direct functional code
when it is the clearest model, and drop into explicit vectors, loops, and
mutation when a program needs predictable work and memory behavior. The
compiler type-checks the whole program before generating Wasm, and can explain
many safety and optimization facts it proves.

```que
(let sum-squares
  (lambda (xs)
    (|> xs
        (map (lambda (x) (* x x)))
        sum)))

(sum-squares [1 2 3 4 5])
; 55
```

## What Calque provides

- Hindley–Milner type inference, polymorphic functions, and explicit `sig`
  declarations where a boundary needs a known type.
- Lexical closures, recursion, partial application, macro expansion, and
  data-last functional APIs.
- Mutable locals and mutable vectors with direct loop constructs.
- Deterministic reference-counted memory for vectors, tuples, and closures;
  source-level reference cycles are rejected.
- Fixed-size `Int` arithmetic, fixed-point `Dec`, and compact closed enums.
- Static warnings for likely out-of-bounds access, empty-vector operations,
  unsafe arithmetic, and termination facts that could not be proven.
- WebAssembly output that runs through an external WASI runtime, plus an
  optional wasm2c/native-C repository path for ahead-of-time native binaries.
- The `que` command, formatter, language servers, and editor integrations.

## Status and compatibility

Calque is pre-1.0. It has a substantial regression and differential test suite,
but the language and standard-library API may still evolve. Treat the compiler
version as part of a project’s build input; pin a release for long-lived work.

The rest of this book describes behavior implemented by the repository. For a
live list of library signatures from the compiler you are actually running,
use:

```sh
que --lib types
que --lib types 'Table/*'
que --lib source Table/get
```
