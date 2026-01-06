# Getting Started

_Learn the basics of Que Script_

## Installation

If you just want to try Que, then there is no need to install it locally.

#### I recomend sticking to the online editor which has best support for the language at the moment

#### If you still want to install it - just go to github, clone the repo and build it.

## Your First Program

Let's write a simple 'Hello, World!' program. Press play to see the output!

```lisp
"Hello World"
```

## Editor

###

#### You can find the full on this [link](https://at-290690.github.io/rust-lisp/playground/)

---

| Button        | Icon | Description                                           | Use case                                            |
| ------------- | ---- | ----------------------------------------------------- | --------------------------------------------------- |
| Play          | \|>  | Type check and run code                               | To quckly type check and then run                   |
| Check         | T    | Type check only                                       | When you only want to check if program type checks. |
| JavaScript    | ⚡   | Run JavaScript compiled code                          | JS is quite fast because it's JIT compiled          |
| Sparkles Mode | ✨   | Sparkles appear when you type and run                 | No use case, but makes your code feel magical       |
| Unsafe Mode   | 🪲   | You can only run js code, cmd/control + s now runs js | Disable type checker and run JavaScript             |
| Documentaiton | 📖   | Opens a modal of the documentation                    | Search functions by signature and name              |
| Share         | 🔗   | Opens a modal for creating a link                     | To share your program and settings                  |

---

The last expression will aways be printed on the terminal with it's type information

```lisp
42
```

## Errors

Often times you will get errors and they will be printed in red.

```lisp
(+ 1 true)
```

# Basic Syntax

_Understand the fundamental syntax and structure of Que Script. New to Lisp? Don't worry, I start from the very basics._

## Comments

Comments in Que Script start with a semicolon (;) and continue until the end of the line. They are ignored by the interpreter.

```lisp
; This is a single-line comment
9 ; You can also add comments after code
; 10 ; This line won't run
```

## S-Expressions

Que Script uses S-expressions (symbolic expressions) as its primary syntactic structure.

#### Everything in Que Script is an expression that returns a value.

#### An S-expression is simply a list enclosed in parentheses, where the first element is the function or operator, and the remaining elements are the arguments.

---

| Element             | Description                                                                          | Example                          | Result/Meaning                   |
| ------------------- | ------------------------------------------------------------------------------------ | -------------------------------- | -------------------------------- |
| Literal             | A raw value — number, string, boolean, etc.                                          | 42                               | An integer literal               |
| Symbol              | A name referring to a variable or function                                           | x                                | Refers to the value of x         |
| List (S-Expression) | A list where the first element is a function or operator, and the rest are arguments | (+ 1 2 3)                        | Adds 1, 2, and 3 → 6             |
| Nested Expression   | S-expressions can appear inside others                                               | (\* 2 (+ 3 4))                   | 2 \* (3 + 4) → 14                |
| Variable Definition | Define a name using let                                                              | (let a 10)                       | Creates a binding a = 10         |
| Lambda Expression   | Defines an anonymous function                                                        | (lambda x (\* x x))              | A function that squares a number |
| Function Call       | Calls a function with arguments                                                      | ((lambda x (\* x x)) 5)          | Calls the lambda with 5 → 25     |
| Conditional         | All control flow is an expression                                                    | (if (> x 10) "big" "small")      | Returns "big" if x > 10          |
| Do Block            | Groups multiple expressions and returns the last                                     | (do (let x 2) (+ x 3))           | Returns 5                        |
| Pipe Expression     | Passes values through multiple functions                                             | (\|> [1 2 3] (map square) (sum)) | Returns 14                       |

---

```lisp
(+
  (+ 1 2 3) ; Returns: 6
  (* 2 (+ 3 4)) ; Returns: 14
) ; Returns: 20

; Here:
; (+ 1 2 3) means “add 1, 2, and 3.”
; (* 2 (+ 3 4)) first evaluates (+ 3 4) → 7, then multiplies 2 * 7.
; (+ (+ 1 2 3) (* 2 (+ 3 4))) the outer expression is (+ 6 14) → 20
; Everything — even if, let, and lambda — uses this same structure.
```

## Variables

Define variables using the 'let' keyword. Que Script supports immutable bindings by default.

#### 'let' returns the sentinel value 0.

#### To actually get the value defined you have to 'use' the variable name.

```lisp
(let x 10) ; 0 sentinel value
(let y 20) ; 0 sentinel value
(+ x y) ; Returns: 30
```

## Scopes

The environment where a variable is defined and can be accessed.

---

| Concept              | Syntax/Example                              | Description                                               | Result/Behavior    |
| -------------------- | ------------------------------------------- | --------------------------------------------------------- | ------------------ |
| Define Variable      | (let x 10)                                  | Creates a new variable x with value 10                    | x → 10             |
| Access Variable      | x                                           | Evaluates to the variable’s current value                 | Returns 10         |
| Multiple Variables   | (do (let a 5) (let b 7) (+ a b))            | Multiple variables can be defined in sequence using do    | Returns 12         |
| Immutable by Default | (let x 1) (let x 2)                         | Redefining the same name in the same scope is not allowed | Compile-time error |
| Variable in Function | (let square (lambda x (\* x x)))            | Functions can be stored as variable values                | (square 4) → 16    |
| Derived Variable     | (let y (+ 2 3))                             | Variable values can be computed from expressions          |
| Scoped Definition    | (let scope (lambda (do (let x 5) (+ x 2)))) | Variables exist only within their local function scope    | Returns 7          |

---

```lisp
(let x 8) ; It will not get used
(let y 2)
(let scope (lambda (do
        (let x 5) ; shadowing outer x
        (+ x y) ; capturing outer y
)))
(scope)
```

## Do

`do` in Que allows you to sequence multiple expressions and evaluate them in order, returning the value of the **last** expression. It is commonly used to introduce multiple steps or bindings inside a function, lambda, or any expression context where only a single expression would normally be allowed.

#### This allows Que Script to express data transformations without deep nesting of function calls.

#### The key rule is that the value of a `do` block is the value of its final expression. Everything before the last expression is executed for side effects or variable bindings, while the last expression determines what the block evaluates to.

```lisp
; global variable z
(let z 19)

(apply ; apply immediately invokes lambda
  (lambda ; lambda creates a scope
    (do ; do creates multiline expression
      (let x 10) ; define x on first line
      (let y 8) ; define y on second line
      (+ x y z)))) ; return value on last line
; result => 37
```

## Pipe

The pipe operator is used to chain expressions in a readable, left-to-right manner.

#### It takes the result of one expression and passes it as the first argument to the next function.

#### This allows Que Script to express data transformations without deep nesting of function calls.

#### The pipe is purely syntactic sugar — no runtime cost, just improved readability.

---

| Concept         | Explanation                                                               |
| --------------- | ------------------------------------------------------------------------- |
| Symbol          | \|>                                                                       |
| Type            | a -> (a -> b) -> b                                                        |
| Meaning         | “Take a value, and feed it into the next function.”                       |
| Associativity   | Left-to-right — expressions are evaluated from the left and passed along. |
| Return Value    | The final expression’s result.                                            |
| Syntactic Sugar | (\|> value f g h) -> desugar -> (h (g (f value)))                         |

---

```lisp
; ❌ Nobody wants to write this
(sum (map (lambda x (* x x)) (filter (lambda x (= (mod x 2) 1)) (range 1 10))))

; ✅ So much better
(|> (range 1 10)
    (filter (lambda x (= (mod x 2) 1)))
    (map (lambda x (* x x)))
    (sum))
```

## Aliases

Que provides aliases for many commonly used standard library functions. These aliases are simply shorter names that refer to fully qualified definitions in the standard library, making everyday code easier to read and write without sacrificing clarity or safety.

Aliases allow you to focus on _what_ a program does instead of _where_ each function comes from. In practice, this means you can write expressive pipelines using concise names, while still having access to the full, explicit names when precision matters.

The following example shows the same program written using short aliases and fully qualified names. Both versions are identical in behavior and type checking — the only difference is readability.

```lisp
; Short names
(|> (range 1 10) (map square) (map Integer->String) (Vector->String ','))
; Full names - but they use data first pipe (<|)
(<| (std/vector/int/range 1 10) (std/vector/map std/int/square) (std/vector/map std/convert/integer->string) (std/convert/vector->string ','))
```

Aliases are resolved automatically and do not introduce name clashes. If you define a local function with the same name as an alias, your local definition always takes precedence. This means you are free to choose meaningful names in your own code without worrying about conflicts with the standard library.

When ambiguity matters, you can always fall back to the fully qualified name. This gives Que a clean balance: short names for ergonomics, long names for precision, and no import statements or namespace management required.

This design keeps the language minimal while still scaling to large programs. Small scripts remain concise, while larger codebases can opt into explicitness exactly where it helps readability and maintenance.

# Functions

_Functions are first-class citizens in Que Script. They let you group logic, reuse code, and express computation elegantly. Every function in Que Script is an expression that returns a value._

## Lambda Functions

Create anonymous functions using the 'lambda' keyword. These functions can be passed around as values.

#### Create anonymous functions using the 'lambda' keyword. These functions can be passed around as values.

#### Lambdas (also known as abstractions) define parameters followed by a body expression or a (do) block.

---

