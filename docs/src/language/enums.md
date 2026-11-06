# Enums and matching

An `enum` declares a closed set of variants. A zero-payload enum is the most
compact form:

```que
(enum Shape Rock Paper Scissors)
```

`Rock`, `Paper`, and `Scissors` have type `Shape`; they are not `Int` values
even though the compiler represents them compactly as `i32` tags at runtime.
That separation prevents mixing unrelated domains by accident.

If names would collide, choose qualified ordinary identifiers yourself:

```que
(enum Traffic Traffic/Red Traffic/Amber Traffic/Green)
```

## Compare enum values

Use `=~` for enum equality:

```que
(=~ Rock Paper) ; false
(=~ Rock Rock)  ; true
```

Both operands must have the same enum type. `=~ Rock 1` and comparison between
values from two distinct enums are type errors. Ordinary `=` remains the
integer equality operator; this deliberate distinction stops an enum tag from
silently becoming an arbitrary number.

## Exhaustive `match`

`match` is for enum dispatch. It requires every variant, unless the final
branch is `_`:

```que
(let score
  (lambda (shape)
    (match shape
      Rock 1
      Paper 2
      Scissors 3)))
```

```que
(let describe
  (lambda (shape)
    (match shape
      Rock "rock"
      _ "another shape")))
```

The match expression is checked statically. Adding a variant to a fully
matched enum exposes every site that must decide how to handle it.

## Enums in functions and Tables

Enum values flow through ordinary variables and functions:

```que
(let winner?
  (lambda ({left right})
    (or (and (=~ left Rock) (=~ right Scissors))
        (and (=~ left Paper) (=~ right Rock))
        (and (=~ left Scissors) (=~ right Paper)))))
```

They are also efficient `Table` keys. Their compact tags are hashed and
compared directly—there is no conversion to a string:

```que
(sig points [[{ Shape Int }]])
(let points (Table/new))
(Table/set! Rock 1 points)
(Table/set! Paper 2 points)
```

## Payload and recursive variants

A parenthesized variant declares named payload fields. This is how to model
trees, linked values, ASTs, and other recursive data:

```que
(enum Tree
  Empty
  (Node value Int left Tree right Tree))

(let tree (Node 4 (Node 2 Empty Empty) (Node 7 Empty Empty)))

(letrec size
  (lambda (tree)
    (match tree
      Empty 0
      (Node value left right)
        (+ 1 (+ (size left) (size right))))))

(size tree) ; 3
```

The payload binding count must match the variant declaration. Payload enums
are boxed and reference-counted; plain zero-payload enums remain immediate
`i32` tags. Recursive *types* are therefore fine, but managed ownership
cycles are not: a tree may own its children, while a cycle such as `a → b → a`
is rejected by Calque's RC-cycle checks.

`=~` remains useful for zero-payload state enums. It is not structural payload
equality; use `match` to inspect a payload variant. Payload enums are not
currently supported as `Table` keys.

## Serialize payload enums

Payload enums use an explicit, readable recursive format. `serialize` writes
the enum type, variant, and each field; `deserialize` validates that exact
shape against the concrete type supplied by `sig`:

```que
(enum Tree Empty (Node value Int left Tree right Tree))

(let saved (serialize (Node 4 Empty Empty)))
; Tree{ Node 4 Tree{ Empty } Tree{ Empty } }

(sig restored Tree)
(let restored (deserialize saved))
```

Malformed text, an unknown variant, or a missing field is an error. Recursive
payloads are decoded recursively, so this format also round-trips ordinary
trees and ASTs without a separate schema.

`match/string` remains available for the older string-dispatch macro use case.
