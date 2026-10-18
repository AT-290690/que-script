# Types

Calque infers types with Hindley–Milner inference. Most bindings need no type
annotation; incompatible uses are errors before Wasm generation.

## Core types

| Source type | Meaning |
|---|---|
| `Int` | signed 32-bit integer (`i32`) |
| `Dec` | fixed-point decimal backed by a scaled `i32` |
| `Bool` | `true` or `false` |
| `Char` | a Unicode scalar value |
| `()` | unit; `nil` has this type |
| `[T]` | vector of values of type `T` |
| `{A B}` | ordered tuple |
| `A -> B` | function from `A` to `B` |
| `Name` | a declared enum type |

Strings are `[Char]`. This makes string operations ordinary vector-oriented
operations at the language level; it does **not** mean strings are a packed
UTF-8 byte array in the current runtime representation.

```que
(let count 3)              ; Int
(let ready? true)          ; Bool
(let title "Calque")       ; [Char]
(let pair {count title})   ; {Int [Char]}
```

## Numeric types

`Int` does not silently become `Dec`. Use the decimal operators (`+.`, `-.`,
`*.`, `/.`) and conversions such as `Int->Dec` / `Dec->Int` when crossing the
boundary. The suffixed comparison operators follow the same pattern: `<.` for
`Dec`, `<#` for `Char`, and so on.

`Dec` source literals are parsed as exact base-10 values. At runtime, a `Dec`
is a scaled signed `i32` (scale `1000` by default and configurable with
`QUE_DECIMAL_SCALE` at compile time). Decimal multiplication and division use
`i64` intermediates before returning to the configured fixed-point scale.

`Int` overflow is meaningful: in debug mode guards can trap it, and strict
analysis reports arithmetic it cannot prove safe. The `/safe?`, `+/safe?`,
`*/safe?`, and related predicates can establish a guarded operation.

## Polymorphism

Library functions and ordinary inferred functions may be generic:

```que
(let first-or
  (lambda (fallback xs)
    (if (empty? xs) fallback (car xs))))
```

Its shape is conceptually `T -> [T] -> T`: the fallback and vector element
must be the same type, but callers choose that type independently.

Use a `sig` when an external boundary needs a concrete type or when you want
to state the public contract explicitly. See [Inference and
signatures](type-inference.md).