| Concept                                                     | Description                                                                                                                    | Example                                   |
| ----------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------ | ----------------------------------------- |
| Definition                                                  | A lambda defines an anonymous function (a function without a name). It’s created using the lambda keyword.                     | (lambda x (\* x x))                       |
| Syntax                                                      | (lambda arg1 arg2 ... body)                                                                                                    |
| The parameters come first, followed by the body expression. | (lambda a b (+ a b))                                                                                                           |
| Body                                                        | The body is a single expression or a (do) block containing multiple expressions. The value of the last expression is returned. | (lambda x (do (print x) (\* x x)))        |
| Return Value                                                | Always the result of the last expression in the body.                                                                          | (lambda x (\* x x)) => x²                 |
| Type Signature                                              | Que Script infers types automatically. Example: (lambda x (\* x x)) → Int -> Int                                               | (lambda a b (+ a b)) => Int -> Int -> Int |
| Invocation                                                  | Call a lambda by wrapping it in parentheses with its arguments.                                                                | ((lambda x (\* x x)) 4) => 16             |
| As a Value                                                  | Lambdas are first-class values — they can be stored in variables, passed to functions, or returned (mostly).                   | (let square (lambda x (\* x x)))          |
| Also Known As                                               | Abstractions — functions defined directly without a name.                                                                      | —                                         |

---

```lisp
(lambda x (* x x)) ; A simple function that squares a number
; The type signature is Int -> Int
```

## Named Functions

Define reusable functions by binding lambdas to names using 'let'.

```lisp
; lambda is a value that can be assigned to a variable
(let power (lambda x (* x x)))
; you can only pass up ot 1 argument to this function
(power 5) ; Apply 5
; 25
```

## Functions with Multiple Arguments

Functions can take more than one argument — just list them all after lambda.

```lisp
(let add (lambda a b (+ a b)))
; The signature of this function is
add ;  Int -> Int -> Int
(add 5 7)
; Returns: 12
```

## Functions with Multiple Expressions

To include multiple steps in a function body, wrap them in a do block. Only the last expression in the block is returned.

```lisp
(let greet (lambda name (do
    (let message (cons "Hello, " (cons name "!")))
    message)))

(greet "Que")
; Returns: "Hello, Que!"
```

## Higher-Order Functions

Functions that take other functions as arguments or return functions are powerful tools of the functional programmer. In Que Script, functions are first-class values — you can pass them as arguments, return (sometimes) them from other functions, and store them in variables.

```lisp
(let apply-twice (lambda f x
    (f (f x))))

(let double (lambda n (* n 2)))

(apply-twice double 5)
; Returns: 20

; Here:
; apply-twice takes a function f and a value x.
; It applies f to x twice → f(f(x)).
```

## Anonymous Functions in Pipelines

You can use anonymous lambdas inline with the pipe operator (|>), especially for quick filtering or mapping.

```lisp
(|> [1 2 3 4 5]
    (filter (lambda x (> x 2)))
    (map (lambda x (* x x))))
; Returns: [9 16 25]
```

## Recursion

Functions in Que Script can call themselves and thanks to Tail Call Optimization, recursion is efficient and safe.

```lisp
(let~ factorial (lambda n total
    (if (= n 0)
        total
        (factorial (- n 1) (* total n)))))

(factorial 5 1)
; Returns: 120
```

## Memoization

Memoization is an optimization technique used to cache the results of expensive function calls so future calls with the same input can reuse previously computed values. In Que, memoization pairs naturally with pure functions, because identical inputs are guaranteed to produce identical outputs. This makes caching both safe and predictable.

```lisp
(let* fibonacci (lambda n (if (< n 2) n (+ (fibonacci (- n 1)) (fibonacci (- n 2))))))
(fibonacci 10)
```

Without memoization, recursive functions like Fibonacci recompute the same values many times, resulting in exponential runtime. Memoization transforms this behavior by storing intermediate results in a table so they can be reused instead of recomputed.

```lisp
(let* fibonacci (lambda n memo
    (do
    (let key (Integer->String n))
    (if (< n 2) n (if (Table/has? memo key) (snd (first (Table/get memo key))) (do
      (let res (+ (fibonacci (- n 1) memo) (fibonacci (- n 2) memo)))
      (Table/set! memo key res)
      res))))))

(fibonacci 10 [[] [] [] [] [] [] []])
```

Memoization is especially effective in Que because purity ensures that cached results never become invalid. It is not a replacement for algorithms, but it turns expressive—but slow—recursive definitions into practical ones. The key idea: pure functions plus memoization enable efficient computation without sacrificing clarity.

## Explicit recursion (let\* & let~)

If a function is a tail call, Que rewrites it internally as a loop for efficiency but only if explicitly defined as let~.

```lisp
; Explicit tail call can be optimized
(let~ tail-call/sum (lambda xs acc
    (if (= (length xs) 0) acc
        (tail-call/sum (drop/first 1 xs) (+ acc (get xs 0))))))

; Explicit recursion can't be optimized
(let* non-tail-call/sum  (lambda xs
    (if (empty? xs) 0 (+ (car xs)
        (non-tail-call/sum (cdr xs))))))

[
  (tail-call/sum [ 1 2 3 4 5 ] 0)
  (non-tail-call/sum  [ 1 2 3 4 5 ])
] ; both produce 15
```

You need to explicitly mark a function as recursive using `let*`. This is especially useful for functions that are deeply or mutually recursive, such as the Ackermann function. Using `let*` tells the compiler that recursion is intentional and not tail-call optimizable — preventing TCO and ensuring the recursive structure remains untouched.

```lisp
(let* ackermann (lambda m n
    (cond
        (and (< m 0) (< n 0)) -1
        (= m 0) (+ n 1)
        (and (> m 0) (= n 0)) (ackermann (- m 1) 1)
        (and (> m 0) (> n 0)) (ackermann (- m 1) (ackermann m (- n 1)))
        0)))

(ackermann 2 3)
```

However, note that `let*` disables Tail Call Optimization — which means you should only use it when you are absolutely sure. For tail-recursive functions, prefer `let~` so the compiler can optimize them into loops.

## Partial Application

Functions can be partially applied — you can call them with fewer arguments to create a new function.

```lisp
(let subtract (lambda a b (- a b)))
(let subtract-10 (subtract 10))

(subtract-10 3)
; Returns: 7
```

## Partial Application in the real world

Partial application lets you create specialized functions by fixing some arguments of an existing one.

#### The result is a new function that remembers those fixed arguments. This gives us:

#### 1. Stateful Behavior in Pure Syntax - Although written functionally, Que Script handles state safely through controlled mutation (set!, get).

#### 2. Functional Reuse - You can reuse the same buffer/push! logic to create multiple independent ring buffers

---

| Step                | Operation                                                                                                               | Buffer State                                       |
| ------------------- | ----------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------- |
| Partial Application | Freezes part of the argument list of a function, returning a new callable that fills in the remaining parameters later. | []                                                 |
| Mutable References  | The pointer ([0]) acts as a mutable “box” storing the current position.                                                 | []                                                 |
| loop 0              | Push 0                                                                                                                  | [0]                                                |
| loop 1              | Push 1                                                                                                                  | [0, 1]                                             |
| loop 2              | Push 2                                                                                                                  | [0, 1, 2]                                          |
| loop 3              | Push 3                                                                                                                  | [0, 1, 2, 3]                                       |
| loop 4              | Push 4                                                                                                                  | [0, 1, 2, 3, 4]                                    |
| loop 5              | Push 5                                                                                                                  | [5, 1, 2, 3, 4] (overwrites index 0 — ring effect) |

---

```lisp
; create a ring buffer with many arguments
(let buffer/push! (lambda buffer pointer len item (do
        (let pt (get pointer))
        (set! buffer pt item)
        (set pointer (mod (+ len pt 1) len))
        item)))

; init our buffer empty
(let buffer [])
; create a custom function that partly applies
; - buffer
; - pointer state
; - capacity

(let my-buffer/push! (buffer/push! buffer [ 0 ] 5))
; now we have a simple ring buffer push!
; that takes onlt 1 argument
(loop 0 6 (lambda i (my-buffer/push! i)))
buffer ; Returns [5 1 2 3 4]
```

## Zip and Unzip

Tuples become especially powerful when combined with `zip` and `unzip`. These operations allow you to merge two vectors into a vector of tuples — and later split them back into separate vectors. This is a common pattern in functional programming for pairing related data together while keeping everything type-safe and pure.

```lisp
(let nums (range 0 5))
(let bools (|> nums (map (lambda x (= (mod x 2) 0)))))

(zip { nums bools })
; => [[0 true] [1 false] [2 true] [3 false] [4 true]]
```

`unzip` performs the reverse operation: it takes a vector of tuples and separates them into two individual vectors. Together, `zip` and `unzip` make it easy to structure and transform parallel data without losing relationships.

```lisp
(let tups (|> (range 0 5)
(map (lambda x { x (= (mod x 2) 0) } ))))
(unzip tups)
; => [[0 1 2 3 4] [true false true false true]]
```

Because tuples in Que are strongly typed, `zip` and `unzip` automatically infer the correct types for both vectors. This makes data pairing simple, expressive, and memory-safe — without any type annotations.

## Where 'Functions are first-class citizens' breaks

Here I will attempt to address something about the VM which I think you should know even though you will probably never encounter this issue.

#### While higher-order functions (functions that take or return other functions) are supported in the JavaScript compiler, the VM implementation takes a more constrained approach to ensure memory safety and predictability.

#### Que Script was designed to run in two environments:

#### - Native VM — designed for embedding, low memory usage, and deterministic performance using reference counting instead of a garbage collector.

#### - JavaScript backed by a garbage collector, capable of safely managing closures and returned functions

#### Because of this, returning functions (closures) from other functions can create memory leaks in the VM:

#### - Closures may capture outer environments that never get released.

#### - Reference cycles cannot be broken by reference counting alone.

#### - Memory gradually accumulates with each returned lambda.

#### For the sake memory safety the VM will prevents closure retention that could create circular references.

