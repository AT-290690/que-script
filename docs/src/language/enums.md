# Enums and matching

An `enum` declares a closed set of zero-payload values:

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

Current enums have no payload variants. `match/string` remains available for
the older string-dispatch macro use case.
