# Text, numbers, and utilities

## Text

Strings are `[Char]`, and the library provides both general collection helpers
and string-specific operations:

```que
(let fields (split "," "one,two,three"))
(join " / " fields)
(trim "  Calque  ")
(String/starts? "Calque" "Calque compiler")
(String/contains? "omp" "Calque compiler")
```

Useful conversion functions include `String->Integer`, `Integer->String`,
`String->Dec`, `Dec->String`, `Char->Int`, and `Int->Char`. Conversion is
explicit because an `Int`, a decimal fixed-point value, a character, and text
remain distinct static types.

## Numeric helpers

In addition to primitive arithmetic, the library provides range construction,
clamping, min/max, absolute value, parity predicates, bit conversion, random
helpers, and decimal-specific aliases. Big integers are a library-level digit
vector representation:

```sh
que --lib types 'BigInt/*'
que --lib types 'SignedBigInt/*'
```

They are useful when a calculation exceeds `Int`’s signed `i32` range, but are
not a replacement for the fixed-cost behavior of normal `Int` arithmetic.

## Dates and metadata

`Date/*` helpers parse and project date-like vectors. `Meta/*` provides simple
metadata/token helpers. These are intentionally ordinary library APIs; use the
live reference for their types and behavior before building a dependency on a
particular representation.

## Testing helpers

The library contains `test`, `test-suite/static`, `assert/static`, and related
macros for examples and compile-time-shaped test definitions. Compiler and
runtime behavior is tested from the Rust workspace as well:

```sh
cargo test
```

For program-level checks, favor small `.que` test programs run under `--debug`
and, for optimized code, compare the same expected result with `--opt`.
