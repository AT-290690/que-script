# Correctness analysis

The type checker catches incompatible values. Calque’s static analyser adds
conservative warnings about runtime properties that types alone cannot prove.

Enable strict warnings in a source file:

```que
; use-strict-warnings!
```

For a compact message-to-fix table, see [Warnings and proof
recipes](../reference/warnings.md).

Or inspect a full report:

```sh
que explain program.que --opt
que explain program.que --opt --json
```

The report groups warnings before termination, effects and permissions,
performance observations, and generated-code details.

## Reading `que explain`

`que explain` is the programmer-facing view of the typed program, static
proofs, and generated WAT shape. It does not run the program or benchmark it.
Use `--opt` when you want its generated-code and performance observations to
describe the optimized build you intend to run:

```sh
que explain program.que --opt
que explain program.que --opt --json --out explain.json
```

The text report has these sections:

| Section | What it tells you |
|---|---|
| **Result type** | The inferred type of the final user expression. |
| **Correctness** | Static arithmetic/bounds warnings, source locations, suggestions, and successful bounds proofs. |
| **Termination** | A proven or unknown result for analysed loops and recursive functions, including the inferred measure when available. |
| **Effects and permissions** | The combined effect (`pure`, mutation, host I/O, or unknown call) and required host imports. |
| **Performance** | A static count of allocations, dynamic applies, checked/unchecked reads, direct calls, and WAT bytes. |
| **Generated code** | Per-function allocation and call details, plus inferred source-form types and effects. |

For a safe counted access, the useful part looks conceptually like this:

```text
Correctness:
  Proven bounds:
    (get i xs) proven safe
      0 <= i <= 9

Termination:
  proven terminating: while (< i (length xs))
    i increases toward the exit bound
```

The report's performance section is deliberately **static WAT shape, not a
runtime profile**. It can identify a remaining allocation or bounds check and
point toward a useful rewrite, but timing, CPU effects, and allocator behavior
still require a real benchmark.

`--json` contains the same information in stable fields such as
`result_type`, `warnings`, `bounds_proofs`, `termination`, `effect`,
`host_imports`, `metrics`, `compiled_functions`, and `optimization_targets`.
That form is intended for editor integrations, CI, and custom reporting.

## The analysis model

Type inference answers questions such as “is this value an `Int` or a
`[Char]`?” It cannot answer “can this `Int` overflow?” or “is this index below
this particular vector's length?” Calque runs a second, flow-sensitive static
analysis after type inference for those questions.

It is an **abstract interpreter**: it executes a finite mathematical summary
of the program rather than executing its concrete values. A useful simplified
view of the state at a program point is:

```text
S = (R, L, A, P)

R(x) = [lower, upper]       possible Int range for x
L(xs) = n or >= n           known exact or minimum vector length
A                         symbolic affine relations, such as i < length(xs)
P                         path facts, aliases, and recognised guard summaries
```

For example, on the true path of this condition the analyser refines the
abstract state to include both facts needed by `get`:

```que
(if (and (>= i 0) (< i (length xs)))
    (get i xs)
    fallback)
```

```text
true path:  0 <= i < length(xs)
therefore:  (get i xs) is proven safe
```

It keeps bounded alternatives when useful—for example, a value known to be
negative *or* positive rather than merely “some `Int`”—and tracks simple
affine relations such as `a + b <= 2147483647`. Arithmetic ranges are
calculated in a wider analysis domain, then checked against the signed `i32`
range used by `Int`.

### Control flow and loops

The analyser refines facts independently along each branch. Where paths meet,
it retains only facts that remain valid on every incoming path. Informally:

```text
after (if condition left right) = join(state(left), state(right))
```

Loops require a fixed point: the body is analysed repeatedly until its summary
stops gaining useful facts. Widening deliberately broadens a changing range
when necessary so this process always finishes. That is why Calque can prove
common counted loops without attempting to solve the halting problem.

```que
(loop i (< i (length xs))
  (get i xs))
```

The loop supplies an initial value, condition, and update. The analysis derives
`0 <= i < length(xs)` for the body, proves the read safe, and can record the
counter as moving toward its exit bound.

### Proof outcomes

Each checked operation has one of three outcomes:

| Result | Meaning |
|---|---|
| **proven safe** | Every abstract execution reaching the operation satisfies the required fact. |
| **definitely invalid** | Every possible execution violates it, such as a known negative index. |
| **unknown** | The analysis cannot establish either conclusion with its conservative model. |

Only a proven-safe fact is eligible for an optimization such as omitting a
runtime bounds check. `unknown` is intentionally not a claim that the program
fails; write a guard, simplify the shape, or keep the check if the invariant
lives outside the visible source.

The same model supports several analyses:

- **bounds and non-empty vectors** — `get`, `set!`, `car`, and `pop-val!`;
- **arithmetic** — `+`, `-`, `*`, `/`, and `%`, including a non-zero divisor
  and the special `minimum Int / -1` overflow case;
- **termination evidence** — counted loops and recursive calls whose argument
  is shown to move toward a base-case guard;
- **effects** — mutation, host I/O, and calls whose effect cannot yet be
  resolved.

Recognised helper predicates can carry proof information too. A conventional
definition such as `in-bounds?` can establish a later `get` when its true
branch is taken, and `+/safe?` can establish the corresponding guarded
addition. The analyser is conservative about aliases and mutation: once a
fact may have been invalidated, it stops relying on it.

## Bounds and empty vectors

The analyser recognizes common loop shapes:

```que
(loop i (< i (length xs))
  (get i xs))
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
