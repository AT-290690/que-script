# Programming Language

An experimental functional programming language with:

- **Hindley–Milner (HM) type inference**
- **tree-shaking** of standard libary
- A **stack-based bytecode VM** for speed

Write code in main.lisp and quickly type check and run it with

```bash
cargo run
```

---

### Hindley–Milner Type Inference

- No type annotations required: the compiler figures everything out.
- Supports polymorphism and higher-order functions.
- Only 4 types - functions, booleans, integers and arrays.
- Guarantees **soundness**: if your program compiles, it won’t have type errors at runtime.
- Example:

```lisp
(let sum-odd-squares (lambda xs
    (|> xs
        (filter odd?)
        (map square)
        (sum))))

(sum-odd-squares [ 1 2 3 4 5 6 7 8 9 10 ])
; Int
; 165
```

- **filter**, **map** and **sum** will be tree shaked from std.
- Pipe (|> ... ) will be desuggered to:

```lisp
(sum (map (filter xs odd?) square))
```

- Argument type of the function will be [Int].
- Return type of the function will be Int.
- **filter** will only work with [Int] and callback of type Int -> Bool
- **map** will only work with [Int] and callback of type Int -> Int
- **sum** will only work with [Int]

```lisp
; The first Advent of Code puzzle
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
(let solve (lambda input (- (count input char:left-brace) (count input char:right-brace))))
(map samples solve)
; [Int]
; =>  [0 0 3 3 3 -1 -1 -3 -3]
```
