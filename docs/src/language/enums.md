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
equality; use `match` to inspect a payload variant.

### Parameterized enums

Payload enums can also take type parameters. This makes the type relationship
between a payload and its enclosing variant explicit without giving up
exhaustive matching. The standard library defines this common type already:

```que
(let read-or
  (lambda (fallback option)
    (match option
      None fallback
      (Some value) value)))

{(read-or 0 (Some 4))
 (read-or "none" (Some "Calque"))}
; {4 "Calque"}
```

Use the applied form in annotations: `(Option Int)`, `(Option [Char])`, or a
generic relationship such as `(Option T)`. Recursive generic enums work the
same way:

```que
(enum Tree T
  Empty
  (Node value T left (Tree T) right (Tree T)))
```

Every use chooses one payload type. `(Some 1)` and `(Some "one")` can coexist
in a program, but they have different types: `(Option Int)` and
`(Option [Char])`.

### Typed choices without overloads

Parameterized enums are also useful when an API needs a choice that must stay
connected to the type of another argument. Calque functions are not overloaded
by argument type: one function has one type. Put the type-dependent operation
in a generic enum instead.

For example, an ordering holds the comparison function for one element type:

```que
(enum SortOrder T
  (Asc compare (T -> T -> Bool))
  (Desc compare (T -> T -> Bool)))

(sig sort-ord ((SortOrder T) -> [T] -> [T]))
(let sort-ord
  (lambda (order values)
    (match order
      (Asc compare) (sort compare values)
      (Desc compare) (sort compare values))))

(sig Asc/Int (SortOrder Int))
(let Asc/Int (Asc <))

(sig Desc/Int (SortOrder Int))
(let Desc/Int (Desc >))

(sig Asc/Dec (SortOrder Dec))
(let Asc/Dec (Asc <.))

(sort-ord Asc/Int [3 1 2])
; [1 2 3]

(sort-ord Desc/Int [3 1 2])
; [3 2 1]

(sort-ord Asc/Dec [3.2 1.1 2.4])
; [1.1 2.4 3.2]
```

`sort-ord` is one polymorphic function, not separate Int and Dec overloads.
The signature ties the `T` inside `(SortOrder T)` to the element type of
`[T]`, so `(sort-ord Asc/Int [1.2 2.4])` is a type error. The named values
`Asc/Int` and `Asc/Dec` are ordinary bindings; an enum variant name does not
need to encode the concrete type.

Payload enums are also valid `Table` keys. Their tag and fields are hashed and
compared structurally, including nested payload enums, vectors, tuples, and
records:

```que
(let table (Table/new))
(Table/set! (Node 1 Empty Empty) "stored" table)
(Table/get (Node 1 Empty Empty) table)
```

The same rule applies to a payload such as `(Named ["calque" "lisp"])`: a
separately allocated value with the same nested text is the same key. Tuples
and records can also be used directly as structural `Table` keys.

Like existing `[Char]` keys, do not mutate a managed value after using it as a
key: changing its contents can change its hash and make it unreachable in the
table.

## Serialize payload enums

Payload enums use an explicit, readable recursive format. `serialize` writes
the enum type, variant, and each field; `deserialize` validates that exact
shape against the concrete type supplied by `sig`:

```que
(enum Tree T Empty (Node value T left (Tree T) right (Tree T)))

(let saved (serialize (Node 4 Empty Empty)))
; Tree{ Node 4 Tree{ Empty } Tree{ Empty } }

(sig restored (Tree Int))
(let restored (deserialize saved))
```

Malformed text, an unknown variant, or a missing field is an error. Recursive
payloads are decoded recursively, so this format also round-trips ordinary
trees and ASTs without a separate schema.

`match/string` remains available for the older string-dispatch macro use case.
