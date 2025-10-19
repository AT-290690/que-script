# Lamb

<img src="./logo-lamb.png" width="256px"/>

My hobby, scripting language implemented in Rust.
It's called **Lamb**, short for **Lambda** the greek letter **λ**
(the **keyword** for the [anonymous functions](https://en.wikipedia.org/wiki/Anonymous_function) in the language)

**Lamb** is:

- **[Lisp](<https://en.wikipedia.org/wiki/Lisp_(programming_language)>)**
- **[Stack-based bytecode Virtual Machine](https://en.wikipedia.org/wiki/Stack_machine)**
- **[Standard library](https://en.wikipedia.org/wiki/Standard_library)**
- **[Tree-shaking](https://en.wikipedia.org/wiki/Tree_shaking)** of Standard Libary
- **[Strictly evaluated](https://en.wikipedia.org/wiki/Evaluation_strategy)**
- Everything is an **[Expression](<https://en.wikipedia.org/wiki/Expression_(computer_science)>)**
- **[Syntactic sugar](https://en.wikipedia.org/wiki/Syntactic_sugar)** layer
- **[Strongly typed](https://en.wikipedia.org/wiki/Strong_and_weak_typing)** using the **[Hindley-Milner](https://en.wikipedia.org/wiki/Hindley–Milner_type_system)** type system
- **[Compiler](https://en.wikipedia.org/wiki/Compiler)** to [JavaScript](https://en.wikipedia.org/wiki/JavaScript)
- **[WASM](https://en.wikipedia.org/wiki/WebAssembly)** build for [online editor](https://at-290690.github.io/rust-lisp/)
- It supports some cool features from **functional programming**

  - **[Partial function application](https://en.wikipedia.org/wiki/Partial_application)**
  - **[Lexically scoped closures](<https://en.wikipedia.org/wiki/Closure_(computer_programming)>)**
  - **[First-class functions](https://en.wikipedia.org/wiki/First-class_function)**
  - **[Type inference](https://en.wikipedia.org/wiki/Type_inference)**
  - **[Tail Call Optimization](https://en.wikipedia.org/wiki/Tail_call)**

Try it online at [playground](https://at-290690.github.io/rust-lisp/)

or clone this project and write code in **main.lisp**

```bash
# type check and eval
cargo run
```

build wasm bundle

```bash
wasm-pack build --target nodejs --out-dir pkg/node
wasm-pack build --target web --out-dir pkg/web
```

---

### Hindley–Milner Type Inference

- No type annotations required: the compiler figures everything out.
- Supports **polymorphism** and **higher-order functions**.
- Only 4 types - **functions**, **booleans**, **integers** and **arrays**.
- Guarantees **soundness**: if your program compiles, it won’t have type errors at runtime.
- Example:

```lisp
(let sum-odd-squares (lambda xs
    (|> xs
        (std:vector:filter std:int:odd?)
        (std:vector:map std:int:square)
        (std:vector:int:sum))))

(sum-odd-squares [ 1 2 3 4 5 6 7 8 9 10 ])
; Int
; 165
```

- **filter**, **map** and **sum** will be tree shaked from std.
- Pipe (|> ... ) will be desuggered to:

```lisp
(std:vector:int:sum (std:vector:map (std:vector:filter xs std:int:odd?) std:int:square))
```

- Argument type of the function will be [Int].
- Return type of the function will be Int.
- **filter** will only work with [Int] and callback of type Int -> Bool
- **map** will only work with [Int] and callback of type Int -> Int
- **sum** will only work with [Int]

Short names can be extracted from std using **import**:

```lisp
(import filter map std:vector)
(import odd? square std:int)
(import sum std:vector:int)

(let sum-odd-squares (lambda xs
    (|> xs
        (filter odd?)
        (map square)
        (sum))))

(sum-odd-squares [ 1 2 3 4 5 6 7 8 9 10 ])
```

**sum** is a sub import of vector under **int**:

```lisp
; import int:sum directly from std:vector
(import int:sum filter map std:vector)
(import odd? square std:int)

(let xs [ 1 2 3 4 5 6 7 8 9 10 ])
; here we use int:sum
(int:sum (map (filter xs int:odd?) int:square))
```

### Tail Call Optimization

A call is said to be in tail position if it is the last instruction executed before returning from the current function. Compilers can optimize such calls by discarding the caller frame and replacing the call with a jump.

This is especially useful for recursive functions. For instance, take this function that sums the elements of a vector:

```lisp
(let sum (lambda xs acc
    (if (= (length xs) 0) acc
        (sum (std:vector:drop xs 1) (+ acc (get xs 0))))))

(sum [ 1 2 3 4 5 ] 0)
; Int
; 15
```

With a regular call, this consumes 𝒪(n) stack space: each element of the vector adds a new frame on the call stack. With a long enough vector, this could very quickly overflow the stack. By replacing the call with a jump, tail call optimization effectively turns this recursive function into a loop which uses 𝒪(1) stack space:

```lisp
(let sum (lambda xs acc (do
    (let _acc [ acc ])
    (let _xs [ xs ])
    (let _new_xs [])
    (let _new_acc [])
    (loop (not (= (length (get _xs 0)) 0)) (lambda (do
        (set! _new_xs 0 (std:vector:drop (get _xs 0) 1))
        (set! _new_acc 0 (+ (get _acc 0) (get (get _xs 0) 0)))
        (set! _xs 0 (get _new_xs 0))
        (set! _acc 0 (get _new_acc 0)))))
    (get _acc 0))))
```

This optimization is particularly important for functional languages. They rely heavily on recursive functions, and pure ones like Haskell don’t even provide loop control structures. Any kind of custom iteration typically uses recursion one way or another. Without tail call optimization, this would very quickly run into a stack overflow for any non-trivial program:

```lisp
; TCO recursion
(let k-mod (lambda n k (if (< k n) k (k-mod n (- k n)))))
; taking advantage of partial apply
(let mod2 (k-mod 2))
; TCO recursion
(let collatz (lambda n steps
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

**Tail Call Optimization Convention**

When a function is transformed for tail-call optimization, the last parameter is treated as the accumulator/result parameter.
On entry, the compiler initializes the result register from this last parameter.
On each tail-call, the accumulator is updated before the next iteration.
This guarantees that type inference can resolve the result type statically.
Users are encouraged to design tail-recursive functions so that the accumulator is the last argument.

**Cast**

An empty vector has a polymorphic type (it can contain anything):

```lisp
(let xs [])
xs
; [t7]
; []
```

To enforce a type we can use **as**:

```lisp
(let xs (as [] [Int]))
xs
; [Int]
; []
```

Now the vector can only have **Ints** and will error out if anything else is pushed to it.

_Note: This works nicely for vectors of any depth with concrete types Int or Bool. Lambdas are a bit awkward to cast and are therefore discouraged for now._

### Loop Limit

Loops are capped at 5,000,000 (five million) total iterations for the entire program.

To ensure programs remain safe when running locally or on shared servers, loops must be "safe" and unable to hang or block the main thread. This limit also applies to tail-call optimized recursion.

### Solving Puzzles

Starting in the top left corner of a 2x2 grid, and only being able to move to the right and down, there are exactly 6 routes to the bottom right corner:

```lisp
(let factorial (lambda n total
   (if (= n 0)
       total
       (factorial (- n 1) (* total n)))))

(let bionomial-coefficient (lambda a b
    (/ (factorial a 1)
            (*
                (factorial b 1)
                (factorial (- a b) 1)))))

(let m 2)
(let n 2)
(bionomial-coefficient (+ m n) m)
; Int
; 6
```

How many such routes are there through a 20x20 grid?
Unfortunately, we can't fit that number in 32 big integers.
Instead we have to use **Big** integers (or numbers as a vectors with arbitrary precision):

```lisp
(let factorial (lambda n total
        (if (= (get n 0) 0)
            total
            (factorial (std:int:big:sub n [ 1 ]) (std:int:big:mul total n)))))

(let bionomial-coefficient (lambda a b
    (std:int:big:div (factorial a [ 1 ])
            (std:int:big:mul
                (factorial b [ 1 ])
                (factorial (std:int:big:sub a b) [ 1 ])))))

(let m [ 2 0 ])
(let n [ 2 0 ])
(bionomial-coefficient (std:int:big:add m n) m)
; [Int]
; [1 3 7 8 4 6 5 2 8 8 2 0]
```

**Advent of Code 2015**

--- Day 1: Not Quite Lisp ---

_Santa is trying to deliver presents in a large apartment building, but he can't find the right floor - the directions he got are a little confusing. He starts on the ground floor (floor 0) and then follows the instructions one character at a time._

_An opening parenthesis, (, means he should go up one floor, and a closing parenthesis, ), means he should go down one floor._

_The apartment building is very tall, and the basement is very deep; he will never find the top or bottom floors._

For example:

```
(()) and ()() both result in floor 0.
((( and (()(()( both result in floor 3.
))((((( also results in floor 3.
()) and ))( both result in floor -1 (the first basement level).
))) and )())()) both result in floor -3.
To what floor do the instructions take Santa?
```

```lisp
(let samples [
    "(())"    ; result in floor 0.
    "()()"    ; result in floor 0.
    "((("     ; result in floor 3.
    "(()(()(" ; result in floor 3.
    "))(((((" ; also results in floor 3.
    "())"     ; result in floor -1 (the first basement level).
    "))("     ; result in floor -1 (the first basement level).
    ")))"     ; result in floor -3.
    ")())())" ; result in floor -3.
])
(let solve (lambda input (- (std:vector:int:count input std:int:char:left-brace) (std:vector:int:count input std:int:char:right-brace))))
(std:vector:map samples solve)
; [Int]
; [0 0 3 3 3 -1 -1 -3 -3]
```

### WASM usage

**Web**

```js
import init, {
  exec,
  comp,
  cons,
  run,
  check,
  js,
  get_output_len,
} from "./pkg/web/fez_rs.js";
(async () => {
  const wasm = await init();
})();
```

**Node**

```js
import {
  exec,
  comp,
  cons,
  run,
  check,
  js,
  get_output_len,
} from "./pkg/node/fez_rs.js";
const memory = __wasm.memory;
```

**Helper functions**

```js
const readWasmString = (ptr, len) =>
  new TextDecoder().decode(new Uint8Array(wasm.memory.buffer, ptr, len));
// Use these
const typeCheck = (program) => readWasmString(check(program), get_output_len());
const compileJs = (program) => readWasmString(js(program), get_output_len());
const compileBiteCode = (program) =>
  readWasmString(comp(program), get_output_len());
const execBiteCode = (program) =>
  readWasmString(exec(program), get_output_len());
const concatenateBiteCode = (a, b) =>
  readWasmString(cons(a, b), get_output_len());
const typeCheckAndRun = (program) =>
  readWasmString(run(program), get_output_len());
```

**Example**

```js
const program = "(+ 1 2)";
console.log(typeCheck(program));
console.log(execBiteCode(compileBiteCode(program)));
```

**Disclaimer!**

<img src="./bug.png" width="64px"/>

_This project is a work in progress and might contain bugs!_
