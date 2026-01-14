Que Script is a **functional, expression-only, Lisp-style language** using **S-expressions**.

---

### Core

- Everything is an **expression**
- Expressions **always return a value**
- Lists are function calls: `(f a b)`
- The **last expression** is the result
- No statements

---

### Bindings & Scope

- `(let name value)` — immutable, returns **Unit (`0`)**
- Rebinding in the same scope is illegal
- `do` sequences expressions, returns the last
  (required for multi-expression lambdas)

```lisp
(do
  (let x 1)
  (let y 2)
  (+ x y))
```

---

### Functions

- `(lambda arg1 arg2 body)`
- Last is aways the body
- Called as `(f x y)`
- Use `do` for multi-step bodies

---

### Recursion

- `let~` — **tail recursion**, TCO enabled
- `let*` — explicit recursion, no TCO
- VM forbids returning closures capturing outer scope

---

### Control Flow

- `(if cond then else)`
- `(cond c1 e1 c2 e2 ... default)`
- `(unless cond expr nil)`
- `and` / `or` short-circuit

---

### Loops

- `(loop start end body)` — for-style
- `(loop condition body)` — while-style
- No `break`; use flags
- Returns **Unit (`0`)**

---

### Pipe

- `(|> x f g h)` → `(h (g (f x)))`
- Left → right, data last

---

### Types

- Primitives: `Int Float Bool Char`
- Composite:

  - `[T]` — Vector
  - `{A B}` — Tuple
  - `A -> B` — Function

- String = `[Char]`
- Tuples via `fst`, `snd`, or destructuring

---

### Mutation (Explicit)

Immutable by default.
Mutable state is boxed:

```lisp
(integer i 0)
(boolean b false)
(variable x v)
(get i) (++ i) (set i v)
```

Vectors: `push! set! pull!`

---

### Unit

- Side-effect expressions return **Unit**
- Runtime value: `0`
- `nil` == Unit

---

### Typesystem

- Hindley–Milner inference
- No annotations required

---

### Idioms

- Prefer `map filter reduce`
- Use `zip / unzip`
- Tuples as enums
- Convert non-tail recursion to loops or stacks
- Flags instead of `break`

---

### Useful Functions

```
map      (T -> T) -> [T] -> [T]
filter   (T -> Bool) -> [T] -> [T]
reduce   (T -> T -> T) -> T -> [T] -> T
range    Int -> Int -> [Int]
min/max  Int -> Int -> Int
sum/prod [Int] -> Int
every?/some? (T -> Bool) -> [T] -> Bool
zip {[T] * [T]} -> [{T * T}]
unzip [{T * T}] -> {[T] * [T]}
```

---

### Example Functions

```lisp
; let* defines recursive function
(let* fibonacci (lambda n
    (if (or (= n 0) (= n 1)) n
        (+ (fibonacci (- n 1)) (fibonacci (- n 2))))))

(fibonacci 23)
```
