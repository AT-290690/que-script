# FFI

Que FFI is currently **Wasm host-import FFI**.

It is not a C ABI.
It is not callback-oriented.
It is not a generic plugin loader yet.

The model is:

1. declare an import in Que with `extern`
2. compile it to a Wasm import
3. register a matching host function in Rust

For a custom host scaffold, run:

```sh
que init-host my-host
```

## Que syntax

Example:

```lisp
(extern env add_one add-one! (Int -> Int))
(extern host read_file read! ([Char] -> [Char]))
```

Rules:

- `extern` is top-level only
- all externs are treated as effectful
- extern local names must end with `!`

This is valid:

```lisp
(extern env clock_now_ms clock-now-ms! (() -> Int))
```

This is rejected:

```lisp
(extern env add_one add-one (Int -> Int))
```

because `add-one` does not end with `!`.

## Type syntax

Current extern type syntax uses the same surface type shapes used by Que hints:

```lisp
Int
Dec
Bool
Char
[Char]
[Int]
{Int Int}
(Int -> Int)
([Char] -> [Char])
([Char] [Char] -> ())
(() -> Int)
```

Important:

- `Dec` is currently fixed-point `i32`, not `f64`
- zero-arg functions use `(() -> T)`
- unit return is `()`

## Wasm ABI

Current lowering:

- `Int` -> `i32`
- `Dec` -> `i32`
- `Bool` -> `i32`
- `Char` -> `i32`
- `()` -> `i32` at Wasm ABI level through the existing backend conventions
- `[T]` / tuples / strings -> `i32` pointer into Que linear memory

Strings are `[Char]`.

Managed values returned from host must be valid Que heap objects.
Managed arguments passed into host are borrowed by default.

## What exists today

Built-in host IO already uses this extern path:

- `read!`
- `write!`
- `list-dir!`
- `mkdir!`
- `delete!`
- `move!`
- `print!`
- `sleep!`
- `clear!`

The declarations live in:

- [src/externals.rs](src/externals.rs)

The native CLI host linker registrations live in:

- [src/io.rs](src/io.rs)

## Adding a new builtin host function

If you want a new function to work in the native `que` CLI out of the box, do this:

### 1. Add the Rust host function

In:

- [src/io.rs](src/io.rs)

Example shape:

```rust
pub fn host_time_now_ms(_caller: Caller<'_, ShellStoreData>) -> wasmtime::Result<i32> {
    Ok(0)
}
```

If the function needs strings or vectors, use the existing helpers:

- `read_lisp_string`
- `write_lisp_string`

### 2. Add the extern spec

In:

- [src/externals.rs](src/externals.rs)

Add a `BuiltinHostExternSpec` entry:

```rust
BuiltinHostExternSpec {
    module: "host",
    import: "time_now_ms",
    local_name: "time-now-ms!",
    typ: || fn1(Type::Unit, Type::Int),
},
```

### 3. Register it in the native linker

In:

- [src/io.rs](src/io.rs)

Extend `register_builtin_host_import(...)` with a new arm:

```rust
"time_now_ms" => {
    linker.func_wrap(spec.module, spec.import, host_time_now_ms)?;
}
```

That is enough for:

- builtin env type support
- extern declaration injection
- effect classification
- native runtime linking

Builtin host specs use real Rust-side `Type` values now, not string type snippets.

## Using a user-declared extern

If the host already provides an import, user code can declare it directly:

```lisp
(extern env add_one add-one! (Int -> Int))
(add-one! 41)
```

This compiles to a Wasm import and direct call.

In that case the host must provide:

```rust
linker.func_wrap("env", "add_one", |x: i32| -> i32 {
    x + 1
})?;
```

## Effect model

All externs are effectful.

Compiler consequences:

- extern calls are not treated as pure
- extern names must end with `!`

There is no “pure extern” mode right now.

## Current limitations

Not supported yet:

- callbacks from host into Que functions
- generic host registration through CLI config
- C ABI
- richer automatic marshalling for arbitrary Que data

The supported, stable path is host imports plus the existing Que heap/string ABI.
