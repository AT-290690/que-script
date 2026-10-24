# Debugging cookbook

Calque separates type errors, static warnings, runtime guards, permissions,
and generated-code inspection. Start with the mode that preserves the most
information:

```sh
que program.que --debug
```

After the program's invariants are established, inspect and run the optimized
form:

```sh
que explain program.que --opt
que program.que --opt
```

## “Undefined variable” or an unexpected binding

Bindings are lexical. `let` creates an immutable local, `mut` a mutable local,
and `letrec` is required only when a binding calls itself. Check for a name
outside its scope or a recursive function written with `let`:

```que
; Wrong: `factorial` is not in scope while its value is being defined.
(let factorial
  (lambda (n) (factorial (- n 1))))

; Right.
(letrec factorial
  (lambda (n)
    (if (<= n 1) 1 (* n (factorial (- n 1))))))
```

For macro-heavy code, inspect the expanded source. This distinguishes your
source binding from a generated temporary without exposing implementation
details in normal diagnostics:

```sh
que program.que --emit source --out expanded.que
```

## “Expected … but got …”

The error normally identifies the argument or expression that conflicts. Ask
the compiler for the inferred shape of a library operation instead of guessing:

```sh
que --lib types 'Table/get*'
que --lib types 'map*'
```

At a public boundary, add a `sig` to state the intended type. This both records
the contract and makes an accidental specialization easier to find:

```que
(sig square (Int -> Int))
(let square (lambda (n) (* n n)))
```

Check destructuring too: `[head . tail]` needs a non-empty vector, while
`{left right}` expects a two-field tuple.

The compiler and both language servers collect independent top-level type
errors together. During diagnostic recovery only, a failed declaration gets an
internal unknown placeholder so later forms can still be checked; it never
makes the program valid or compilable. This avoids distracting “undefined
variable” cascades while still finding other errors in those later forms.

## A bounds, empty-vector, or arithmetic warning

Warnings mean the analyser could not prove the required fact. They are not all
runtime failures, but they deserve an explicit decision. Run the full report:

```sh
que explain program.que --opt
```

Make the condition visible at the use site:

```que
(if (and (>= index 0) (< index (length xs)))
    (get xs index)
    fallback)
```

For arithmetic, make both outcomes value-producing with `if` or `cond`:

```que
(if (+/safe? total amount)
    (+ total amount)
    total)
```

`when` is for an effect-only body and returns `nil` when its condition is
false. Use it for a guarded mutation, not to produce an `Int` result.

## A debug runtime trap

`--debug` retains configured runtime guards for operations that static analysis
could not prove safe. A trap such as integer overflow, division by zero, or an
out-of-bounds vector access is evidence that the input reached an invalid
state—not merely a compiler warning.

Fix the source invariant first. The diagnostic points at the original Calque
source span; if a macro is involved, emit expanded source as shown above. Once
the program is correct in debug mode, use `--opt` for release timing.

## An operation is denied at runtime

File, terminal, clock, and stdin functions require explicit permissions under
the external WASI runtime. Grant only the needed groups:

```sh
que report.que --debug --allow read print
```

Use `--allow all` only for trusted local experiments. See [Effects and
permissions](../language/effects.md) for the operation-to-permission mapping.

## The optimized program differs from debug

That is a compiler bug, not an expected trade-off. Preserve the smallest source
that demonstrates it and compare the forms:

```sh
que program.que --debug
que program.que --opt
que program.que --opt --emit opt-source --out optimized.que
que program.que --opt --emit wat --out program.wat
```

If you use the native path, compare that too:

```sh
./scripts/compile-native-c.sh program.que
```

Record the compiler version, command lines, expected output, actual output,
and the smallest reproducer. Do not work around a debug/optimized mismatch by
shipping the unverified optimized result.

## A program is correct but unexpectedly slow

First find out whether the issue is generated shape or measured runtime:

```sh
que explain program.que --opt
que program.que --opt
```

The explain report can reveal remaining allocations, dynamic application, or
checked accesses. It cannot tell you elapsed time. Benchmark multiple complete
runs, then inspect optimized source and WAT only after the benchmark identifies
a real hot path. The [Performance guide](../language/performance.md) gives the
common source-level choices.

## Editor diagnostics look stale

Save the buffer and confirm the command-line compiler sees the same source:

```sh
que program.que --debug
```

Then confirm the editor launched the intended `quelsp` from `PATH`. Restarting
the language server is appropriate after installing a new release. If the
command-line and editor diagnostics disagree on saved source, keep the source
and both messages as a language-server bug report.
