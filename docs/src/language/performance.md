# Performance guide

Calque is designed so ordinary, direct source can compile efficiently. Start
with clear code, establish its invariants in `--debug`, inspect the optimized
shape with `que explain`, and measure the complete program. Do not optimize
from source appearance alone.

```sh
que program.que --debug
que explain program.que --opt
que program.que --opt
```

`que explain` reports static generated-code facts—such as remaining
allocations, direct calls, and checked or unchecked reads. It is not a timing
profiler. Use a repeatable wall-clock benchmark for timing decisions.

## Choose the simplest fitting shape

| Situation | Prefer |
|---|---|
| One clear collection transformation | `|>`, `map`, `select`, `reduce`, `sum` |
| Reusable transformation with no meaningful input name | `comp` |
| Tight numeric or indexed work | `loop` / `while` and explicit mutable state |
| A known burst of appends | `capacity!` before `push!` |
| Reusing a temporary vector | `empty!`, retaining its capacity |
| A completed compute-heavy program | the repository wasm2c/native-C path, after measuring |

Functional collection code is not automatically slow. The optimizer can fuse
supported chains of collection operations, avoiding intermediate vectors. Keep
the natural pipeline first:

```que
(let sum-positive-squares
  (lambda (xs)
    (|> xs
        (select (lambda (n) (> n 0)))
        (map (lambda (n) (* n n)))
        sum)))
```

Then use `que explain program.que --opt` to see whether the generated shape
still contains avoidable allocations. An opaque callback, escaping collection,
or operation with different semantics may correctly prevent fusion.

## Allocate deliberately

Vectors grow when necessary, but a vector that repeatedly grows must allocate
and copy backing storage. Reserve once when the eventual append count is known:

```que
(let squares [])
(capacity! squares 1000)

(loop i (< i 1000)
  (push! squares (* i i)))
```

`capacity!` is only a reservation: it does not set the length or impose a
maximum. If the same scratch vector is refilled many times, `empty!` resets
its length while keeping that capacity.

For a table with an approximate number of entries, use `Table/new/capacity`.
This avoids repeated hash-table resizing; it does not change table semantics.

## Make proofs visible

The analyser and optimizer work from source-level facts. A counted loop makes
both the access invariant and the intended work clear:

```que
(let total 0)
(loop i (< i (length xs))
  (alter! total (+ total (get xs i))))
```

Here the compiler can establish `0 <= i < length(xs)`. In optimized output it
can use that proof to remove the corresponding bounds check. If an invariant
comes from input validation or another source the analyser cannot see, state it
as a guard rather than assuming `--opt` will discover it:

```que
(if (and (>= index 0) (< index (length xs)))
    (get xs index)
    fallback)
```

Likewise, use arithmetic safety predicates when a calculation reaches the
edge of `Int`'s signed `i32` range:

```que
(if (+/safe? left right)
    (+ left right)
    fallback)
```

## Keep ownership simple

Vectors, tuples, strings, and closures use deterministic reference counting.
The compiler can transfer a fresh value, borrow an immutable value, and remove
some final-use retains/releases. It has the most freedom when a value has an
obvious owner and does not escape through an unknown callback or mutable alias.

This is a reason to write direct expressions rather than manually simulating
memory management. There is no source-level retain/release operation to call.
See [Memory management and ownership](memory.md) for the model and its
no-cycle rule.

## Recursion and loops

Use a loop when the algorithm is naturally iterative. Direct self tail calls
that meet the current tail-call safety policy are lowered to a Wasm local loop
in optimized builds, so they also work through wasm2c. Non-tail recursion,
such as naive Fibonacci, still consumes one frame per outstanding call and
cannot be converted into a loop without changing the algorithm.

```que
(letrec count-down
  (lambda (n total)
    (if (= n 0)
        total
        (count-down (- n 1) (+ total 1)))))
```

Use `que explain program.que --opt` and a real benchmark before choosing
between the recursive and loop forms.

## Select an execution path after measuring

The ordinary command uses an external WASI runtime, normally Wasmtime:

```sh
que program.que --opt
```

For a completed compute-heavy program, the repository can lower the same Wasm
through wasm2c and a C compiler:

```sh
./scripts/compile-native-c.sh program.que
```

The source language and correctness pipeline are the same in both cases. The
best path depends on startup cost, workload length, host compiler, and runtime;
benchmark the exact deployment shape rather than assuming native C always wins.
