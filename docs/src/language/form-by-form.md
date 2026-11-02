# Calque, form by form

This is a guided tour of what happens between a Calque source form and a
running WebAssembly program. It is a map of the system, not a replacement for
the language reference.

Calque deliberately combines two styles: direct functional transformations
when they are clearest, and explicit loops and mutable vectors when work and
storage should be visible in the source.

```que
(let sum-even-squares
  (lambda (xs)
    (|> xs
        (select (lambda (x) (= (% x 2) 0)))
        (map (lambda (x) (* x x)))
        sum)))

(sum-even-squares [1 2 3 4 5 6])
; 56
```

The exact generated instructions may improve between releases. The language
rules and safety model below are the contract.

## 1. Forms become ordinary expressions

Calque is an expression-oriented Lisp. Calls are prefix forms, vectors use
square brackets, tuples use braces, and the final expression of a program or
function body is its value. In the example, `sum-even-squares` is bound to a
function and the final call produces an `Int`.

`|>` is convenient surface syntax, not a second evaluation model. It is a
macro that rewrites the source into ordinary, data-last calls before type
inference. Conceptually, the function body becomes:

```que
(sum
  (map (lambda (x) (* x x))
       (select (lambda (x) (= (% x 2) 0)) xs)))
```

That is why pipelines, `comp`, and explicit calls have the same type rules and
runtime behaviour. Other bundled conveniences such as `loop`, `when`, `cond`,
`+=`, and `match/string` are macros too: they generate normal Calque forms;
they do not bypass type checking or ownership.

Inspect an expansion when it is surprising:

```sh
que program.que --emit source
```

See [Pipelines and composition](pipelines.md) and [Macros](macros.md).

## 2. Inference gives every form a shape

After macro expansion, Calque infers a type for every expression. The example
has the conceptual type:

```text
sum-even-squares : [Int] -> Int
```

The lambda parameter begins unknown. `%` constrains its elements to `Int`;
`select` preserves the element type; `map` produces another `[Int]`; and `sum`
produces an `Int`. No annotation was needed, but an inconsistent use is still
rejected before Wasm generation.

This is Hindley–Milner-style inference extended with vectors, tuples, nominal
closed enums, and a value restriction for mutable allocations. An empty vector
does not become independently polymorphic at every use:

```que
(let values [])
(push! 1 values)       ; values is now [Int]
; (push! true values)  ; type error
```

Enum tags compile compactly but are not `Int` values in the language type
system. That prevents accidentally mixing domains and lets `match` check that
all cases are covered:

```que
(enum Light Red Amber Green)

(let action
  (lambda (light)
    (match light
      Red "stop"
      Amber "wait"
      Green "go")))
```

Read [Types](types.md), [Inference and signatures](type-inference.md), and
[Enums and matching](enums.md) for the full rules.

## 3. Analysis proves runtime facts

Types establish that a value is an `Int`, a `[T]`, or a function. They cannot
alone establish that an index is within *this* vector's current length, that an
addition cannot overflow, or that a loop makes progress.

Calque therefore runs a flow-sensitive static analysis after inference. A
counted loop makes a useful bounds invariant visible:

```que
(let copy
  (lambda (xs)
    (let out [])
    (capacity! (length xs) out)
    (loop i (< i (length xs))
      (push! (get i xs) out))
    out))
```

Inside the loop, the analyser can derive `0 <= i < length(xs)`, so the `get`
is eligible for unchecked optimized lowering. If it cannot prove a property,
that is a warning—not proof that the program is wrong. Make an input-dependent
invariant explicit with a guard:

```que
(if (and (>= index 0) (< index (length xs)))
    (get index xs)
    fallback)
```

The same analysis tracks simple arithmetic ranges, recognised safety
predicates, loop progress, effects, and selected ownership facts. It is
conservative: it removes a check only after a proof, and discards facts that a
mutation or unknown call could invalidate.

```sh
que explain program.que --opt
que explain program.que --opt --json --out explain.json
```

`que explain` reports result type, warnings, proven bounds, termination
evidence, effects, allocation/call shape, and generated-code observations. It
is a static report, not a timing profiler. See [Correctness analysis](correctness.md)
and [Warnings and proof recipes](../reference/warnings.md).

## 4. Values are immediate or managed

The core scalar values are immediate Wasm values:

| Calque value | Runtime representation |
|---|---|
| `Int` | signed `i32` |
| `Dec` | scaled signed `i32` |
| `Bool`, `Char`, unit | immediate `i32` values |
| enum value | compact `i32` tag, still nominally typed |

