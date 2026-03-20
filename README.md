# Que Script

**A pocket lisp that gives you safety without verbosity**

- **[Lisp](<https://en.wikipedia.org/wiki/Lisp_(programming_language)>)**
- **[Stack-based bytecode Virtual Machine](https://en.wikipedia.org/wiki/Stack_machine)**
- **[Standard library](https://en.wikipedia.org/wiki/Standard_library)**
- **[Tree-shaking](https://en.wikipedia.org/wiki/Tree_shaking)** of Standard Libary
- **[Strictly evaluated](https://en.wikipedia.org/wiki/Evaluation_strategy)**
- Everything is an **[Expression](<https://en.wikipedia.org/wiki/Expression_(computer_science)>)**
- **[Syntactic sugar](https://en.wikipedia.org/wiki/Syntactic_sugar)** layer
- **[Strongly typed](https://en.wikipedia.org/wiki/Strong_and_weak_typing)** using the **[Hindley-Milner](https://en.wikipedia.org/wiki/Hindley–Milner_type_system)** type system
- **[WASM](https://en.wikipedia.org/wiki/WebAssembly)** build for [online editor](https://at-290690.github.io/rust-lisp/playground)
- It supports some cool features from **functional programming**

- **[Partial function application](https://en.wikipedia.org/wiki/Partial_application)**
- **[Lexically scoped closures](<https://en.wikipedia.org/wiki/Closure_(computer_programming)>)**
- **[First-class functions](https://en.wikipedia.org/wiki/First-class_function)**
- **[Anonymous Functions](https://en.wikipedia.org/wiki/Anonymous_function)**
- **[Type inference](https://en.wikipedia.org/wiki/Type_inference)**
- **[Tail Call Optimization](https://en.wikipedia.org/wiki/Tail_call)**

Try it online at [playground](https://at-290690.github.io/rust-lisp/editor)

Check out official website at [website](https://at-290690.github.io/rust-lisp/)

## Install

### Linux / macOS

Using the installer scripts:

```bash
curl -fsSL https://raw.githubusercontent.com/AT-290690/que-script/refs/heads/main/scripts/install.sh | bash
curl -fsSL https://raw.githubusercontent.com/AT-290690/que-script/refs/heads/main/scripts/install-wat.sh | bash
curl -fsSL https://raw.githubusercontent.com/AT-290690/que-script/refs/heads/main/scripts/lsp.sh | bash
```

Without the installer scripts:

```bash
curl -fsSL https://github.com/AT-290690/que-script/releases/latest/download/que -o /tmp/que
chmod +x /tmp/que
sudo mv /tmp/que /usr/local/bin/que

curl -fsSL https://github.com/AT-290690/que-script/releases/latest/download/quewat -o /tmp/quewat
chmod +x /tmp/quewat
sudo mv /tmp/quewat /usr/local/bin/quewat

curl -fsSL https://github.com/AT-290690/que-script/releases/latest/download/quelsp -o /tmp/quelsp
chmod +x /tmp/quelsp
sudo mv /tmp/quelsp /usr/local/bin/quelsp

curl -fsSL https://github.com/AT-290690/que-script/releases/latest/download/que-lib.lisp -o /tmp/que-lib.lisp
sudo mkdir -p /usr/local/share/que
sudo mv /tmp/que-lib.lisp /usr/local/share/que/que-lib.lisp
```

### Windows

Using the PowerShell installers:

```powershell
irm https://raw.githubusercontent.com/AT-290690/que-script/refs/heads/main/scripts/install.ps1 | iex
irm https://raw.githubusercontent.com/AT-290690/que-script/refs/heads/main/scripts/install-wat.ps1 | iex
irm https://raw.githubusercontent.com/AT-290690/que-script/refs/heads/main/scripts/lsp.ps1 | iex
```

This installs to:

- `%LOCALAPPDATA%\Programs\Que\bin`
- `%LOCALAPPDATA%\Programs\Que\share\que\que-lib.lisp`

Without the installers:

1. Create:
   - `%LOCALAPPDATA%\Programs\Que\bin`
   - `%LOCALAPPDATA%\Programs\Que\share\que`
2. Download these release assets into those folders:
   - `que.exe`
   - `quewat.exe`
   - `quelsp.exe`
   - `que-lib.lisp`
3. Add `%LOCALAPPDATA%\Programs\Que\bin` to your user `PATH`.
4. For VS Code, set:

```json
"que.languageServer.path": "C:\\Users\\<you>\\AppData\\Local\\Programs\\Que\\bin\\quelsp.exe"
```

### Uninstall

Linux / macOS:

```bash
curl -fsSL https://raw.githubusercontent.com/AT-290690/que-script/refs/heads/main/scripts/uninstall.sh | bash
```

Windows:

```powershell
irm https://raw.githubusercontent.com/AT-290690/que-script/refs/heads/main/scripts/uninstall.ps1 | iex
```

## Build

```bash
./scripts/build-all.sh
```

Test

```bash
cargo test
```

---

### Hindley–Milner Type Inference

- No type annotations required: the compiler figures everything out.
- Supports **polymorphism** and **higher-order functions**.
- Only 7 types - **functions**, **booleans**, **integers**, **floats**, **characters**, **vectors** and **tuples**.
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
(sum (map square (filter odd? xs)))
```

- Argument type of the function will be [Int].
- Return type of the function will be Int.
- **filter** will only work with [Int] and callback of type Int -> Bool
- **map** will only work with [Int] and callback of type Int -> Int
- **sum** will only work with [Int]

### Solving Puzzles

Starting in the top left corner of a 2x2 grid, and only being able to move to the right and down, there are exactly 6 routes to the bottom right corner:

```lisp
(letrec factorial (lambda n total
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
(letrec factorial (lambda n total
        (if (= (get n 0) 0)
            total
            (factorial (BigInt/sub n [ 1 ]) (BigInt/mul total n)))))

(let bionomial-coefficient (lambda a b
    (BigInt/div (factorial a [ 1 ])
            (BigInt/mul
                (factorial b [ 1 ])
                (factorial (BigInt/sub a b) [ 1 ])))))

(let m [ 2 0 ])
(let n [ 2 0 ])
(bionomial-coefficient (BigInt/add m n) m)
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
(let solve (lambda input (- (count/char '(' input) (count/char ')' input))))
(map solve samples)
; [Int]
; [0 0 3 3 3 -1 -1 -3 -3]
```

Setup some env flags

```bash
  export QUE_WASM_OPT=speed QUE_TCO=aggressive QUE_DEVIRTUALIZE=aggressive QUE_BOUNDS_CHECK=0 QUE_DIV_ZERO_CHECK=0 QUE_VEC_MIN_CAP=8 QUE_VEC_GROWTH_NUM=2 QUE_VEC_GROWTH_DEN=1
```

Fallback to default ones

```bash
  unset QUE_WASM_OPT QUE_TCO QUE_DEVIRTUALIZE QUE_BOUNDS_CHECK QUE_DIV_ZERO_CHECK QUE_VEC_MIN_CAP QUE_VEC_GROWTH_NUM QUE_VEC_GROWTH_DEN
```

Generating docs

```bash
 # with std
  cargo run --no-default-features --features repo-tools --bin quelibdump -- --output ./example/dist/lib.json
 # without std
  cargo run --no-default-features --features repo-tools --bin quelibdump -- --split-std --output ./example/dist/lib.json
```

**Disclaimer!**

_This project is a work in progress and might contain bugs! Do NOT use it in production!_

_APIs and behavior may change. New releases can break existing code._

![logo](./footer.svg)
