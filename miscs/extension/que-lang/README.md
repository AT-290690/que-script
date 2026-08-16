### Eclisp language support for Que

Eclisp is the Lisp-like language. Que is the toolchain that runs it, compiles it, explains it, and provides the language server.

This extension keeps the existing `.que` file extension and uses the `quelsp` binary for editor features.

Requires the Que LSP binary (`quelsp`) to be installed or configured with:

```json
"que.languageServer.path": "/absolute/path/to/quelsp"
```

Works in:

- `.que` files
- shell heredocs passed to `que --eval`, for example `<<'QUE' ... QUE`

Types:

| Type | Description               | Example | Notes                               |
| ---- | ------------------------- | ------- | ----------------------------------- |
| Bool | Logical truth value       | true    | Used in conditions and logical ops  |
| Int  | 32-bit integer            | 42      | Arithmetic, indexing, counting      |
| Dec  | 32-bit fixed-point number | 42.0    | Decimal arithmetic                  |
| Char | Single character          | 'a'     | Building block of strings           |
| Unit | No meaningful value       | nil     | Returned by effect-only expressions |
| [T]  | Vector of T               | [1 2 3] | Homogeneous sequence                |
| {T K}| Tuple                     | {1 true}| Fixed structural grouping           |

Notes:

- Strings are vectors of chars: `[Char]`.
- Vectors are homogeneous: all elements share the same type.
- Tuples are fixed-shape product values and can contain different field types.
- `do` sequences expressions without creating a new scope.
- `block` sequences expressions and creates a new scope.

Shell eval highlighting:

Use a `QUE` heredoc in shell files to get embedded Eclisp highlighting:

```bash
que --eval "$(cat <<'QUE'
(if (empty? ARGV) "Provide a file"
  (do
    (let [file] ARGV)
    (let text (map lower (read! file)))
    text))
QUE
)" "./path/to/file.txt" --allow read
```
