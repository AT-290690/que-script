# Standard library overview

The active library is implemented in the repository’s `lisp/` directory and
is bundled into normal compilation. It is deliberately ordinary Calque source
where possible: the compiler recognizes a small set of primitive operations,
while functional helpers, collection APIs, loop conveniences, and algorithms
are written in the language itself.

## Do not guess a signature

The installed compiler can query the library it actually bundles:

```sh
que --lib names 'Table/*'
que --lib types 'map*'
que --lib source Table/update!
```

This is the authoritative API reference between releases. Start with
[Functions by example](by-example.md) for copyable everyday use, then use the
[complete library index](index.md) and the live query when you need a
specialist or implementation-level name.

## Naming and argument order

- `!` marks a mutating or effectful operation.
- `?` marks a predicate.
- `/i` is commonly an index-aware collection variant.
- `/raw` is an implementation-level helper; prefer its public counterpart.
- Namespace-like names use `/`: `Vector/sort!`, `String/contains?`,
  `Table/update!`.
- Transform-style functions usually take data last so they work with `|>`.
- Mutating collection operations normally take the collection first.

```que
(|> xs
    (map normalize)
    (select valid?)
    sum)

(Table/set! table key value)
```

## Main families

| Family | Purpose |
|---|---|
| Core vectors | `get`, `set!`, `push!`, `pop!`, `length`, `car`, `cdr` |
| Functional transforms | `map`, `select`, `filter`, `reduce`, `sort`, `range` |
| Text and conversion | `split`, `join`, `trim`, `slice`, `String/*`, `Integer->String` |
| Data structures | `Table/*`, `Set/*`, `Heap/*`, `Que/*`, `Matrix/*` |
| Numbers | decimal helpers, bit operations, random, BigInt and SignedBigInt |
| Meta and utility | dates, parsing helpers, test helpers, constants |

The following pages cover the most important usage patterns and the complete
public library families.