#### The VM version deterministic and safe, while the JS version remains fully functional.

---

| Environment | Returning Lambdas | Memory Model       | Status          |
| ----------- | ----------------- | ------------------ | --------------- |
| JavaScript  | ✅ Allowed        | Garbage Collector  | Safe            |
| VM (Native) | ⚠️ Restricted     | Reference Counting | Potential leaks |

---

```lisp
; This function returns another function that closes over 'y'
(let fn-a (lambda x y
            (lambda z
                (+ z y)))) ; Captures 'y' — potential memory leak

(let f-a (fn-a 1 2))
; If you uncomment this it will cause an error
; in VM at run time (clicking the green play button)
; undefined variable y
; (f-a 8)

; this will be fine
(let fn-b (lambda x y z (+ z y)))
(let f-b (fn-b 1 2))
(f-b 8) ; this will be fine

; both will work with the (clicking the yellow zap button which does js run)
```

# Data Types

_Que Script keeps things simple — it only has four core data types, but they’re flexible and powerful._

## Core Types

Que Script’s type system is designed to be simple, sound, and statically inferred using Hindley–Milner (HM) inference. There are five fundamental types in Que Script, but the three most common primitives are:

#### Bool — truth values

#### Int — integer numbers

#### Float — floating point numbers

#### Char — individual characters

#### Each of these types is atomic (not composed of other types) and has strict, well-defined operations.

---

| Type  | Description                  | Example | Notes                              |
| ----- | ---------------------------- | ------- | ---------------------------------- |
| Bool  | Logical truth value          | true    | Used in conditions and logical ops |
| Int   | 32-bit integer               | 42      | Arithmetic, indexing, counting     |
| Float | 32-bit floating point number | 42      | Arithmetic                         |
| Char  | Single character             | 'a'     | Building blocks of strings         |

---

## Integers (Int)

Integers are the most common numeric type in Que Script. You can use standard arithmetic operations: +, -, \*, /, and mod

#### Represents signed 32-bit integers (whole numbers).

#### Used for all numeric arithmetic, counting, and indexing operations.

---

| Operator | Description      | Example    | Result |
| -------- | ---------------- | ---------- | ------ |
| +        | Addition         | (+ 2 3)    | 5      |
| -        | Subtraction      | (- 10 4)   | 6      |
| \*       | Multiplication   | (\* 3 4)   | 12     |
| /        | Integer division | (/ 9 2)    | 4      |
| mod      | Modulus          | (mod 10 3) | 1      |
| abs      | Absolute value   | (abs -8)   | 8      |
| =        | Equality         | (= 5 5)    | true   |
| >        | Greater than     | (> 3 1)    | true   |
| <        | Less than        | (< 2 5)    | true   |
| >=       | Greater or equal | (>= 10 10) | true   |
| <=       | Less or equal    | (<= 7 8)   | true   |

---

```lisp
0
42
-7
123456
(- 1) ; -1
(+ 10 5) ; Returns: 15
(- 20 7) ; Returns: 13
(mod 10 3) ; Returns: 1
```

## Floating point numbers (Float)

Floats are the fractional numbers in Que Script. You can use special arithmetic operations: +., -., \*., /., and mod.

#### Represents signed 32-bit floats (fractional numbers).

#### Used for all numeric arithmetic, counting, and indexing operations.

---

| Operator | Description          | Example             | Result     |
| -------- | -------------------- | ------------------- | ---------- |
| +.       | Float Addition       | (+. 2.0 3.0)        | 5.0        |
| -.       | Float Subtraction    | (-. 10.2 4.1)       | 6.1        |
| \*.      | Float Multiplication | (\*. 0.5 2.0)       | 1.0        |
| /.       | Float division       | (/. 9.0 2.0)        | 4.5        |
| mod.     | Float Modulus        | (mod. 10.2 3.3)     | 0.29999995 |
| =.       | Float Equality       | (=. 5.0 5.0)        | true       |
| >.       | Greater than         | (>. 3.1 3.0)        | true       |
| <.       | Less than            | (<. 2.10 2.11)      | true       |
| >=.      | Greater or equal     | (>=. 10.123 10.123) | true       |
| <=.      | Less or equal        | (<=. 3.14 4.20)     | true       |

---

```lisp
0.0
42.0
-7.12
3.14
3.1415927
(-. 1.23) ; -1.23
(+. 2.5 0.5) ; Returns: 3.0
(-. 20.0 0.6) ; Returns: 19.4
```

## Booleans (Bool)

Booleans represent truth values. You can compare numbers and use logical operators like and, or, and not.

#### Literals:

#### - true

#### - false

#### Bool values are the result of comparisons or logical operations.

#### Can be used in conditionals such as if expressions.

#### There are no truthy or falsy conversions — only Bool values are valid in boolean contexts.

---

| Operator | Description         | Example          | Result |
| -------- | ------------------- | ---------------- | ------ |
| not      | Logical negation    | (not true)       | false  |
| and      | Logical conjunction | (and true false) | false  |
| or       | Logical disjunction | (or false true)  | true   |
| =?       | Equality comparison | (=? true true)   | true   |

---

```lisp
(> 5 2) ; true

(= 10 10) ; true

(and true false) ; false

(or false true) ; true

(not false) ; true
```

## Characters (Char)

Characters represent single symbols or letters. They’re often used for parsing or text-based problems.

#### A Char is not a string — it’s a single symbol.

#### Strings in Que Script are represented as vectors of characters ([Char]).

#### Useful for parsing text, counting symbols, and solving character-based puzzles

---

|     | Description             | Example       | Result |
| --- | ----------------------- | ------------- | ------ |
| +#  | Sum of two chars        | (+# 'a' 'B')  | '£'    |
| \*# | Product of two chars    | (\*# 'a' 'B') | 'ᤂ'    |
| -#  | Difference of two chars | (-# 'a' ' ')  | 'A'    |
| =#  | Compare two chars       | (=# 'a' 'a')  | true   |
| >#  | Compare two chars       | (># 'a' 'b')  | false  |
| <#  | Compare two chars       | (<# 'a' 'b')  | true   |
| >=# | Compare two chars       | (># 'a' 'b')  | false  |
| <=# | Compare two chars       | (<=# 'a' 'a') | true   |
| /#  | Quotient of two chars   | (/# 'a' ' ')  | ''     |

---

```lisp
(let c '(')
; A character literal
c
; Returns: '('
```

## Vectors ([T])

Vectors are ordered collections (similar to arrays or lists). You can use special ! operations (like push! or sort!) for mutable behavior.

```lisp
(let nums [1 2 3 4 5])
nums
; Returns: [1 2 3 4 5]

(first nums)
; Returns: 1

(last nums)
; Returns: 5

(reverse nums)
; Returns: [5 4 3 2 1]
```

## Tuples ({T1 \* T2})

Tuples in Que are small, fixed-size containers that hold exactly two values — potentially of different types. They’re immutable and type-safe, making them ideal for representing pairs such as coordinates, key–value pairs, or flags with data. You can access elements using `fst` and `snd`, or destructure them directly in function parameters.

```lisp
{ true 10 } ; -> tuple { Bool * Int } {true 2
(fst { true 10 }) ; -> Bool
(snd { true 10 }) ; -> Int
```

Tuples are written using curly braces `{ }`. `{ true 10 }` is shorthand for `(tuple true 10)` and represents a value of type `{ Bool * Int }`. You can extract the first and second elements using `fst` and `snd`, or destructure them in lambdas.

```lisp
(let show (lambda { a b } { a b }))
(show { "x" 42 }) ; => { "x" 42 }   ; type { Char * Int }
(fst (show { "x" 42 })) ; => "x"
(snd (show { "x" 42 })) ; => 42
```

Tuples also integrate smoothly with vectors. You can zip two vectors together into a vector of tuples using `std/vector/tuple/zip`, and later separate them with `std/vector/tuple/unzip`.

```lisp
(zip { [1 2 3] [true false true] })
; => [[1 true] [2 false] [3 true]]

(unzip (zip { [1 2 3] [true false true] }))
; => [[1 2 3] [true false true]]
```

You can also use tuple destructuring with skipped values. Use `.` to ignore elements you don’t need.

```lisp
(let second (lambda { . b } b))
(second { true 5 }) ; => 5
```

## Strings ([Char])

They are just Vectors of Chars. You can also count or filter specific characters in strings or vectors:

```lisp
(let hello "Hello")
(let world [ 'W' 'o' 'r' 'l' 'd' '!' ])
(cons hello " " world)
```

## Functions (T to T)

Functions are first-class values — they can be assigned, passed, or returned (sometimes).

#### In Que Script, every function has a type signature that describes what types it takes as input, and what type it returns as output.

#### Function types are written using the arrow notation (meaning 'to')

#### When a function takes multiple parameters, its type expands right-associatively.

#### Int to Int to Int

#### is equivalent to

#### Int to (Int to Int)

#### You can read it as a function that takes an Int, and returns another function that takes an Int and returns an Int.

---

| Type                       | Read As                                               | Example Function           |
| -------------------------- | ----------------------------------------------------- | -------------------------- |
| Int -> Int                 | Takes an Int, returns an Int                          | (lambda x (\* x x))        |
| Int -> Bool                | Takes an Int, returns a Bool                          | (lambda n (= (mod n 2) 0)) |
| (Int -> Int) -> Int -> Int | Takes a function and an Int, returns an Int           | apply-twice                |
| t -> t                     | Takes a value of any type and returns the same type   | identity                   |
| [t] -> (t -> Bool) -> [t]  | Takes a list and a predicate, returns a filtered list | filter                     |

---

