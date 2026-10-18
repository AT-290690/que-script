# Macros

Macros transform syntax before the program is inferred and compiled. They are
written in Calque using `letmacro`, quasiquote (`qq`), unquote (`uq`), and
unquote-splice (`uqs`).

```que
(letmacro unless
  ((test body)
    (qq (if (uq test) nil (uq body)))))
```

```que
(unless (= 1 2)
  (println! "runs"))
```

Macros are appropriate when you need new surface syntax or want to generate a
repeated structural pattern. They are not ordinary runtime functions: they
receive syntax, not evaluated values.

## Bundled macros

The bundled library implements many familiar forms as macros, including:

- `when`, `when-not`, `unless`, `cond`, `!=`, `<>`
- `|>`, `<|`, `comp`, `apply`
- `loop`, `loop/for`, `loop/in`, `loop/in/i`, range-loop helpers, `times`
- `let*`, `+=`, `-=`, `*=`, `/=`, `++`, `--`
- `&mut`, `&get`, `&alter!`, `variable`
- `match/string`

Use `que <file> --emit source` to inspect expanded source and understand what
a macro call becomes. Formatter, LSP, and diagnostics preserve source
locations where possible, but macro-generated code can still make a complex
error easier to understand after looking at that expansion.

## Macro discipline

Prefer a function when values are enough. Prefer a macro when the caller needs
new binding structure, control flow, or syntactic rearrangement. Keep macro
expansions small, hygienic in spirit, and transparent enough that the expanded
form remains ordinary, type-checkable Calque.
