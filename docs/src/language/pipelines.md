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
form. The reverse pipeline `<|` feeds a value into the final position from the
other direction.

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
