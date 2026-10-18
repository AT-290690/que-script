# FFI and host imports

Calque’s current FFI is a **WebAssembly host-import FFI**. A Calque `extern`
declaration becomes a Wasm import; an embedding host must supply a function
with the same module and import names. It is not a C ABI, plugin system, or
callback framework.

## Declare an import

```que
(extern env add_one add-one! (Int -> Int))

(add-one! 41)
; 42 when the host supplies env.add_one
```

The declaration syntax is:

```que
(extern module import local-name! Type)
```

Rules:

- `extern` is top-level only.
- A local external name must end in `!`.
- External calls are treated as effectful by the compiler.
- The declaration has a concrete type; it is not dynamically typed.

Zero-argument imports use unit as their argument:

```que
(extern env clock_now_ms clock-now-ms! (() -> Int))
```

## Supported declared shapes

Extern signatures use the familiar core type syntax:

```que
Int
Dec
Bool
Char
()
[Char]
[Int]
{Int Int}
(Int -> Int)
([Char] [Char] -> ())
```

At the Wasm ABI boundary, scalar core values lower to `i32`. Vectors, tuples,
and strings lower to an `i32` pointer into Calque’s linear memory. `Dec` is a
fixed-point `i32`, not an `f64`. Calque’s managed representation is internal to
the runtime: a host returning a managed value must construct a valid Calque heap
object, while managed values received from Calque are borrowed unless the host
explicitly retains them using the runtime convention.

## Built-in host operations

The standard runtime integration supplies typed host imports for file, stdin,
terminal, clock, serialization, and randomness operations, including:

```text
read!             read/chunks!       read/lines!        list-dir!
stdin!            stdin/chunks!      write!             mkdir!
move!             delete!            print!             println!
clear!            sleep!             time!              random!
serialize         deserialize
```

They are capability-gated under the external WASI runner; see [Effects and
permissions](../language/effects.md).

## Custom hosts

The normal `que` runner delegates to an external WASI runtime. It can run the
built-in operations through Calque’s WASI adapter, but it does not load arbitrary
user host modules from command-line configuration. A program containing a
custom `extern` therefore needs an embedding/application that instantiates the
generated Wasm module and supplies the declared import.

For example, a Rust Wasmtime host conceptually registers:

```rust
linker.func_wrap("env", "add_one", |x: i32| -> i32 { x + 1 })?;
```

before instantiating the compiled module. The exact initialization needed for
Calque-managed values belongs to that host integration, not the source program.

## Current limits

The current supported model deliberately excludes:

- callbacks from host code into Calque closures
- an async/await runtime or automatic future bridging
- a stable public C ABI for Calque objects
- a generic CLI plugin loader for arbitrary imports
- automatic marshalling of arbitrary managed data

The optional wasm2c/native-C runner still uses this host-import model: wasm2c
translates the guest code, while the adapter supplies the imports. It does not
turn Calque’s internal memory representation into a general C interface.