```lisp
(let add1 (lambda x (+ x 1)))
(map add1 [1 2 3])
; Returns: [2 3 4]
```

## Unit ()

The Unit type, written as (), represents the absence of a meaningful value. It’s used for expressions that perform side effects rather than return data.

#### In Que Script, every expression returns something — even actions like mutation, printing, definitions or looping.

#### When an operation’s result isn’t a value you can use, it returns Unit.

#### Represents a computation with no useful return value.

#### They return the Unit type but still the value at runtime is the 0 sentinel value

---

| Function    | Type Signature        | Description                               | Returns    |
| ----------- | --------------------- | ----------------------------------------- | ---------- |
| let         | T -> ()               | Binds a variable                          | 0 Sentinel |
| loop        | (… -> ())             | Executes repeated actions                 | 0 Sentinel |
| push!       | [T] -> T -> ()        | Pushes a value into a vector              | 0 Sentinel |
| pull!       | [T] -> ()             | Pops a value and discards it              | 0 Sentinel |
| pop!        | [T] -> ()             | Pops a value (returns ())                 | 0 Sentinel |
| set!        | [T] -> Int -> T -> () | Mutates an element of a vector            | 0 Sentinel |
| +=          | [Int] -> Int -> ()    | In-place addition on vector element       | 0 Sentinel |
| -=          | [Int] -> Int -> ()    | In-place subtraction on vector element    | 0 Sentinel |
| \*=         | [Int] -> Int -> ()    | In-place multiplication on vector element | 0 Sentinel |
| /=          | [Int] -> Int -> ()    | In-place division on vector element       | 0 Sentinel |
| ++          | [Int] -> ()           | Increments integer in place               | 0 Sentinel |
| --          | [Int] -> ()           | Decrements integer in place               | 0 Sentinel |
| \*\*        | [Int] -> ()           | Squares integer in place                  | 0 Sentinel |
| boolean/set | [Bool] -> Bool -> ()  | Sets boolean value in place               | 0 Sentinel |
| nil         | ()                    | Represents the Unit value itself          | 0 Sentinel |
| null        | () -> ()              | Returns Unit explicitly                   | 0 Sentinel |

---

```lisp
(let xs [ 1 2 3 4 ])
(push! xs 10) ; Type () Returns 0
```

## Type Inference

You don't need to declare types manually — Que Script uses Hindley–Milner type inference. This means the compiler automatically figures out types safely

---

| Concept           | Example                      | Explanation                          |
| ----------------- | ---------------------------- | ------------------------------------ |
| Primitive Types   | Int, Float, Bool, Char       | Cannot be broken down further        |
| Composite Types   | [Int], [Char], (Int -> Bool) | Built from primitives                |
| Polymorphic Types | [t], (t -> t)                | Generic types inferred automatically |

---

```lisp
; Here, the compiler infers:
; xs → [Int]
; return type → Int

(let summation (lambda xs (reduce + 0 xs)))
(summation [1 2 3 4])
; Int
; 10
```

## Casting with 'as'

#### Que’s type inference is powerful, but there are situations where the programmer would prefer to explicitly state the type. In those cases, the `as` construct allows you to explicitly assert a type and guide the type checker.

#### `as` does not convert values at runtime. Instead, it _annotates_ a value with a specific type, telling the compiler how it should be treated during type checking. This is especially useful when working with empty vectors, nested structures, or polymorphic code where inference has insufficient context.

#### A common use case is initializing empty collections. An empty vector `[]` has no elements, so the compiler cannot infer what its element type should be. By casting it, you make the intended type explicit from the start.

```lisp
(let xs (as [] [Int]))
xs
```

#### Casting is also useful for complex nested types. In the example below, the cast fixes the vector’s element type to `[Bool]`, preventing accidental insertion of values with the wrong type.

```lisp
(let xs (as [[]] [[Bool]]))
(push! xs [false])
(push! xs [1]) ; ❌ Cannot unify Int with Bool
```

`as` should be used sparingly. Que’s inference will infer the correct type automatically. Casting is best reserved for cases where where clarity is more important than brevity.

## Type Assertions

#### Type assertions allow you to verify that the type inferred by the compiler matches your expectation.

#### Assertions do not override type inference. They simply check that the inferred type unifies with the expected type.

#### They can be applied after defining a variable, or inline using (: ...), though inline usage is not recommended except for advanced scenarios.

#### Type assertions are especially useful for empty collections, complex return types, and higher-order or recursive functions.

```lisp
(let fn (:
  (Lambda (: Int a) (: Int b) Bool) ; Returns Bool
  (lambda a b (+ a b)))) ; Returns Int

(fn 1 2)
; Cannot unify Bool with Int
; (: (lambda a b (do (: Int a) (: Int b) Bool)) (lambda a b (+ a b)))
```

Use Cases for Type Assertions:

- Guarding empty or initially ambiguous values, like empty arrays, to prevent inference ambiguity.

- Documenting complex return types, especially when functions return multiple values or nested structures.

- Validating higher-order functions and recursive functions to ensure arguments and return types behave as intended.

- Acting as a confidence check or contract at API boundaries.

Important Notes

- Type assertions do not influence inference — they only validate it.

- Assertions must come before a definition if used outside (: ...).

- Incorrect assertions fail compilation immediately.

- Overusing inline assertions with (: ...) is possible but generally discouraged to keep code clean.\_\_

## Named Function Types

Named function types allow you to define reusable, explicit function type signatures.

The type keyword lets you name a function type once and reuse it across multiple definitions.
This is especially useful for complex function signatures, multi-value returns, and higher-order functions.
Named types act as contracts: they do not change inference, but they validate that a function matches the declared shape.

```lisp
(type T::fn (Lambda (: Int a) (: Int b) (: Bool c) { Int Bool }))
(let fn (: T::fn (lambda a b c { (+ a b ) (if c (> a b) (= a b))})))
```

Using Named Types for Parameters and Returns

Named function types are first-class and can appear anywhere a type is expected.

This includes function parameters, return types, and higher-order functions.

This makes higher-order code significantly more readable by removing large inline type annotations.

# Control Flow

_Control the flow of execution in your programs with conditionals and loops._

```lisp
(type T::Int/Predicate? (Lambda (: Int x) Bool))

(let odd-n? (: T::Int/Predicate? (lambda n (= (mod n 2) 1))))
(let even-n? (: T::Int/Predicate? (lambda n (= (mod n 2) 0))))
(let zero-n? (: T::Int/Predicate? (lambda n (= n 0))))
(let one-n? (: T::Int/Predicate? (lambda n (= n 1))))
(let inverted-n? (: T::Int/Predicate? (lambda n (= n -1))))

; Can only contain T::Int/Predicate?
(let predicates (: [T::Int/Predicate?] [odd-n? even-n? zero-n? one-n? inverted-n?]))
```

Using type signatures for functions ::

Syntactic shorthand for type definition and type assertion.

By using the same name as a function in the current scope, we can reduce the type declaration to only one line of code.

```lisp
(type String [Char])
(:: common-strings (Lambda (: [String] a) (: [String] b) Int))
(let common-strings (lambda xs ys (|>
(Set/intersection (Vector->Set xs) (Vector->Set ys))
Set/size)))
```

What This Enables

Clear and reusable higher-order function signatures.

Consistent typing across APIs without repeating complex type expressions.

Safer refactoring, since mismatched implementations are caught immediately.

Notes

Named function types do not change runtime behavior — they are compile-time checks only.

They do not force types; they verify that inferred types match expectations.

They compose naturally with Hindley–Milner inference and generalisation.

## Conditional Expressions

Use 'if' expressions to make decisions based on conditions. All branches must return values.

#### Both branches must return compatible types so that the compiler can infer a single return type.

#### If only one branch is used then the return will be Unit

---

| Part      | Description                                               |
| --------- | --------------------------------------------------------- |
| condition | A Boolean expression that must evaluate to true or false. |
| then      | Expression evaluated if condition is true.                |
| else      | Expression evaluated if condition is false.               |

---

```lisp
(if true 1 0) ; Returns 1

(let absolute (lambda n
  (if (< n 0)
      (- n)
      n)))
(absolute -5) ; Returns: 5
```

It can be used with one or two branches

---

| Form                     | Description                                                              | Type |
| ------------------------ | ------------------------------------------------------------------------ | ---- |
| (if condition then)      | Evaluates condition. If true, returns then, otherwise returns 0 Sentinel | Unit |
| (if condition then! nil) | Evaluates condition. If true, do somethig, otherwise returns 0 Sentinel  | Unit |
| (if condition then else) | Evaluates condition. If true, returns then, otherwise returns else       | T    |

---

```lisp
(let xs [])
(if true (do (push! xs 10) nil)) ; Type Unit, Returns 0 Sentinel
xs ; Returns [10]
```

## Loops

The loop construct in Que Script provides iteration over numeric ranges or conditional repetition. It comes in two forms:

#### - For-loop style — (loop start end body)

#### - While-loop style — (loop condition body)

#### Both return the Unit type

---

| Form                  | Purpose                         | Example                          | Notes                       |
| --------------------- | ------------------------------- | -------------------------------- | --------------------------- |
| (loop start end body) | Iterates over numeric range     | (loop 0 5 (lambda i (print i)))  | Like a traditional for loop |
| (loop condition body) | Repeats while condition is true | (loop (< n 5) (lambda (do ...))) | Like a while loop           |

---

```lisp
(let out [])
; for loop
(loop 0 5 (lambda i (push! out i)))
; while loop
(integer counter 0)
(loop (< (get counter) 5)
  (lambda (do
    (push! out (get counter))
    (++ counter))))
out ; Returns [0 1 2 3 4 0 1 2 3 4]
```

