# Syntax and literals

Calque uses S-expressions. Whitespace separates forms; parentheses make a
call or special form; square brackets make a vector; braces make a tuple.

```que
(+ 1 2)
(let xs [1 2 3])
(let point {10 20})
```

## Literals

```que
42             ; Int
-7             ; Int
3.14           ; Dec
true           ; Bool
false          ; Bool
nil            ; unit, type ()
'a'            ; Char
'\n'           ; newline Char
"hello"        ; [Char]
```

Comments begin with `;` and end at the line boundary:

```que
; this is a comment
(let answer 42) ; so is this
```

## Forms and patterns

```que
(lambda (left right) (+ left right))
(let {x y} {10 20})
(let [a b] [10 20])
(let [a . tail] [1 2 3])
```

The final rest pattern works in `let` and lambda vector destructuring. It
binds the remaining tail after the fixed positions. When destructuring requires
elements, static analysis may warn unless it can prove the source vector has
sufficient length.

## Operators are names

Operators are ordinary callable names in prefix position. Their suffix
communicates their type family:

```que
(+ 1 2)        ; Int
(+. 1.5 2.0)   ; Dec
(+# 'A' 1)     ; Char arithmetic
(= 2 2)        ; Int equality
(=. 2.0 2.0)   ; Dec equality
(=# 'x' 'x')   ; Char equality
(=? true true) ; Bool equality
```

Enum equality is intentionally separate: `(=~ A B)`.

## Naming conventions

Names are ordinary identifiers, so these spellings are conventions with useful
semantic meaning—not a separate method syntax. Follow them for library and
application APIs so callers can understand an operation before opening its
implementation.

| Form | Convention | Examples |
|---|---|---|
| `name?` | Returns a `Bool`; asks a question or establishes a condition. | `empty?`, `Table/has?`, `+/safe?`, `in-bounds?` |
| `name!` | Performs an observable effect: usually mutation, sometimes host I/O. | `push!`, `set!`, `capacity!`, `read!`, `println!` |
| `name/i` | An index-aware collection variant; it receives the element and its index. | `map/i`, `select/i`, `every/i?` |
| `Namespace/name` | A namespace-like library family; it is still one ordinary identifier. | `Table/get`, `String/contains?`, `Vector/sort!` |
| `From->To` | Explicit conversion between representations or numeric families. | `String->Integer`, `Int->Dec`, `Char->Int` |
| `name/raw` | Internal or lower-level helper. Prefer the corresponding public API unless you need its precise representation contract. | `Vector/append/raw!` |

`!` does **not** mean only “changes its first argument.” `push!` and `set!`
mutate a collection, while `read!`, `println!`, `sleep!`, and `random!` are
also bang-named because they interact with the host. The type/effect analysis
tracks the actual mutation and I/O behavior; the suffix makes that behavior
visible at the call site.

Likewise, `?` says a function's result is a boolean predicate, not that it is
necessarily pure. A predicate can be used directly in a condition and some
recognised guards, such as `+/safe?` or a conventional `in-bounds?` helper,
can refine facts for the static analyser.

The punctuation suffixes on primitive operators select a value family:

```que
(+ 1 2)        ; Int arithmetic
(+. 1.5 2.0)   ; Dec arithmetic
(+# 'A' 1)     ; Char arithmetic

(= 2 2)        ; Int equality
(=. 2.0 2.0)   ; Dec equality
(=# 'x' 'x')   ; Char equality
(=? true true) ; Bool equality
(=~ Rock Rock) ; enum equality
```

For the live library surface and exact inferred signatures, query the
installed compiler rather than guessing from a name:

```sh
que --lib types 'Table/*'
que --lib types '*safe?'
que --lib source Vector/sort!
```
