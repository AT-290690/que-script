### The official extension for the Que Scripting language

_Requres the LSP installed_

Types

| Type  | Description                  | Example | Notes                               |
| ----- | ---------------------------- | ------- | ----------------------------------- |
| Bool  | Logical truth value          | true    | Used in conditions and logical ops  |
| Int   | 32-bit integer               | 42      | Arithmetic, indexing, counting      |
| Float | 32-bit floating point number | 42.     | Arithmetic                          |
| Char  | Single character             | 'a'     | Building blocks of strings          |
| Unit  | No meaningful value          | ()      | Returned by effect-only expressions |
| [T]   | Vector of T                  | [1 2 3] | Universal data structure            |

Notes:
• Unit represents the absence of a value (effects only)
• [T] is a homogeneous vector — all elements share the same type
• {T \* K} is a tuple (but runtime a vector). It can be of only 2 different types (even another tuple)

---

Shell eval highlighting:

Use a `QUE` heredoc in shell files to get embedded Que highlighting:

```bash
que --eval "$(cat <<'QUE'
(if (empty? ARGV) "Provide a file"
  (do
    (let [file .] ARGV)
    (let text (map lower (read! file)))
    text))
QUE
)" "./projects/rust-lisp/src/infer.rs" --allow read
```
