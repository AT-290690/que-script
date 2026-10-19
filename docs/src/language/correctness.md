# Correctness analysis

The type checker catches incompatible values. Calque’s static analyser adds
conservative warnings about runtime properties that types alone cannot prove.

Enable strict warnings in a source file:

```que
; use-strict-warnings!
```

Or inspect a full report:

```sh
que explain program.que --opt
que explain program.que --opt --json
```

The report groups warnings before termination, effects and permissions,
performance observations, and generated-code details.

## Bounds and empty vectors

The analyser recognizes common loop shapes:

```que
(loop i (< i (length xs))
  (get xs i))
```

Here it can establish `0 <= i < length(xs)`. If it cannot prove an access, it
reports the source expression and suggests a guard such as:

```que
(and (>= index 0) (< index (length xs)))
```

It also reports calls such as `pop!`, `pop-val!`, and `car` when the vector may
be empty. A warning is not necessarily a bug: it says a proof was unavailable.

## Arithmetic

`Int` is signed `i32`. The analyser tracks ranges where it can and warns about
possible overflow, underflow, division by zero, and unsafe squares. Use the
matching predicate in a value-producing `if`:

```que
(if (+/safe? left right)
    (+ left right)
    fallback)
```

`when` is for effect-only bodies: its false branch is `nil`, so its body must
also produce `nil`. It is appropriate when the guarded operation is a mutation:

```que
(when (+/safe? total amount)
  (alter! total (+ total amount)))
```

Debug execution additionally inserts runtime guards for configured checks;
optimized execution assumes the program’s invariants. For this reason, first
use `--debug`, address real warnings, then benchmark with `--opt`.

## Termination

Counter loops and many recursive calls have an inferred measure. For example,
a counter initialized at zero, tested against a bound, and incremented toward
that bound can be reported as proven terminating. Recursive code may be
reported as `unknown` when the analyser cannot identify a decreasing measure.
That result is intentionally conservative, not a claim of non-termination.

Avoid writing a “fake” base case that does not actually guard the recursive
call; the analyser follows control flow rather than just looking for a test.
