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

## Built-in effect operations

The normal `que` runner supplies these operations through its WASI adapter.
They have ordinary, statically checked Calque types; `!` marks the operations
that perform an external effect. `println!` is the standard-library convenience
form of `print!` plus a newline, but has the same source-level type.

### Files and directories

| Operation | Type | Permission | Meaning |
|---|---|---|---|
| `read!` | `[Char] -> [Char]` | `read` | Read a complete text file. |
| `list-dir!` | `[Char] -> [[Char]]` | `read` | List directory-entry names. |
| `read/chunks!` | `[Char] -> Int -> ([Char] -> Bool) -> Bool` | `read` | Visit successive text chunks. |
| `read/lines!` | `[Char] -> ([Char] -> Bool) -> Bool` | `read` | Visit successive lines, without newline terminators. |
| `write!` | `[Char] -> [Char] -> ()` | `write` | Write text to a path. |
| `mkdir!` | `[Char] -> ()` | `write` | Create a directory path. |
| `move!` | `[Char] -> [Char] -> ()` | `write` | Rename or move a path. |
| `delete!` | `[Char] -> ()` | `delete` | Delete a file or directory path. |

Paths and text are both `[Char]`, normally written as string literals:

```que
(let config (read! "config.txt"))
(write! "copy.txt" config)
(let names (list-dir! "."))
```

### Standard input and callbacks

| Operation | Type | Permission | Meaning |
|---|---|---|---|
| `stdin!` | `() -> [Char]` | `stdin` | Read all standard input as text. |
| `stdin/chunks!` | `Int -> ([Char] -> Bool) -> Bool` | `stdin` | Visit standard input in chunks. |

All three callback operations—`read/chunks!`, `read/lines!`, and
`stdin/chunks!`—share one control convention:

- The callback receives a `[Char]` chunk or line.
- Return `false` to continue.
- Return `true` to stop early.
- The operation itself returns `true` when it stopped early and `false` when it
  reached end of input.

Chunk size must be positive. For example, print at most ten lines:

```que
(&mut seen 0)
(let stopped?
  (read/lines! "input.txt"
    (lambda (line)
      (&alter! seen (+ (&get seen) 1))
      (println! line)
      (>= (&get seen) 10))))
```

The callback API avoids constructing a vector of all lines or chunks. The
current external WASI adapter reads its input text before it partitions it for
callbacks; an embedding or native-C host may stream at the host level. Do not
assume the callback spelling alone guarantees bounded host-side input memory on
every target.

### Terminal and clock

| Operation | Type | Permission | Meaning |
|---|---|---|---|
| `print!` | `[Char] -> ()` | `print` | Print text without adding a newline. |
| `println!` | `[Char] -> ()` | `print` | Print text followed by a newline. |
| `clear!` | `() -> ()` | `print` | Request a terminal clear. |
| `sleep!` | `Int -> ()` | `clock` | Sleep for a non-negative number of milliseconds. |
| `time!` | `() -> Int` | `clock` | Current wall-clock time in whole Unix seconds. |
| `random!` | `() -> Int` | `clock` | Get one host-provided random `Int`. |

`println!` accepts text, not an arbitrary value. Convert an integer explicitly:

```que
(println! (Integer->String (time!)))
(sleep! 250)
```

`random!` is effectful host randomness. The library's `random`, `random/int`,
`random.`, and `random/dec` are separate deterministic seed-to-next-state
functions for reproducible computations.

### Serialization

| Operation | Type | Permission | Meaning |
|---|---|---|---|
| `serialize` | `T -> [Char]` | none | Convert a statically known supported Calque value to text. |
| `deserialize` | `[Char] -> T` | none | Decode text as the statically expected type. |

`deserialize` cannot infer a target type from characters alone. State the
expected result with `sig` before the binding:

```que
(let saved (serialize {42 "Ada"}))

(sig restored {Int [Char]})
(let restored (deserialize saved))
; {42 "Ada"}
```

Enums serialize by their declared variant name and are checked against the
expected enum on decode. See [Enum-keyed table round trip](../examples/enum-table-serialization.md)
for a table example.

### Compatibility and persistence

`serialize` is designed for Calque-to-Calque data within a compatible compiler
release. It is **not yet a versioned, stable public storage or network format**.
Do not make a database schema, long-lived cache, or external API depend on its
exact textual spelling without pinning the Calque compiler version and testing
the decoder during upgrades.

For persistent application data today, store an explicit version field next to
the serialized value and own the migration in source:

```que
(let saved {1 (serialize value)})
; On read, inspect the version before choosing the expected decoder and migration.
```

Serialization needs a concrete, fully known value type. It cannot serialize an
unresolved generic or a function/closure, and `deserialize` needs an expected
type through its binding, `sig`, or surrounding context. Malformed input or a
type mismatch is a decoding failure, not an implicit conversion.

Run a program using an operation only after granting the matching capability:

```sh
que program.que --allow read write print
```

`--allow all` grants every group. See [Effects and permissions](../language/effects.md)
for the short permission overview.

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