## Breaking out of a loop

In short - there is no way to break out of a loop

#### Some languages or loop constructs lack a built-in break. Use a placement flag to emulate breaking: set the flag when you want to stop further work for the current iteration and ensure subsequent loop iterations skip actions based on that flag.

#### Pattern: initialize a boolean (e.g., placed = false) before scanning; inside the loop, when the condition to stop is met set placed = true; in later iterations guard actions with (not placed). After the loop, check the flag to decide post-loop behavior (e.g., push a new segment only if placed is false).

---

| Form                  | Purpose                                 | Example                                                                                                                                            | Notes                                                                                                   |
| --------------------- | --------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------- |
| (loop start end body) | Iterates over numeric range             | (loop 0 5 (lambda i (do ...)))                                                                                                                     | Like a traditional for loop                                                                             |
| (loop condition body) | Repeats while condition is true         | (loop (< n 5) (lambda (do ...)))                                                                                                                   | Like a while loop                                                                                       |
| Flag pattern          | Emulate break when you can't exit early | Before loop: (boolean placed false) -- Inside loop: (when condition (boolean/set placed true)) -- After loop: (when (false? placed) (do-fallback)) | Use when break/return aren't available ensures a single action happens and prevents repeated placements |

---

```lisp
(let INPUT "58:5,3,7,8,9,10,4,5,7,8,8")
(let parse (lambda input (do
  (let parts (|> input (String->Vector ':')))
  (let head (|> (car parts) (Chars->Integer)))
  (let tail (|> (cdr parts) (car) (String->Vector ',') (map Chars->Integer)))
  { head tail })))

;  5   3-5   3-5-7   3-5-7   3-5-7   3-5-7   3-5-7   3-5-7   3-5-7   3-5-7   3-5-7 
;                      |       |       |       |       |       |       |       |
;                      8       8-9     8-9   4-8-9   4-8-9   4-8-9   4-8-9   4-8-9
;                                      |       |       |       |       |       |
;                                      10      10    5-10    5-10    5-10    5-10
;                                                              |       |       |
;                                                              7       7-8     7-8
;                                                                              |
;                                                                              8
; Check all segments of the spine, starting from the top.
; If your number is less than the one on the spine segment
; and the left side of the segment is free - place it on the left.
; If your number is greater than the one on the spine segment
; and the right side of the segment is free - place it on the right.
; If no suitable place is found at any segment,
; create a new spine segment from your number and place it as the last one.

(let part1 (lambda { . nums } (do
  (let sword [[-1 (get nums 0) -1]])
  (loop 1 (length nums) (lambda i (do
    (let num (get nums i))
    (boolean placed false)
    (integer J 0)
    (loop (and (false? placed) (< (get J) (length sword))) (lambda (do
      (let j (get J))
      (++ J)
      (let segment (get sword j))
      (cond
          (and (false? placed) (< num (get segment 1)) (= (get segment 0) -1))
                (do (set! segment 0 num) (boolean/set placed true))
          (and (false? placed) (> num (get segment 1)) (= (get segment 2) -1))
                (do (set! segment 2 num) (boolean/set placed true))
          nil))))
      (if (false? placed) (push! sword [-1 num -1])))))
  (|> sword (map (lambda [. x .] (std/convert/integer->string x))) (flat)))))
(part1 (parse INPUT))
```

## Boolean Short-Circuiting

Boolean expressions in Que use short-circuit evaluation. This means that `and` and `or` only evaluate the expressions they need to. If the result can be determined from the left-hand side alone, the right-hand side is never executed. This allows conditional logic to be both fast and safe, especially when the right side performs a computation or triggers a side effect.

```lisp
(let expensive-computation (lambda (do (loop 0 1000000000 (lambda . nil)) false)))

(and false (expensive-computation)) ; never calls expensive-computation
```

Short-circuiting isn't just an optimization — it's a semantic guarantee. Expressions like `(and x y)` and `(or x y)` preserve referential transparency when functions are pure, and they prevent unnecessary work. Critically, they also guard against errors: `(and (not (empty? xs)) (car xs))` is safe, because if `xs` is empty, the second operand is never evaluated.

```lisp
(let dangerous-operation (lambda (car [])))

(or true (dangerous-operation)) ; dangerous-operation is skipped
```

The table below illustrates how short-circuiting works step-by-step. Only the minimum number of evaluations occur to determine the result.

---

| Expression    | Left Evaluated? | Right Evaluated? | Result |
| ------------- | --------------- | ---------------- | ------ |
| (or true x)   | Yes             | No               | true   |
| (or false x)  | Yes             | Yes              | x      |
| (and false x) | Yes             | No               | false  |
| (and true x)  | Yes             | Yes              | x      |

---

## Cond Matching

The cond expression is Que Script’s way to handle multiple conditional branches elegantly.

#### It's a syntactic suggar for nested if expressions

#### It evaluates each condition in order and executes the expression associated with the first true condition.

```lisp
(let x 2)

(cond
  (= x 1) "one"
  (= x 2) "two"
  (= x 3) "three"
  "unknown")
; Returns: "two"

; same as writting
; (if (= x 1)
;     "one"
;     (if (= x 2)
;         "two"
;         (if (= x 3)
;             "three"
;             "unknown")))
```

## Unless

Rarerly you might need 'unless' which is the logical opposite of if

#### It's a syntactic suggar for reversed if expressions

#### It executes its body only if the condition is false.

#### This makes code more readable when you want to run something unless a condition is true.

---

| Construct | Executes When      | Example                                 | Behavior  |
| --------- | ------------------ | --------------------------------------- | --------- |
| if        | Condition is true  | (if (> x 10) (set out "big") nil)       | if x > 10 |
| unless    | Condition is false | (unless (> x 10) (set out "small") nil) | if x ≤ 10 |

---

```lisp
(variable out "")
(let x 9)

(unless (> x 10) (set out "small") nil)
(get out) ; "small"
```

# Mutation

_Immutability is a core idea in functional programming: it makes reasoning, testing, and concurrency simpler by keeping data unchanged. In real apps you still must manage state, so practical systems pair immutable data with controlled, explicit mutation — for example, by isolating state changes behind well-defined APIs, using copies or snapshots, or confining mutations to small, tested modules. The result: the clarity and safety of immutability where it matters, with deliberate, auditable mutation where it’s necessary._

## Mutable Scalars

By default, all let bindings in Que Script are immutable — once defined, their value cannot be changed.

#### However, Que Script also provides mutable scalar bindings for working with stateful values like counters, flags, and accumulators.

#### Mutable scalars are declared using typed scalar constructors such as integer, boolean, or variable.

#### Each of these creates a boxed variable — a container that holds a single value and can be modified in place.

---

| Form                  | Type    | Description                                 |
| --------------------- | ------- | ------------------------------------------- |
| (integer name value)  | [Int]   | Declares a mutable integer variable.        |
| (floating name value) | [Float] | Declares a mutable floating point variable. |
| (boolean name value)  | [Bool]  | Declares a mutable boolean variable.        |
| (variable name value) | [T]     | Declares a mutable variable of any type.    |

---

```lisp
(integer counter 0)
(++ counter)
(++ counter)
(get counter) ; Returns: 2

(floating fraction 0.0)
(+=. fraction 0.25)
(+=. fraction 0.25)
(get fraction) ; Returns: 0.5

(boolean flag false)
(boolean/set flag true)
(get flag) ; Returns: true

(variable xs [])
(set xs [1 2 3])
(get xs)
; Returns: [1 2 3]
```

## Mutation Operations

Que Script provides special functions and operators for mutating scalar values without using !.

#### These are the only mutable operations that lack the ! suffix.

#### Use (get variable) with no index to read the current value from a scalar box.

#### (get variable index) is only used for indexed structures like vectors.

---

| Operation   | Type Signature         | Description                          |
| ----------- | ---------------------- | ------------------------------------ |
| ++          | [Int] -> ()            | Increment integer by 1               |
| --          | [Int] -> ()            | Decrement integer by 1               |
| +=          | [Int] -> Int -> ()     | Add a value to integer               |
| -=          | [Int] -> Int -> ()     | Subtract a value from integer        |
| \*=         | [Int] -> Int -> ()     | Multiply integer by a value          |
| /=          | [Int] -> Int -> ()     | Divide integer by a value            |
| ++.         | [Float] -> ()          | Increment float by 1.0               |
| --.         | [Float] -> ()          | Decrement float by 1.0               |
| +=.         | [Float] -> Float -> () | Add a value to float                 |
| -=.         | [Float] -> Float -> () | Subtract a value from float          |
| \*=.        | [Float] -> Float -> () | Multiply float by a value            |
| /=.         | [Float] -> Float -> () | Divide float by a value              |
| boolean/set | [Bool] -> Bool -> ()   | Set boolean value                    |
| set         | [T] -> T -> ()         | Assign new value to generic variable |

---

Mutable Scalars vs Immutable Bindings

---

| Form              | Mutable?     | Example              | Result         |
| ----------------- | ------------ | -------------------- | -------------- |
| (let x 1)         | ❌ Immutable | (set! x 0 2)         | ❌ Error       |
| (integer x 1)     | ✅ Mutable   | (++ x)               | ✅ x = 2       |
| (floating x 1.0)  | ✅ Mutable   | (+= x 0.5)           | ✅ x = 1.5     |
| (boolean y false) | ✅ Mutable   | (boolean/set y true) | ✅ y = true    |
| (variable z [])   | ✅ Mutable   | (set z [1 2 3])      | ✅ z = [1 2 3] |

---

