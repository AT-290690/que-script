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
| `Name` | a declared non-generic enum or record type |
| `(Name T K)` | an application of a parameterized enum or record type |

Strings are `[Char]`. This makes string operations ordinary vector-oriented
operations at the language level; it does **not** mean strings are a packed
UTF-8 byte array in the current runtime representation.

The standard library also provides the generic enum `(Option T)` with `None`
and `(Some value)`. Safe lookups use it to represent a present value or a
missing one without a sentinel value:

```que
(match (Vector/get* 1 [10 20])
  None 0
  (Some value) value)
; 20
```

For operations that need to report *why* they failed, use the generic
`(Result T E)` enum. `Result/ok` carries a successful `T`; `Result/err`
carries an error value `E`:

```que
(let divide
  (lambda (numerator denominator)
    (if (= denominator 0)
      (Result/err "division by zero")
      (Result/ok (/ numerator denominator)))))

(match (divide 12 3)
  (Result/ok value) value
  (Result/err message) 0)
; 4
```

```que
(let count 3)              ; Int
(let ready? true)          ; Bool
(let title "Calque")       ; [Char]
(let pair {count title})   ; {Int [Char]}
```

## Records

`record` declares a named, nominal fixed-field product type. Unlike a tuple,
a record carries its declared identity: a raw `{Int Int}` tuple cannot be used
where a `Point` is required, even if its elements have the same types.

```que
(record Point x Int y Int)

(let origin (Point 0 0))
(Point/x origin) ; 0
(Point/y origin) ; 0
```

Destructure by naming the record and its fields. The compiler expands this to
the generated accessors, then checks that the value really is that record:

```que
(let (Point x y) origin)
(+ x y) ; 0
```

Use `_` to skip a field: `(let (Point x _) origin)`.

Use `(field local-name)` when the field needs a different local binding—for
example, because it would collide with a standard function:

```que
(record Pair first Int second Int)
(let pair (Pair 3 4))
(let (Pair (first left) (second right)) pair)
(+ left right) ; 7
```

The declaration syntax is flat: `(record Name field Type ...)`. It creates a
constructor named `Name` and a typed accessor for every field, named
`Name/field`. Fields may use any concrete Calque type, including strings,
vectors, tuples, other already-declared records, and functions. Records work
in vectors and functions:

```que
(record Message title [Char] tags [[Char]] count Int)
(let message (Message "Calque" ["typed" "lisp"] 2))
(Message/title message) ; "Calque"
```

Constructors and accessors are ordinary first-class, curried functions. They
can be assigned, partially applied, or stored in collections:

```que
(record Point x Int y Int)
(let make Point)
(let make-x (make 4))
(let get-x Point/x)
(get-x (make-x 9)) ; 4
```

### Parameterized records

Records can take named type parameters after their name. These are ordinary
Hindley–Milner type variables: the constructor and accessors are polymorphic,
while every individual record value has one concrete instantiation.

```que
(record Pair T K first T second K)

(let number-and-name (Pair 7 "Calque"))
(Pair/first number-and-name)  ; 7
(Pair/second number-and-name) ; "Calque"
```

In type annotations, apply the type constructor with parentheses:

```que
(sig swap ((Pair T K) -> (Pair K T)))
(let swap
  (lambda (pair)
    (Pair (Pair/second pair) (Pair/first pair))))
```

`T` and `K` are just conventional parameter names; different names denote
different roles. A generic declaration is nominal, so `(Pair Int [Char])` is
still distinct from a tuple `{Int [Char]}`.

A record owns its managed fields. The runtime retains vector, tuple, record,
and function fields on construction when necessary and releases them when the
record's final reference is gone. Known ownership cycles involving a closure
stored through a record are rejected by the same compile-time RC-cycle check
used for vectors.

Records are nominal now; their future packed `Vector<Record>` representation
is a separate storage optimization. Do not rely on the current implementation
layout as an FFI ABI.

An individual record uses one runtime allocation: its fixed fields and the
small ownership bitmap for managed fields are stored inline with its header.
Boxed payload-enum values use the same compact layout. This is an internal
runtime detail, not an FFI ABI guarantee.

Records serialize with their type and field names, and can be deserialized
when a `sig` supplies the expected record type:

```que
(record Point x Int y Int)
(let saved (serialize (Point 3 4)))
(sig restored Point)
(let restored (deserialize saved))
```

The same applies to a parameterized record: provide its fully instantiated
type at the deserialization boundary.

```que
(record Pair T K first T second K)
(sig restored (Pair Int [Char]))
(let restored (deserialize "Pair{ first 3 second \"Calque\"}"))
```

Deserialization is strict: missing or malformed numeric fields trap instead of
silently becoming `0`.

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
    | [τ] | {τ τ} | τ -> τ | EnumName | RecordName

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

**Records are nominal product types.** A record declaration fixes both its
field names and field order. The type checker never treats a tuple with the
same field types as that record, which prevents accidental field-order bugs.

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
(push! 1 xs)       ; fixes xs as [Int]
; (push! true xs)  ; type error
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
