# Control flow

All control forms are expressions.

## `if`, `cond`, and `when`

```que
(if (> score 0) "positive" "not positive")

(cond
  (< score 0) "negative"
  (= score 0) "zero"
  "positive")

(when ready?
  (println! "go"))

(unless ready?
  (println! "wait"))
```

`cond` tests each condition in order. Its last unpaired expression is the
default branch. `when`, `when-not`, and `unless` are bundled macros.

## `while` and `loop`

`while` runs while its condition is true. The counter-style `loop` macro
introduces and updates its named counter for you:

```que
(mut i 0)
(while (< i 3)
  (println! i)
  (++ i))

(loop i (< i 3)
  (println! i))
```

There are library macros for common forms:

```que
(loop/for i 1 (<= i 10) (++ i) (println! i))
(loop/in value [10 20 30] (println! value))
(loop/in/i (value index) [10 20] (println! {index value}))
(times 3 (println! "tick"))
```

The correctness analyser recognizes many counter loops and can show the
initial value, condition, update, and exit bound in `que explain`. It is
conservative: “unknown” means it did not establish a proof, not that the loop
is definitely infinite.

## `block` and `do`

Use `block` when a control-flow branch needs its own bindings. Use `do` only
when you need explicit sequencing without a new scope.