```lisp
(integer i 0)
(boolean done false)
(variable data [])

(loop (< (get i) 5)
  (lambda (do
    (push! (get data) (get i))
    (++ i)
    (if (= (get i) 5)
        (boole-set done true)
        nil))) )

(get data)
; Returns: [0 1 2 3 4]
```

## Mutable vs Immutable Cheatsheet

In Que Script, immutability is the default — but when you need to manage state or counters, mutable scalars provide safe, explicit mutability.

#### This table summarizes the key differences between immutable and mutable constructs.

---

| Feature                  | Immutable (let)                       | Mutable Scalar (integer / boolean / variable)                                              |
| ------------------------ | ------------------------------------- | ------------------------------------------------------------------------------------------ |
| Purpose                  | Fixed, unchanging values              | Changeable state or counters                                                               |
| Syntax                   | (let name value)                      | (integer name value) / (floating name value) /(boolean name value) / (variable name value) |
| Storage                  | Direct binding                        | Boxed value (a reference)                                                                  |
| Mutation                 | Not allowed                           | Allowed with set, ++, boolean/set, etc.                                                    |
| Access                   | Just name                             | (get name) to read current value                                                           |
| Return Type of Mutations | Unit                                  | Unit                                                                                       |
| Common Use Case          | Constants, parameters, pure functions | Counters, flags, accumulators, state tracking                                              |

---

```lisp
(let x 10)
(+ x 5)
; Returns: 15

(set x 20)
; ❌ Error
```

## Dynamic Mode, Side Effects, and Disabling the Type Checker

Que Script is built to be statically typed with a sound Hindley–Milner system and tail call optimization (TCO). However, for some real-world or exploratory problems, you might want to disable the type checker and run your code in dynamic mode — just like JavaScript (because it is JavaScript at that point).

#### You can enable dynamic mode in the online editor by clicking the 🐞 Bug Icon —

#### this turns off type checking and runs the JavaScript backend instead of the VM.

#### When you disable type checking and switch to JavaScript execution: Functions and closures are managed by JavaScript’s garbage collector. Self-referencing functions like backtrack are completely safe. The language effectively behaves like a dynamic Lisp.

```lisp
(+ true 10)
; 11 in Dynamic Mode
; Type error otherwise
```

# Examples

_Real-world examples demonstrating Que Script's capabilities._

## Halve

###

#### Function halve ; halve [T] to {[T] \* [T]} that splits an even-lengthed list into two halves solution

```lisp
; Define custom functions
;    halve :: [T] -> {[T] * [T]}
(let halve (lambda xs (do
          (let half (/ (length xs) 2))
          { (take/first half xs) (take/last half xs) })))


(halve [ 1 2 3 4 5 6 7 8 ]) ; { [ 1 2 3 4 ] [ 5 6 7 8 ] }
```

## Check Order

###

#### You are given an array of integers [Int]

#### Write a script to re-arrange the given array in an increasing order and return the indices where it differs from the original array.

---

| Example | Input         | Output    |
| ------- | ------------- | --------- |
| 1       | [5 2 4 3 1]   | [0 2 3 4] |
| 2       | [1 2 1 1 3]   | [1 3]     |
| 3       | [ 3 1 3 2 3 ] | [0 1 3]   |

---

```lisp
(let solve (lambda xs (|>
    { (|> xs (map identity) (sort! <)) xs }
    (zip)
    (map (lambda { a b } (<> a b)))
    (enumerate)
    (filter snd)
    (map fst))))
[
  (solve [ 5 2 4 3 1 ])
  (solve [ 1 2 1 1 3 ])
  (solve [ 3 1 3 2 3 ])
]
```

## Common Strings

###

#### You are given two array of strings. Write a script to return the count of common strings in both arrays

---

| Example | Input 1                      | Input 2                       | Output |
| ------- | ---------------------------- | ----------------------------- | ------ |
| 1       | ["que" "weekly" "challenge"] | ["lisp" "weekly" "challenge"] | 2      |
| 2       | ["que" "lisp" "python"]      | ["python" "java"]             | 1      |
| 3       | ["guest" "contribution"]     | ["fun" "weekly" "challenge"]  | 0      |

---

```lisp
(let common-strings (lambda xs ys (|> (Set/intersection (Vector->Set xs) (Vector->Set ys)) (flat) (length))))

[
  (common-strings ["que" "weekly" "challenge"] ["lisp" "weekly" "challenge"])
  (common-strings ["que" "lisp" "python"] ["python" "java"])
  (common-strings ["guest" "contribution"] ["fun" "weekly" "challenge"])
]
```

## Maximum Subarray Sum - Kadane's Algorithm

###

#### Given an integer array arr[], find the subarray (containing at least one element) which has the maximum possible sum, and return that sum.

#### Note: A subarray is a continuous part of an array.

```lisp
; Kadane's algorithm: returns maximum subarray sum for a vector of Ints
(let max-subarray-sum (lambda xs (do
  (let step (lambda acc x (do
    (let current (max x (+ (get acc 0) x)))
    (let best (max (get acc 1) current))
    [ current best ])))

  (let init [ (get xs 0) (get xs 0) ]) ; start with first element as current and best
  (let rest (cdr xs))
  (let result (reduce step init rest))
  (get result 1))))

; Examples
[
    (max-subarray-sum [ -2 1 -3 4 -1 2 1 -5 4 ]) ; Int -> 6 (subarray [4 -1 2 1])
    (max-subarray-sum [ 1 2 3 ]) ; Int -> 6
    (max-subarray-sum [ -3 -2 -1 ]) ; Int -> -1
]
```

## The Collatz Conjecture

The Collatz conjecture is an unsolved mathematical problem that asks whether a sequence starting from any positive integer will always eventually reach 1.

#### To generate the sequence, you apply a simple rule: if the number is even, divide it by 2;

#### if it is odd, multiply it by 3 and add 1.

#### This process is repeated until the number reaches 1.

#### I will use partial application and get free tail-call optimization.

```lisp
; TCO recursion
(let~ k-mod (lambda n k (if (< k n) k (k-mod n (- k n)))))
; taking advantage of partial apply
(let mod2 (k-mod 2))
; TCO recursion
(let~ collatz (lambda n steps
               (if (= n 1)
                    steps
                    (collatz (if (= (mod2 n) 0)
                                 (/ n 2)
                                 (+ (* n 3) 1))
                                 (+ steps 1)))))

(collatz 27 0)
; Int
; 111
```

## Advent Of Code 2019 Day 1

https://adventofcode.com/2019/day/1

```lisp
(let INPUT
"12
14
1969
100756")

(let parse (lambda input
    (|> input (String->Vector nl) (map Chars->Integer))))

(let PARSED (parse INPUT))

(let part1 (lambda input (|>
    input
    (map (lambda x (- (/ x 3) 2)))
    (sum))))

(let part2 (lambda input (do

(let retry (lambda x (do
    (let~ tail-call:retry! (lambda x out (do
        (let result (- (/ x 3) 2))
        (if (<= result 0) out
            (tail-call:retry! result (do (push! out result) out))))))
     (tail-call:retry! x []))))

 (|>
    input
    (map retry)
    (map sum)
    (sum)))))

[(part1 PARSED) (part2 PARSED)] ; [34241 51316]
```

## Advent Of Code 2016 Day 1

https://adventofcode.com/2016/day/1

```lisp
; The Document indicates that you should start at the given coordinates (where you just landed) and face North. Then, follow the provided sequence: either turn left (L) or right (R) 90 degrees, then walk forward the given number of blocks, ending at a new intersection.
; Following R2, L3 leaves you 2 blocks East and 3 blocks North, or 5 blocks away.
; R2, R2, R2 leaves you 2 blocks due South of your starting position, which is 2 blocks away.
; R5, L5, R5, R3 leaves you 12 blocks away.

(let parse (lambda input
    (|> (cons input ",")
        (String->Vector ' ')
        (map (lambda x (drop/last 1 x)))
        (map (lambda [ D M ] [ (Char->Int D) (Chars->Integer M) ])))))
(let delta-diff (lambda [ y x . ] (+ (abs y) (abs x))))
(let part1 (lambda input (|> input
    (reduce (lambda [ y x a . ] [ D M . ] (do
                                (let F (mod (+ a (if (=# (Int->Char D) 'R') 1 3)) 4))
                                (cond
                                    (= F 0) [ y (+ x M) F ]
                                    (= F 1) [ (- y M) x F ]
                                    (= F 2) [ y (- x M) F ]
                                    (= F 3) [ (+ y M) x F ]
                                    [ y x a ])))
                                [ 0 0 1 ])
    (delta-diff))))

; Then, you notice the instructions continue on the back of the Recruiting Document. Easter Bunny HQ is actually at the first location you visit twice.
; For example, if your instructions are R8, R4, R4, R8, the first location you visit twice is 4 blocks away, due East.
; How many blocks away is the first location you visit twice?
(let turn
  (lambda facing D
    (mod (+ facing (if (=# (Int->Char D) std/char/R) 1 3)) 4)))
(let step
  (lambda y x facing
    (cond
      (= facing 0) [(+ y 1) x]
      (= facing 1) [y (+ x 1)]
      (= facing 2) [(- y 1) x]
      (= facing 3) [y (- x 1)]
      [])))
(let point->key
  (lambda y x
    (cons (Integer->String y) "," (Integer->String x))))
(let part2
  (lambda input
    (do
      (integer y 0)
      (integer x 0)
      (integer facing 0) ; North

      (let visited (buckets 128))
      (Table/set! (point->key (get y) (get x)) true visited)

      (integer result 0)

      (for (lambda [ D M . ]
          (if (= (get result) 0)
              (do
                (set facing (turn (get facing) D))
                (loop 0 M
                  (lambda .
                    (if (= (get result) 0)
                        (do
                          (let p (step (get y) (get x) (get facing)))
                          (set y (get p 0))
                          (set x (get p 1))

                          (let key (point->key (get y) (get x)))
                          (if (Table/has? key visited)
                              (set result (+ (abs (get y))
                                             (abs (get x))))
                              (Table/set! key true visited))))))))) input)

      (get result))))

{
  (|> [ "R2, L3"  "R2, R2, R2" "R5, L5, R5, R3" ] (map parse) (map part1))
  (part2 (parse "R8, R4, R4, R8"))
} ; [[5 2 12] 4]

```

