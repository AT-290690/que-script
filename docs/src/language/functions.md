# Functions and recursion

## Lambdas and calls

Use `lambda` to build a function. Parameters can be written as one name or a
group:

```que
(let square (lambda (x) (* x x)))
(let add (lambda (left right) (+ left right)))

(square 9)
(add 20 22)
```

Multiple forms in a lambda body are evaluated in order; its final form is the
return value.

```que
(let magnitude
  (lambda (x)
    (let value (if (< x 0) (- 0 x) x))
    value))
```

Functions are first class and lexical closures capture the bindings they use.
Calls support partial application, so `(map square)` is a function waiting for
its vector argument.

## Recursive bindings

Use `letrec` when the binding calls itself:

```que
(letrec factorial
  (lambda (n)
    (if (<= n 1)
        1
        (* n (factorial (- n 1))))))
```

Recursion is intentionally explicit. A `let` binding cannot refer to itself;
this avoids accidental forward references and makes recursive intent obvious.
The current type system does not permit arbitrary mutually recursive
definitions through forward references.

## Tail calls and loops

Write direct loops for work that is naturally iterative. Calque recognizes and
optimizes important tail-recursive shapes, but a `loop` / `while` makes the
space behavior explicit and maps directly to Wasm control flow.

```que
(let factorial
  (lambda (n)
    (mut result 1)
    (loop i (<= i n)
      (alter! result (* result i)))
    result))
```

Use `que explain program.que --opt` to see whether the analyser proved a
recursive function or loop terminating, and use the generated output when
investigating performance.
