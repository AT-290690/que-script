# Bindings and scope

## Immutable and mutable locals

`let` binds an ordinary lexical name. A `let` binding cannot be reassigned.

```que
(let greeting "hello")
(let answer (+ 40 2))
```

`mut` declares a mutable local. Update it with `alter!`, `+=`, `++`, and the
other mutation helpers:

```que
(mut total 0)
(alter! total (+ total 10))
(+= total 20)
(++ total)
total ; 31
```

`alter!` takes a value, not a callback. It is equivalent in spirit to
assignment after the new expression has been evaluated.

## Destructuring

Tuple and vector patterns can bind components directly:

```que
(let {name age} {"Ada" 37})
(let [first second] [10 20])
```

Vector destructuring has the usual shape constraint: a fixed pattern such as
`[first second]` requires the vector to contain those elements. The analyser
may warn when it cannot prove that fact. Rest patterns are available in lambda
parameter patterns:

```que
(let head (lambda ([x . xs]) x))
```

## Scope

Names are lexical. A binding introduced by `block` is not visible after that
block:

```que
(let label
  (if true
      (block (let value "yes") value)
      (block (let value "no") value)))
```

`do` sequences values but does **not** make a lexical scope. This distinction
matters in branches and loops: prefer `block` for branch-local temporary
bindings.

## Reference cells

`&mut`, `&get`, and `&alter!` are macros for an explicit mutable cell. They
are useful when a closure must update state that outlives a single local scope:

```que
(&mut counter 0)
(let next!
  (lambda ()
    (&alter! counter (+ (&get counter) 1))))

(next!) ; 1
(next!) ; 2
```

They preserve the compiler’s normal mutation analysis; they are source-level
convenience forms, not an untyped escape hatch.
