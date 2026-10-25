# Warnings and proof recipes

Warnings are conservative: they mean Calque could not prove a required fact
from the source it can see. They are neither ignored type errors nor proof that
the program fails. Start with `--debug` and inspect the full explanation when
needed:

```sh
que program.que --debug
que explain program.que --opt
```

Put `; use-strict-warnings!` at the top of a source file to surface these
findings in editor diagnostics.

| Finding | Meaning | Usual source-level proof or fix |
|---|---|---|
| `index not proven safe` | An index may be negative or beyond vector length. | Guard with `in-bounds?`, or use a counted loop whose condition includes `(length xs)`. |
| `vector may be empty` | `car`, `pop-val!`, or a similar operation may see no elements. | Check `(> (length xs) 0)`, use a safe library variant, or restructure the branch. |
| `Int overflow/underflow possible` | Signed `i32` arithmetic is not proven in range. | Use `+/safe?`, `-/safe?`, or `*/safe?` with a value-producing `if`; constrain the range. |
| `division by zero possible` | A divisor may be zero. | Guard the divisor before `/` or `%`. |
| `termination: unknown` | The analyser did not infer a decreasing measure. | Make the base-case guard control the recursive call; use an explicit counter loop when that is clearer. |
| `unknown-call` | A function value is called indirectly and its behavior cannot be resolved. | It is valid code. Prefer a known direct helper only when that genuinely improves clarity or hot-path optimization. |
| RC cycle rejection | A managed collection would own a closure that captures the same collection. | Store IDs/indexes instead of owning back-edges; see [memory](../language/memory.md). |
| permission error | A host operation lacks an allowed WASI capability. | Grant the narrow matching `--allow` group, for example `--allow read`. |

## Bounds

The canonical dynamic guard is:

```que
(if (and (>= index 0) (< index (length xs)))
    (get index xs)
    fallback)
```

For loops, keep the relationship visible rather than calculating an unrelated
upper bound elsewhere:

```que
(loop i (< i (length xs))
  (get i xs))
```

## Arithmetic

Safety predicates prove the corresponding operation only in their true path.
Use `if` or `cond` when the expression must produce a value:

```que
(if (+/safe? left right)
    (+ left right)
    fallback)
```

Use `when` only for a body whose result is `nil`, such as a mutation:

```que
(when (+/safe? total amount)
  (alter! total (+ total amount)))
```

## Termination

Calque understands conventional counted loops and some recursive measures. A
base case must dominate the recursive call; merely placing an unrelated test
earlier in the function does not establish termination.

```que
(letrec count-down
  (lambda (n)
    (if (<= n 0)
        0
        (count-down (- n 1)))))
```

An unknown termination result is not a rejection. It means the analysis chose
not to make a proof claim. The full [correctness-analysis
chapter](../language/correctness.md) describes the model and `que explain`
output.
