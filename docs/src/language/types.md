# Types

Calque infers types with Hindley–Milner inference. Most bindings need no type
annotation; incompatible uses are errors before Wasm generation.

## Core types

| Source type | Meaning |
|---|---|
| `Int` | signed 32-bit integer (`i32`) |
| `Dec` | fixed-point decimal backed by a scaled `i32` |
| `Bool` | `true` or `false` |
| `Char` | a Unicode scalar value |
| `()` | unit; `nil` has this type |
| `[T]` | vector of values of type `T` |
| `{A B}` | ordered tuple |
| `A -> B` | function from `A` to `B` |
| `Name` | a declared enum type |

Strings are `[Char]`. This makes string operations ordinary vector-oriented
operations at the language level; it does **not** mean strings are a packed
UTF-8 byte array in the current runtime representation.

```que
(let count 3)              ; Int
(let ready? true)          ; Bool
(let title "Calque")       ; [Char]
(let pair {count title})   ; {Int [Char]}
```

## Numeric types

`Int` does not silently become `Dec`. Use the decimal operators (`+.`, `-.`,
`*.`, `/.`) and conversions such as `Int->Dec` / `Dec->Int` when crossing the
boundary. The suffixed comparison operators follow the same pattern: `<.` for
`Dec`, `<#` for `Char`, and so on.

`Dec` source literals are parsed as exact base-10 values. At runtime, a `Dec`
is a scaled signed `i32` (scale `1000` by default and configurable with
`QUE_DECIMAL_SCALE` at compile time). Decimal multiplication and division use
`i64` intermediates before returning to the configured fixed-point scale.

`Int` overflow is meaningful: in debug mode guards can trap it, and strict
analysis reports arithmetic it cannot prove safe. The `/safe?`, `+/safe?`,
`*/safe?`, and related predicates can establish a guarded operation.

## Polymorphism

Library functions and ordinary inferred functions may be generic:

```que
(let first-or
  (lambda (fallback xs)
    (if (empty? xs) fallback (car xs))))
```

Its shape is conceptually `T -> [T] -> T`: the fallback and vector element
must be the same type, but callers choose that type independently.

Use a `sig` when an external boundary needs a concrete type or when you want
to state the public contract explicitly. See [Inference and
signatures](type-inference.md).

## Hindley–Milner inference in Calque

Calque uses Hindley–Milner-style inference: most source programs do not need
annotations, yet a binding is checked consistently at every use. The
implementation follows the usual **Algorithm W** shape—fresh type variables,
constraints, unification, and generalization—while collecting constraints and
solving them at binding boundaries. This gives better source locations for
diagnostics than a presentation of the algorithm that only returns a type.

### The small mathematical core

The notation below is useful when reading an inferred type or a compiler
diagnostic:

```text
τ ::= Int | Dec | Bool | Char | () | α
    | [τ] | {τ τ} | τ -> τ | EnumName

σ ::= ∀α₁ … αₙ. τ
Γ ⊢ e : τ
```

`τ` is a monotype, `α` is an unknown type variable, `σ` is a polymorphic type
scheme, and `Γ ⊢ e : τ` reads “under type environment `Γ`, expression `e` has
type `τ`.” Calque source prints inferred unknowns as names such as `T0`; the
Greek letters are only the conventional mathematical notation.

For a lambda, the parameter starts with a fresh unknown type:

```text
Γ, x : α ⊢ body : β
─────────────────────
Γ ⊢ (lambda (x) body) : α -> β
```

For an application `f x`, the compiler creates a fresh result type `β` and
requires the type of `f` to unify with `type(x) -> β`. Unification recursively
matches the structure of functions, vectors, and tuples. It includes an
*occurs check*, so it cannot invent an infinite type such as `α = [α]`.

At a safe `let` binding, the compiler quantifies the variables that are free
in the value but not free in the surrounding environment:

```text
generalize(Γ, τ) = ∀(ftv(τ) − ftv(Γ)). τ
```

For example, the identity function is inferred once as `∀α. α -> α`; each use
instantiates `α` with a fresh unknown, so one call may use `Int` and another
may use `[Char]`.

```que
(let id (lambda (x) x))
{(id 42) (id "Calque")}
; {Int [Char]}
```

There is no implicit numeric subtyping or promotion in these rules. `Int` and
`Dec` unify only with themselves; use an explicit conversion when crossing
that boundary.

### Product, vector, and enum types

HM's core is deliberately small. Calque extends its type grammar with the
following structural and nominal types.

**Vectors are homogeneous.** `[T]` has exactly one element type. A vector
literal adds a constraint that every element has that same type:

```que
(let scores [10 20 30]) ; [Int]
; [10 true]             ; type error: Int and Bool do not unify
```

The same relationship is carried through vector operations:

```text
get    : [T] -> Int -> T
set!   : [T] -> Int -> T -> ()
push!  : [T] -> T -> ()
```

**Tuples are ordered pairs and may be heterogeneous.** `{A B}` means the
first field has type `A` and the second has type `B`; unification checks both
positions independently.

```que
(let entry {"Ada" 37}) ; {[Char] Int}
```

**Enums are nominal, closed types.** An enum declaration makes every variant
belong to that one named type. Its compact runtime `i32` tag does not make it
an `Int` at the language level. `=~` requires both operands to have the same
enum type, and `match` checks coverage of that closed variant set. See [Enums
and matching](enums.md).

### Mutation and the value restriction

Plain HM generalizes every suitable `let`. That is unsound in the presence of
mutable containers: if an empty vector were independently polymorphic at each
use, one use could store an `Int` and another could read it as a `Bool`.

Calque therefore uses a value restriction. It generalizes non-expansive
bindings—such as literals, names, and lambdas—but an allocating expression
such as a vector is kept monomorphic for its binding. Its element type is
still inferred; it is simply shared by every later mutation.

```que
(let xs [])
(push! xs 1)       ; fixes xs as [Int]
; (push! xs true)  ; type error
```

`mut` bindings are monomorphic by design and retain one type throughout their
updates:

```que
(mut total 0)
(alter! total (+ total 1))
; (alter! total true) ; type error
```

The `&mut`, `&get`, and `&alter!` forms are library macros over an ordinary
one-element vector cell, so they inherit the same vector constraints rather
than introducing a hidden second reference type.

Effects are tracked alongside inferred expressions as compiler metadata:
pure, mutation, host I/O, or an unknown call. They help diagnostics,
permission checking, and optimization, but they are **not** currently an
effect-row type system: an arrow such as `Int -> Int` does not encode an
effect. See [Effects and permissions](effects.md).

### Recursion, macros, and signatures

For `letrec`, Calque first binds the recursive name to a fresh monomorphic
placeholder, infers its value, and then unifies the placeholder with that
value. This is why a recursive call must agree with the function being
defined. `letrec` is intended for self-recursive functions; mutually recursive
definitions do not have a separate forward-declaration form.

Macros expand before inference. The expanded ordinary Calque expression is
what receives these HM constraints, which keeps macros syntactic rather than
an untyped escape hatch.

`sig` writes a contract in the same type language and verifies the inferred
binding against it. It is useful for public APIs and for boundaries such as
`deserialize`, where the desired result type cannot be recovered from the
serialized text alone. See [Inference and signatures](type-inference.md) for
the syntax and diagnostics.
