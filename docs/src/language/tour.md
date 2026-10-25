# Language tour

Calque is an expression language. A program is a sequence of forms and its
last value is the result. Function bodies also sequence their forms, so a
separate `do` is useful for readability but not required merely to sequence.

```que
(let classify
  (lambda (x)
    (cond (< x 0) "negative"
          (= x 0) "zero"
          "positive")))

(map classify [3 -1 0 8])
; ["positive" "negative" "zero" "positive"]
```

`block` is different from sequencing: it creates a lexical scope. Use it when
two branches or repeated loop bodies need their own local bindings.

```que
(if enabled?
    (block (let label "on") label)
    (block (let label "off") label))
```

## Data and calls

```que
[1 2 3]               ; vector
{ "Ada" 37 }          ; tuple
"hello"               ; string, typed as [Char]
'x'                   ; Char
true                   ; Bool
nil                    ; unit, type ()

(+ 20 22)             ; ordinary prefix call
(map (lambda (x) (* x x)) [1 2 3])
```

Calls are curried. Supplying fewer arguments produces a function value, which
makes partial application useful but also means that accidentally omitting an
argument can produce a closure instead of an immediate error.

```que
(let add2 (+ 2))
(add2 40) ; 42
```

## Functional and imperative code together

Collection transforms use data-last arguments so they compose naturally:

```que
(|> [1 2 3 4 5 6]
    (select (lambda (x) (= (% x 2) 0)))
    (map (lambda (x) (* x x))))
; [4 16 36]
```

For a hot update loop, use a mutable local and a mutable vector directly:

```que
(let increment-all!
  (lambda (xs)
    (loop i (< i (length xs))
      (set! i (+ (get i xs) 1) xs))
    xs))

(increment-all! [10 20 30])
; [11 21 31]
```

The following chapters spell out the rules behind these examples.
