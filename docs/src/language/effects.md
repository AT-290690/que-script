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

## Streaming input

For large input, avoid allocating the full file when a callback can process it
incrementally:

```que
(read/lines! "input.txt"
  (lambda (line)
    (println! line)
    false))
```

The callback returns a `Bool`; use it to control the streaming operation as
documented by the function’s live signature. Query the installed library when
using chunked or line APIs:

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