## N Queen

```lisp
(let n-queen (lambda n (do
    (let solutions [])
    (let cols [[] [] [] [] [] [] []])
    (let positive-diagonal [[] [] [] [] [] [] []])
    (let negative-diagonal [[] [] [] [] [] [] []])
    (let board (std/vector/3d/fill n n (lambda . . ".")))
    (let* backtrack (lambda row
      (if (= row n)
          (set! solutions (length solutions)
            (map (lambda a (Vector->String std/char/empty a)) board))
          (loop 0 n (lambda col
              (unless
                (or
                  (Set/has? (Integer->String col) cols)
                  (Set/has? (Integer->String (+ row col)) positive-diagonal)
                  (Set/has? (Integer->String (- row col)) negative-diagonal))
                (do
                  (Set/add! (Integer->String col) cols)
                  (Set/add! (Integer->String (+ row col)) positive-diagonal)
                  (Set/add! (Integer->String (- row col)) negative-diagonal)
                  (set! (get board row) col "Q")
                  (backtrack (+ row 1))
                  (Set/remove! (Integer->String col) cols)
                  (Set/remove! (Integer->String (+ row col)) positive-diagonal)
                  (Set/remove! (Integer->String (- row col)) negative-diagonal)
                  (set! (get board row) col "."))))))))
    (backtrack 0)
    solutions)))
  [(n-queen 1) (n-queen 4)]
```

## Stack-based Iteration (Manual Recursion Conversion)

Some recursive algorithms cannot be optimized with Tail Call Optimization (TCO).

#### This happens when recursive calls aren’t in tail position — i.e., the function still needs to perform work after the recursive call returns.

#### Instead of relying on the call stack, I can simulate recursion manually by maintaining our own stack.

#### This approach allows Que Script programs to stay safe and efficient even for deep recursions.

#### Consider leetcode problem 733. Flood Fill

#### - 1. Start with the seed pixel (sr, sc) and push it onto the stack.

#### - 2. While the stack is not empty, pop a coordinate.

#### - 3. If the pixel matches the original color:

#### - - - a. Recolor it.

#### - - - b. Push its four neighbors (up, down, left, right) onto the stack.

#### - 4. Repeat until all connected pixels are processed.

---

| Concept               | Description                                                        |
| --------------------- | ------------------------------------------------------------------ |
| Mutable Stack         | Implemented using a vector with push! and pull!.                   |
| Manual State          | I track image, current coordinates, and neighbors explicitly.      |
| Termination           | The loop exits when stack is empty.                                |
| Return Type           | Returns the updated image matrix.                                  |
| Avoids Stack Overflow | Safe for large images — all recursion is flattened into iteration. |

---

```lisp
(let flood-fill (lambda image sr sc color (do
    (let old (get image sr sc))
    (if (= old color)
        image
        (do
            (let m (length image))
            (let n (length (first image)))
            (let stack [[sr sc]])
            (loop (not (empty? stack)) (lambda (do
                (let t (pull! stack))
                (let i (get t 0))
                (let j (get t 1))
                (if (and (>= i 0) (< i m) (>= j 0) (< j n) (= (get image i j) old)) (do
                    (set! (get image i) j color)
                    (push! stack [(+ i 1) j])
                    (push! stack [(- i 1) j])
                    (push! stack [i (+ j 1)])
                    (push! stack [i (- j 1)])
                    nil)))))
        image)))))

(flood-fill [[1 1 1] [1 1 0] [1 0 1]] 1 1 2) ; Output: [[2 2 2] [2 2 0] [2 0 1]]
```

## Apple and Orange