Vectors, strings, tuples containing managed values, and closures are managed
values. A string is `[Char]` at the language level; the current runtime does
**not** store it as a packed UTF-8 byte array. String operations are therefore
ordinary vector operations, with the same predictable trade-off in memory use.

The distinction is practical: scalar computations do not allocate or use
reference counting; managed-memory work happens where a program constructs or
shares managed values.

## 5. Managed values have deterministic ownership

Calque has no tracing garbage collector. The compiler inserts reference counts,
retains, releases, and ownership transfers for managed values. Ordinary source
code has no manual `free`, borrow annotation, lifetime parameter, or retain.

```que
(let xs [10 20 30])
(let alias xs)
(length alias)
```

`xs` and `alias` safely share one vector. It remains live until its last owning
reference is gone. When a fresh value has one known destination, optimized
code can transfer it directly rather than retain then release it again.

Reference counting cannot reclaim an ownership cycle. Calque rejects ordinary
source programs that would create one, such as storing a closure in a vector
when that closure captures the same vector:

```que
(let callbacks [])
(let count-callbacks (lambda () (length callbacks)))

; Rejected: callbacks would own a closure that captures callbacks.
; (push! count-callbacks callbacks)
```

Logical graph cycles are still useful. Represent graph edges with IDs, enum
tags, or table keys instead of managed back-references. For reusable work
buffers, `empty!` releases elements while retaining a vector's backing storage,
and `capacity!` reserves space before known growth:

```que
(let work [])
(capacity! 1000 work)
; fill and consume work
(empty! work)
```

Read [Memory management and ownership](memory.md) for the exact model and FFI
boundary rules.

## 6. Functional transformations and explicit updates share one backend

Pipelines make input-to-output transformations easy to read. A hot indexed
algorithm can instead make its loop order and updates obvious:

```que
(let increment-all!
  (lambda (xs)
    (loop i (< i (length xs))
      (set! i (+ (get i xs) 1) xs))
    xs))
```

There is no separate functional and imperative runtime. Both forms infer
types, run through the same analysis, and lower through the same backend. The
`!` convention generally marks mutation or a host action; `?` generally marks
a predicate. It improves readability, while types and effects remain the
authority.

Use functional transforms for a direct data story. Use `loop`, `set!`, `push!`,
and `capacity!` when they make the algorithm's work or allocation behaviour
clearer. See [Mutation and capacity](mutation.md) and [Performance guide](performance.md).

## 7. Effects are explicit at the execution boundary

Pure computations need no permission. File access, terminal output, clocks,
randomness, and custom `extern` imports are recognised host effects. The
runner requires the capability explicitly:

```que
(let source (read! "input.txt"))
(println! source)
```

```sh
que program.que --debug --allow read print
```

The program is still typed before it runs. Effects are compiler metadata used
for diagnostics, permission reporting, and optimization; function arrows do
not encode effects today. See [Effects and permissions](effects.md) and
[FFI and host imports](../compiler/ffi.md).

## 8. Checked source lowers to WebAssembly

Once a program has expanded, inferred, and been analysed, Calque optimizes the
typed representation and lowers it to WebAssembly. The default runner executes
that Wasm through an external WASI runtime, normally Wasmtime.

```sh
que program.que --debug
que program.que --opt
que compile program.que --opt --out program.wasm
que wat program.que
```

`--debug` keeps runtime guards for unproven vector bounds, integer arithmetic,
and division assumptions. `--opt` enables source and Wasm optimization and may
omit optional Calque guards. Wasm still bounds linear memory, but an unchecked
vector index is not thereby valid: a bad address that stays within linear
memory can corrupt another Calque runtime value. Validate in debug mode before
choosing optimized execution.

The repository can also pass the same Wasm through `wasm2c` and a C compiler
for a native executable. That is an alternative execution route, not a second
language or a different ownership model.

```text
source → macro expansion → inferred typed program → analysis + optimization
       → WAT → Wasm → WASI runtime
                      └→ wasm2c + C compiler (optional native route)
```

For commands and release policy, see [WebAssembly target](../compiler/webassembly.md)
and [Debug, optimized, and native execution](../compiler/release-safety.md).

## Keep this page as the map

When a program is surprising, follow this order:

1. Forms and macros establish source structure.
2. Inference assigns a coherent static shape.
3. Analysis proves visible invariants and reports unknown ones.
4. Ownership makes managed storage deterministic and rejects managed cycles.
5. Optimization and the Wasm backend turn the checked program into executable
   code.

The deeper chapters define each rule. This page explains why they belong to one
toolchain.
