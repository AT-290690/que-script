# Pipelines and composition

Most functional library APIs take their data argument last. That makes a
pipeline read from input to result:

```que
(|> [1 2 3 4 5]
    (select (lambda (x) (> x 2)))
    (map (lambda (x) (* x x)))
    sum)
; 50
```

`|>` inserts the value on its left as the final argument of each following
form. The reverse pipeline `<|` inserts it as the first argument instead.

```que
(let parse-ids
  (comp
    trim
    split/lines
    (map String->Integer)))

(parse-ids "1\n2\n3")
```

`comp` builds a left-to-right data-last pipeline: the first listed function
receives the input first. Pipelines are often more readable for a one-off
transformation because they keep the data visible.

## Point-free definitions

When the input is only passed from one transformation to the next, a named
parameter adds no information. `comp` lets you define that function without
spelling the parameter: this is *point-free* style.

```que
; Explicit input and pipeline.
(let parse-ids
  (lambda (input)
    (|> input
        trim
        split/lines
        (map String->Integer))))

; The same function, point-free.
(let parse-ids
  (comp
    trim
    split/lines
    (map String->Integer)))
```

The stages still run left to right. A bare name is a stage such as `trim`; a
form such as `(map String->Integer)` supplies its non-data arguments first and
receives the composed input last. Therefore the definition above behaves like:

```que
(lambda (input)
  (map String->Integer (split/lines (trim input))))
```

This is especially useful for parsers and reusable collection transforms:

```que
(let positive-squares
  (comp
    (select (lambda (n) (> n 0)))
    (map (lambda (n) (* n n)))))

(positive-squares [-2 -1 0 1 2])
; [1 4]
```

Use `comp` when it makes the flow clearer. When a step needs to name an
intermediate value, branch, or perform several effects, an ordinary `lambda`
and `|>` are usually easier to read.

`apply` is a source-level macro that splices the written argument forms into a
call. It is useful when another macro is constructing a call; it does **not**
unpack a runtime vector:

```que
(apply + 20 22) ; expands to (+ 20 22), therefore 42
```

`|>`, `<|`, `comp`, and `apply` are bundled macros. They expand before
inference and code generation, so they do not add a dynamic dispatch cost.
Extremely long pipeline expansions are intentionally bounded to keep macro
expansion predictable; normal pipelines are unaffected.