[hackerrank/apple-and-orange link](https://www.hackerrank.com/challenges/apple-and-orange/problem)

```lisp
(let solve (lambda s t a b apples oranges (do
          (let helper (lambda xs m
            (|> xs
                (map (lambda x (+ x m)))
                (count (lambda x (and (>= x s) (<= x t)))))))
          [(helper apples a) (helper oranges b)])))

      (solve 7 11 5 15 [ -2 2 1 ] [ 5 -6 ])
```

## Pattern matching

Use tuples as enums

```lisp
(let Sword { 1 "Sword" })
(let Bow { 2 "Bow" })
(let Staff { 3 "Staff" })

(let match-weapon? (lambda { aId . } { bId . } (= aId bId)))

(let calc-damage (lambda base { crit? weapon }
  (cond
    (and crit? (match-weapon? weapon Sword)) (* base 2)
    (and (not crit?) (match-weapon? weapon Sword)) (+ base 5)
    (and crit? (match-weapon? weapon Bow)) (+ base 10)
    (and (not crit?) (match-weapon? weapon Bow)) (+ base 2)
    (and crit? (match-weapon? weapon Staff)) (* base 3)
    (and (not crit?) (match-weapon? weapon Staff)) base
    0)))

(calc-damage 12 { true Sword })
```

## Hexagons

Print a hexagon patterns

```lisp
(let times (lambda n s (do
  (let out [])
  (loop 0 n (lambda . (std/vector/cons! out s)))
  out)))

(let hexagons (lambda x y (times y [(times x "/ \_") (times x "\_/ ")])))

(|> (hexagons 5 5) (Vector->String nl) (String->Vector nl) (filter (lambda x (not (empty? x)))) (Vector->String nl))
```

## Everybody Codes Quest 5

Part 1

```lisp
(let INPUT "58:5,3,7,8,9,10,4,5,7,8,8")
(let parse (lambda input (do
  (let parts (|> input (String->Vector ':')))
  (let head (|> (car parts) (Chars->Integer)))
  (let tail (|> (cdr parts) (car) (String->Vector ',') (map Chars->Integer)))
  { head tail })))

;  5   3-5   3-5-7   3-5-7   3-5-7   3-5-7   3-5-7   3-5-7   3-5-7   3-5-7   3-5-7 
;                      |       |       |       |       |       |       |       |
;                      8       8-9     8-9   4-8-9   4-8-9   4-8-9   4-8-9   4-8-9
;                                      |       |       |       |       |       |
;                                      10      10    5-10    5-10    5-10    5-10
;                                                              |       |       |
;                                                              7       7-8     7-8
;                                                                              |
;                                                                              8
; Check all segments of the spine, starting from the top.
; If your number is less than the one on the spine segment
; and the left side of the segment is free - place it on the left.
; If your number is greater than the one on the spine segment
; and the right side of the segment is free - place it on the right.
; If no suitable place is found at any segment,
; create a new spine segment from your number and place it as the last one.

(let part1 (lambda { . nums } (do
  (let sword [[-1 (get nums 0) -1]])
  (loop 1 (length nums) (lambda i (do
    (let num (get nums i))
    (boolean placed false)
    (loop 0 (length sword) (lambda j (do
      (let segment (get sword j))
      (cond
          (and (false? placed) (< num (get segment 1)) (= (get segment 0) -1))
                (do (set! segment 0 num) (boolean/set placed true))
          (and (false? placed) (> num (get segment 1)) (= (get segment 2) -1))
                (do (set! segment 2 num) (boolean/set placed true))
          nil))))
      (if (false? placed) (push! sword [-1 num -1])))))
  (flat (map sword (lambda [. x .] (std/convert/integer->string x)))))))
(part1 (parse INPUT))
```

## P5 (Processing) sketch Fractal Tree

Fractal Tree

```lisp
  (let R (- 1 (p5/div 1 4)))
(let angle (p5/div 1 2))
(let type/branch 0)
(let type/pop 1)

(let branch (lambda initialLen (Rec [ type/branch initialLen 0 ]
  (lambda [ type len stage . ] (do
    (cond
      (= type type/branch) (do
          ; --- Stage 0: draw the line and move upward
          (cond
            (= stage 0) (do
              (p5/line 0 0 0 (- len))
              (p5/translate 0 (- len))
              (if (<= len 10)
                ; base case: nothing more to push
                { Rec/none [] }
                ; recurse: schedule continuation (stage 1) then child (so child is last)
                { Rec/push [
                      ; continuation for this branch (run stage 1 next)
                      [ type/branch len 1 ]
                    ]
                }))
          ; --- Stage 1: run the left branch
            (= stage 1) (do
              (p5/push)
              (p5/rotate angle)
              {
                Rec/push [
                       ; after left finishes continue with stage 2
                       [ type/branch len 2 ]
                       ; pop to restore transform after left child
                       [ type/pop 0 0 ]
                       [ type/branch (* len R) 0 ]
                    ]
              })
          ; --- Stage 2: run the right branch
            (= stage 2) (do
              (p5/push)
              (p5/rotate (- angle))
              {
                Rec/push [
                    [ type/pop 0 0 ]
                    [ type/branch (* len R) 0 ]
                  ]
              }) { Rec/none [] }))
      (= type type/pop) (do
          (p5/pop)
          { Rec/none [] })
       { Rec/none [] }))))))

(let SETUP (lambda (do
  (p5/background 50 50 50 50)
  (p5/stroke 200 50 100 255)
  (p5/translate (/ 500 2) 425)
  (branch 100))))

(let DRAW false)
```

## P5 (Processing) sketch Game of Life

Conway's Game of Life

```lisp
  (let N 9)

(let add-glider! (lambda matrix y x (do
  (set! (get matrix (+ y 2)) (+ x 1) 1)
  (set! (get matrix (+ y 2)) (+ x 2) 1)
  (set! (get matrix (+ y 2)) (+ x 3) 1)
  (set! (get matrix (+ y 1)) (+ x 3) 1)
  (set! (get matrix (+ y 0)) (+ x 2) 1))))

(let gof (lambda matrix (do
  (std/vector/map/i matrix (lambda arr y (do
    (std/vector/map/i arr (lambda cell x (do
      (let score (std/vector/3d/sliding-adjacent-sum matrix std/vector/3d/moore-neighborhood y x N +))
      (cond
        (and (= cell 1) (or (< score 2) (> score 3))) 0
        (and (= cell 1) (or (= score 2) (= score 3))) 1
        (and (= cell 0) (= score 3)) 1
        0))))))))))

(variable GRID [])

(let SETUP (lambda (do
  (let matrix (std/vector/3d/fill N N (lambda . . 0)))
  (add-glider! matrix 0 0)
  ; (set! (get matrix 6) 2 1)
  ; (set! (get matrix 5) 4 1)
  ; (set! (get matrix 5) 3 1)
  ; (set! (get matrix 3) 3 1)
  (set GRID (gof matrix))
  (p5/noStroke))))

(let DRAW (lambda (do
  (p5/frameRate 5)
  (let M (get GRID))
  (let y (length M))
  (let x (length (get M 0)))
  (let R 56)
  (std/vector/3d/for/i M (lambda c i j (do
      (if (= c 1) (p5/fill 155 155 255 255) (p5/fill 0 0 0 255))
      (p5/rect (* i R) (* j R) R R))))
  (set GRID (gof M)))))
```

## P5 (Processing) sketch Particles

Saturn

```lisp
  (integer counter 100)
(boolean parity_flag false)
(let inc (p5/div 1 100))

(let~ loop/reverse/even (lambda cb i
  (if (>= i 0) (do (cb i) (loop/reverse/even cb (- i 2))) i)))
(let~ loop/reverse/odd (lambda cb i
  (if (> i 0) (do (cb i) (loop/reverse/odd cb (- i 2))) i)))

; code that will run once here
(let SETUP (lambda (do (p5/stroke 255 255 255 255))))

(let W 500) ; Canvas Width
(let H 500) ; Canvas Height
(let C 250) ; Half

(let draw-points (lambda i (do
    ; calculation distance or magnitude from center
    (let radial_offset (+ (p5/div (get counter)
                          (p5/cos (p5/div (get counter) i)))
                          (* (Bool->Int (get parity_flag))
                          (+ (p5/div (get counter) 2) (mod i (get counter))))))
    ; time-based angle or rotation factor, scaled by iteration
    (let angular_phase (+ (p5/div (get counter) 9) (* i i)))
    (let x_position (+ C (* radial_offset (p5/sin angular_phase)
                        (p5/cos (*
                                (Bool->Int (not (get parity_flag)))
                                (p5/div i (get counter)))))))
    (let y_position (+ C (* radial_offset
                         (p5/cos (+ angular_phase
                                (* (Bool->Int (get parity_flag)) 2))))))
    ;  varies between 0 (dim/invisible) and 2 (bright/large dots).
    (let point_size (- 1 (p5/cos angular_phase)))

    (p5/strokeWeight point_size)
    (p5/point x_position y_position))))
; (let DRAW false)
(let DRAW (lambda (do
    (p5/background 0 0 0 255) ; last one is alpha it can go to 255
    (+= counter inc)
    (loop/reverse/even (lambda i (do
                    (set parity_flag false)
                    (draw-points i)))
    2000)
    (loop/reverse/odd (lambda i (do
                    (set parity_flag true)
                    (draw-points i)))
    1999))))
```

## P5 (Processing) sketch Rose

Math Rose

```lisp
  ; Mathematical Roses
; Based on: https://en.wikipedia.org/wiki/Rose_(mathematics)

(let d 8)
(let n 6)

(let width 500)
(let height 500)

(let reduceDenominator (lambda numerator denominator (do
  (let~ rec (lambda a b (if (> b 0) (rec b (mod a b)) a)))
  (/ denominator (rec numerator denominator)))))

(let SETUP (lambda (do
    ; slightly transaprent so you can see the grid
    (p5/background 51 51 51 100) ; last one is alpha it can go to 255
    (p5/push)
    (p5/translate (/ width 2) (/ height 2))
    (p5/beginShape)
    (p5/stroke 255 255 255 255)
    (p5/noFill)
    (p5/strokeWeight 1)
    (let k (p5/div n d))
    (let inc (p5/div 2 100))
    (variable a 0)
    (loop (< (get a) (* p5/PI 2 (reduceDenominator n d))) (lambda (do
      (let r (* 200 (p5/cos (* k (get a)))))
      (let x (* r (p5/cos (get a))))
      (let y (* r (p5/sin (get a))))
      (p5/vertex x y)
      (+= a inc))))
    (p5/endShape)
    (p5/pop))))

; if you need animations
; replace DRAW with (let DRAW (lambda (do ... )))
(let DRAW false)
```

## P5 (Processing) sketch Hearts

Math Heart

Shape Heart

# Exercises

_Practice what you've learned with hands-on coding exercises. Challenge yourself to write Que Script code and reinforce your understanding._

## Halve

Function halve ; halve [T] to {[T] \* [T]} that splits an even-lengthed list into two halves

```lisp
;    halve :: [T] -> {[T] * [T]}
(let halve (lambda xs (do
          ; your code here
          xs ; must return { first-half sedcond-half}
)))

(halve [ 1 2 3 4 5 6 7 8 ]) ; { [ 1 2 3 4 ] [ 5 6 7 8 ] }
```

## Length

Find the length a list. len: [T] to Int to [T]

```lisp
;    len :: [T] -> Int
(let len (lambda xs (do
          ; your code here
          xs ; must return length of list
)))

(len [ 1 2 3 4 5 6 7 8 ]) ; 8
```

## Tail

Write a function lst: [T] to T that returns the last element of a list

```lisp
;    tail :: [T] -> T
(let tail (lambda xs (do
          ; your code here
          xs ; must return last element
)))

(tail [ 1 2 3 4 5 6 7 8 ]) ; 8
```

## Last Two Elements

Find the last two (last and penultimate) elements of a list lst-2: [T] to [T]

```lisp
;    last-two :: [T] -> [T]
(let last-two (lambda xs (do
          ; your code here
          xs ; must return last two elements
)))

(last-two [ 1 2 3 4 5 6 7 8 ]) ; [7 8]
```

## N'th Element

Find the N'th element of a list. lst-n: [T] to Int to [T]

```lisp
;    nth-element :: [T] -> Int -> T
(let nth-element (lambda xs i (do
          ; your code here
          xs ; must return nth elements
)))

(nth-element [ 1 2 3 4 5 6 7 8 ] 3) ; 4
```

## Range

Range of integers from start to including end: Int to Int to [Int]

```lisp
;    rng :: Int -> Int -> [Int]
(let rng (lambda start end (do
          ; your code here
          [] ; returns range from start to end
)))

(rng 1 6) ; [ 1 2 3 4 5 6 ]
```

## Reverse

Reverse a list. rev: [T] to [T]

```lisp
;    rev :: [T] -> [T]
(let rev (lambda xs (do
          ; your code here
          xs ; must return reversed list
)))

(rev [ 1 2 3 4 5 6 7 8 ]) ; [ 8 7 6 5 4 3 2 1 ]
```

## Palindrome

Find out whether a list is a palindrome. palindrome?: [Char] to Bool or [Int] to Bool

```lisp
;    palindrome? :: [Char] -> Bool
(let palindrome? (lambda xs (do
          ; your code here
          xs ; returns true if palimdrome
)))

(palindrome? "x.x.xxxx.xxxx.x.x") ; true
```

## Insert an Element at a Given Position Into a List

Start counting list elements with 0. If the position is larger or equal to the length of the list, insert the element at the end.

```lisp
;    insert/at :: [T] -> T -> Int -> [T]
(let insert/at (lambda xs x idx (do
          ; your code here
          xs ; returns extended list
)))

(insert/at [ 1 2 3 4 ] 100 2) ; [ 1 2 100 3 4 ]
```

## Eliminate Duplicates

Eliminate consecutive duplicates of list elements. de-dup: [Int] to [Int] or [Char] to [Char]

```lisp
;    de-dup :: [Int] -> [Int]
(let de-dup (lambda xs (do
          ; your code here
          xs ; returns list with no duplicates
)))

(de-dup [ 1 1 2 3 3 4 4 4 4 5 ]) ; [ 1 2 3 4 5 ]
```

## Determine the Greatest Common Divisor of Two Positive Integer Numbers

gcd: Int to Int to Int

```lisp
;    gcd :: Int -> Int -> Int
(let gcd (lambda a b (do
          ; your code here
          0 ; return GCD of a and b
)))

(gcd 20 15) ; 5
```

## Run-Length Encoding

If you need so, refresh your memory about [run-length encoding](https://en.wikipedia.org/wiki/Run-length_encoding)

```lisp
;    encode :: [[Char]] -> [{Int * [Char]}]
(let encode (lambda xs (do
  xs ; the run length encoding of the list
)))
(encode ["a" "a" "a" "a" "b" "c" "c" "a" "a" "d" "e" "e" "e" "e"])
; [[4 "a"] [1 "b"] [2 "c"] [2 "a"] [1 "d"] [4 "e"]]
```
