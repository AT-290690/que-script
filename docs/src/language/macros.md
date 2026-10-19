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

## Writing a macro

A macro transformer receives the **written syntax** at its call site and must
return replacement syntax. The smallest form uses a lambda transformer and
quasiquote:

```que
(letmacro add1
  (lambda (x)
    (qq (+ (uq x) 1))))

(add1 41)
; expands to: (+ 41 1)
; 42
```

`qq` constructs the output program. Inside it, ordinary words and lists are
literal output syntax. `(uq x)` inserts the single syntax form supplied as
`x`; it does **not** evaluate that form. In the example, `(add1 (+ 20 21))`
becomes `(+ (+ 20 21) 1)`, then normal type checking and evaluation happen.

Macros can also use pattern clauses. This version of `unless` accepts zero or
more body forms, and uses `uqs` to splice all of them into the generated
`do`:

```que
(letmacro unless*
  (()
    (qq nil))
  ((test . body)
    (qq (if (uq test) nil (do (uqs body))))))

(mut total 0)
(unless* false
  (++ total)
  (++ total))
total
; 2
```

Use `uq` for one expression and `uqs` only where a surrounding list or body
accepts several forms. The rest pattern `. body` is syntax-list rest capture;
it is not a runtime vector value.

### Generated bindings and `gensym`

If a macro introduces a local name, create it with `gensym`. It produces a
fresh identifier, avoiding accidental capture of a caller's name. This macro
evaluates its input once, then reads its final element:

```que
(letmacro last-item
  (lambda (xs)
    (do
      (let values (gensym))
      (qq
        (block
          (let (uq values) (uq xs))
          (get (uq values) (- (length (uq values)) 1)))))))

(last-item [10 20 30])
; 30
```

The generated `values` cannot collide with a caller's `values` binding. Like
`car`, this macro requires a non-empty vector; macros preserve the ordinary
runtime and static-safety rules of the forms they expand to.

### Syntax-time helpers

For macros that inspect or rebuild an arbitrary syntax list, use the
syntax-only helpers `syntax/list?`, `syntax/empty?`, `syntax/head`,
`syntax/tail`, `syntax/prepend`, `syntax/append`, `syntax/word?`, and
`syntax/word=?`. They are available only while a `letmacro` expands; they are
not runtime data-structure functions. The implementation of `|>` in the
bundled library is the reference example of this more advanced style.

## Debugging an expansion

Define a macro before its use, keep its expansion small, and inspect what the
compiler will receive when a type error is unclear:

```sh
que program.que --emit source
```

The result is expanded Calque source. It is particularly useful for checking
`uq` versus `uqs`, generated bindings, and a macro that introduces control
flow. The expanded result must still type-check like handwritten Calque.
