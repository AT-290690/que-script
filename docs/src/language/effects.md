# Effects and permissions

Pure computation is available without runtime permissions. Host operations are
explicit functions and must be granted capabilities when running under the
external WASI host.

```que
(let input (read! "input.txt"))
(println! input)
```

```sh
que program.que --allow read print
```

## What Calque tracks

Effects are compiler metadata inferred from the implementation of an
expression. They are useful for diagnostics, permission reporting, ownership
reasoning, and optimization. They are **not** part of a function's written
type: `Int -> Int` does not say whether a function mutates or performs I/O.

`que explain` and editor hovers can report one or more of these labels:

| Label | Meaning |
|---|---|
| `pure` | No known mutation, host I/O, or unresolved call. |
| `mutate` | Performs a mutation which the compiler can identify. |
| `io` | Calls a host operation or `extern`; it may require a permission. |
| `unknown-call` | Calls a function whose implementation is not statically known at this call site. |

The labels combine. For example, a function may be both `mutate` and `io`.
`pure` is used only when none of the other labels apply.

### `unknown-call` is conservative, not an error

Higher-order code often accepts a callback. The callback may be pure, mutate
its argument, or call the host, but that information is not contained in its
ordinary function type:

```que
(let apply-twice
  (lambda (f x)
    (f (f x))))
```

`apply-twice` is therefore reported with `unknown-call`: the parameter `f`
could refer to any compatible function. This does **not** mean the program is
invalid, that it necessarily performs I/O, or that permission was denied. It
means the compiler cannot make a stronger claim from the source it sees.

By contrast, a direct call to a locally known implementation can be
summarised precisely:

```que
(let plus-one (lambda x (+ x 1)))
(let answer (plus-one 41))
; `plus-one` and this call are pure.
```

The analyser also resolves many direct `letrec` calls. A function passed
through a parameter, stored behind an unknown value, or invoked indirectly may
remain `unknown-call` even when you know it is harmless at runtime.

This conservatism protects correctness and ownership: around an unknown call,
the optimizer does not assume that a mutable argument is unchanged, and avoids
transformations that would reorder, duplicate, or discard the call. It can
also prevent inlining, loop unrolling, and other optimizations that require a
known effect. Make the callee direct, or keep the callback boundary explicit,
when the distinction matters for performance.

Use `que explain` to inspect the inferred result for a program:

```sh
que explain program.que --opt
```

The **Effects and permissions** section gives the combined program effect;
the **Generated code** section lists effects for individual compiled
functions. In the editor, hover over a binding to see its inferred effect.

### Effects, names, and permissions

`!` is a useful naming convention for operations which mutate or interact with
the outside world, but it is not a permission grant and does not by itself
determine an effect. An `extern` declaration is always treated as host I/O,
and the external runner enforces its matching `--allow` capability. Local
functions infer their effects from their bodies.

Effects are informative rather than an effect-row type system: there is no
syntax for declaring “this function is pure,” and `sig` checks value types,
not effects. Runtime permissions remain the authority for built-in host
operations.

## Permission groups

| Permission | Operations |
|---|---|
| `read` | `read!`, `list-dir!`, `read/chunks!`, `read/lines!` |
| `stdin` | `stdin!`, `stdin/chunks!` |
| `write` | `write!`, `mkdir!`, `move!` |
| `delete` | `delete!` |
| `print` | `print!`, `println!`, `clear!` |
| `clock` | `sleep!`, `time!`, `random!` |
| `all` | all groups |

`--allow` accepts the groups separated by spaces on the command line. The
runtime exposes the current working directory to the Wasm program when a file
permission is granted.

## Callback input

The callback forms process chunks or lines without building an output vector:

```que
(read/lines! "input.txt"
  (lambda (line)
    (println! line)
    false))
```

The callback returns a `Bool`: `false` continues and `true` stops early. The
operation returns whether it stopped early. `read/chunks!` and
`stdin/chunks!` require a positive chunk size.

The current external WASI adapter reads the source input before splitting it
for these callbacks, while a native-C or embedding host can stream directly.
The callback API is still useful for early exit and for avoiding a separate
vector of every line or chunk. Query the installed library when using these
APIs:

```sh
que --lib types 'read/*'
que --lib types 'stdin/*'
```

## Serialization and FFI

`serialize` converts a statically known Calque value to text. `deserialize`
requires a concrete expected type, usually supplied by `sig`. Enums serialize
by declared variant name and can be deserialized back into their enum type.

For custom host functions and the Wasm ABI, see [FFI and host
imports](../compiler/ffi.md). External functions must have declared types;
they are not dynamically typed escape hatches.
