# Inference and signatures

Calque infers types for ordinary bindings and generalizes them where valid. Start
without annotations:

```que
(let id (lambda (x) x))
(let pair-with (lambda (left right) {left right}))
```

The compiler rejects inconsistent uses before code generation.

## `sig`

Use `sig` immediately before a binding when the program crosses a boundary
whose type cannot be inferred from ordinary use, or when you want to state the
binding’s contract:

```que
(sig parse/int! ([Char] -> Int))
(let parse/int! (lambda (text) (deserialize text)))
```

`deserialize` is the important practical example: serialized characters do not
contain enough static information for the compiler to know which decoder to
generate. The signature supplies the expected result type.

```que
(enum Shape Rock Paper Scissors)

(sig restored [[{ Shape Int }]])
(let restored (deserialize saved))
```

The type above is the physical representation of a `Table` in the current
library: a vector of buckets, each containing `{key value}` entries. Programs
normally use `Table/new`, `Table/set!`, and `Table/get`; the representation only
needs to be written at a serialization boundary.

## Function type syntax

```que
(sig increment (Int -> Int))
(sig compare (Int -> Int -> Bool))
(sig map-int ((Int -> Int) -> [Int] -> [Int]))
```

An arrow chain associates to the right. Parenthesize a function when it is an
argument type.

## Named generic roles are rigid

Inside a `sig`, a named type variable is a role in the contract, not a
placeholder that the implementation may silently specialize. Repeated names
must remain the same type; different names must remain different roles.

```que
(sig wrap (T -> [T]))
(let wrap (lambda (x) [x])) ; valid

(sig broken (T -> [T]))
(let broken (lambda (x) [3])) ; error: T was forced to Int
```

Likewise, the order of generic roles matters:

```que
(sig pair (T -> K -> [{ T K }]))
(let pair (lambda (left right) [{right left}])) ; error
```

Use `_` for an intentionally unconstrained slot. Every `_` is a new flexible
wildcard, unlike a repeated named role:

```que
(sig keep-left (T -> _ -> T))
(let keep-left (lambda (x ignored) x))
```

## Inspect inferred types

```sh
que program.que --emit types
que explain program.que
```

The LSP also exposes inferred types in hover information. Use a `sig` to make
an intended public type visible and enforced, not merely to silence inference.
