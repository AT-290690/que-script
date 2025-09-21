# Resin

<img src="./logo.png" width="256px"/>

My hobby scripting language implemented in Rust.
(it's called resin because it's tree based and catches bugs)

Try it online at [playground](https://at-290690.github.io/rust-lisp/)

- **Lisp**
- **Stack-based bytecode Virtual Machine**
- **Standard library**
- **Tree-shaking** of standard libary
- **Strictly evaluated**
- Everything is an **Expression**
- **Syntactic suggar** layer
- **Strongly typed** using the **Hindley-Milner** type system
- It supports some cool features from **functional programming**
  - **Partial function application**
  - **Lexically scoped closures**
  - **First-class functions**
  - **Type inference**
  - **Tail Call Optimization**

Write code in **main.lisp**
Type check and execute with:

```bash
cargo run
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
        (std:vector:ints:sum))))

(sum-odd-squares [ 1 2 3 4 5 6 7 8 9 10 ])
; Int
; 165
```

- **filter**, **map** and **sum** will be tree shaked from std.
- Pipe (|> ... ) will be desuggered to:

```lisp
(std:vector:ints:sum (std:vector:map (std:vector:filter xs std:int:odd?) std:int:square))
```

- Argument type of the function will be [Int].
- Return type of the function will be Int.
- **filter** will only work with [Int] and callback of type Int -> Bool
- **map** will only work with [Int] and callback of type Int -> Int
- **sum** will only work with [Int]

Short names can be extracted from std using **import**

```lisp
(import filter map std:vector)
(import odd? square std:int)
(import sum std:vector:ints)

(let sum-odd-squares (lambda xs
    (|> xs
        (filter odd?)
        (map square)
        (sum))))

(sum-odd-squares [ 1 2 3 4 5 6 7 8 9 10 ])
```

**sum** is a sub import of vector under **ints**

```lisp
; import ints:sum directly from std:vector
(import ints:sum filter map std:vector)
(import odd? square std:int)

(let xs [ 1 2 3 4 5 6 7 8 9 10 ])
; here we use ints:sum
(ints:sum (map (filter xs int:odd?) int:square))
```

### Recursive Tail Call Optimization

```lisp
; tco recursion
(let k-mod (lambda n k (if (< k n) k (k-mod n (- k n)))))
; taking advantage of partial apply
(let mod2 (k-mod 2))
; tco recursion
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

**Tail-Call Optimization Convention**

When a function is transformed for tail-call optimization, the last parameter is treated as the accumulator/result parameter.
On entry, the compiler initializes the result register from this last parameter.
On each tail-call, the accumulator is updated before the next iteration.
This guarantees that type inference can resolve the result type statically.
Users are encouraged to design tail-recursive functions so that the accumulator is the last argument.

### Solving Puzzles

Starting in the top left corner of a 2x2 grid, and only being able to move to the right and down, there are exactly 6 routes to the bottom right corner.

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
(let solve (lambda input (- (std:vector:ints:count input std:int:char:left-brace) (std:vector:ints:count input std:int:char:right-brace))))
(std:vector:map samples solve)
; [Int]
; [0 0 3 3 3 -1 -1 -3 -3]
```
