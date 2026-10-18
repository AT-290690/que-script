# Calque

**A statically typed Lisp toolchain targeting WebAssembly.**

Calque is a small expression-oriented Lisp with Hindley–Milner type inference,
macros, first-class functions, and explicit mutation. Its compiler optimizes
and lowers typed programs to WebAssembly, and the project provides runtime
integration, the `que` CLI, formatter, language servers, and editor tooling.

Calque is the language and toolchain name. `que` is the compact technical
spelling used for the command, `.que` source files, `que.toml`, and `quelsp`.

The language is designed to let functional and imperative code live together.
Use pipelines, recursion, partial application, and immutable values where they
make code clearer; use mutable locals, vectors, and counted loops where direct
control over performance matters.

```lisp
(let sum
  (lambda (xs)
    (mut total 0)
    (loop i (< i (length xs))
      (alter! total (+ total (get xs i))))
    total))

(|> [1 2 3 4 5]
    (map square)
    sum)
; => 55
```

## Highlights

- Static Hindley–Milner type inference with polymorphic functions
- Lexical closures, recursion, partial application, composition, and macros
- Explicit local and vector mutation without abandoning functional APIs
- `Int` as a WebAssembly `i32` and configurable fixed-point `Dec`
- Vectors, strings, tuples, booleans, characters, functions, and unit
- Runtime reference counting for managed values
- Optimizations for pipelines, scalar helpers, loops, vector access, tuple
  projections, bounds checks, and reference-count operations
- Compilation to expanded source, optimized source, WAT, split WAT, or Wasm
- Wasmtime execution through `que`, plus an optional wasm2c/native-C path
- Capability-gated filesystem, standard-input, terminal, and clock IO
- Native and Wasm language servers, a formatter, Neovim integration, and a
  VS Code extension
- Static diagnostics for unproven bounds, arithmetic safety, empty-vector
  operations, mutation, and suspicious non-termination

## Project status

Calque is pre-1.0 and under active development. The compiler and runtime have a
large automated test suite and are already useful for experimentation,
algorithmic programs, data processing, language research, and portable Wasm
tools. Language and library compatibility may still change between releases,
so pin a known release before using Calque in long-lived or critical systems.

Bug reports and reduced failing programs are especially valuable while the
language approaches a stable compatibility contract.

## Documentation

The repository contains a full mdBook source under [`docs/`](docs/README.md),
covering the language, static analysis, toolchain, standard library, and
worked examples. Preview it locally with:

```bash
cargo install mdbook
mdbook serve docs
```

## Install

Prebuilt releases currently target Linux, macOS, and Windows. Release artifacts
are available from [GitHub Releases](https://github.com/AT-290690/que-script/releases).

### Linux and macOS

Install the `que` CLI and its library:

```bash
curl -fsSL https://raw.githubusercontent.com/AT-290690/que-script/refs/heads/main/scripts/install.sh | bash
```

Optional tools:

```bash
# WAT runner
curl -fsSL https://raw.githubusercontent.com/AT-290690/que-script/refs/heads/main/scripts/install-wat.sh | bash

# Language server
curl -fsSL https://raw.githubusercontent.com/AT-290690/que-script/refs/heads/main/scripts/lsp.sh | bash
```

The default Unix installation uses `/usr/local/bin` and
`/usr/local/share/que` and may request `sudo`.

### Windows

Run these commands from PowerShell:

```powershell
iex ((New-Object Net.WebClient).DownloadString('https://raw.githubusercontent.com/AT-290690/que-script/refs/heads/main/scripts/install.ps1'))
iex ((New-Object Net.WebClient).DownloadString('https://raw.githubusercontent.com/AT-290690/que-script/refs/heads/main/scripts/install-wat.ps1'))
iex ((New-Object Net.WebClient).DownloadString('https://raw.githubusercontent.com/AT-290690/que-script/refs/heads/main/scripts/lsp.ps1'))
```

Windows installs into `%LOCALAPPDATA%\Programs\Que`.

### Editor support

- [Neovim setup](miscs/neovim/README.md)
- [VS Code extension](miscs/extension/que-lang/README.md)

## Quick start

Create a project:

```bash
mkdir hello-que
cd hello-que
que init --demo
```

This creates a `que.toml`, `main.que`, project README, and a small runnable test
example. Run it with:

```bash
que
que test .
```

For a single file:

```bash
que program.que --debug
que program.que --opt
```

`--debug` enables runtime safety guards and reports static-analysis findings.
`--opt` enables aggressive optimizations and disables the optional runtime
integer-overflow, division-by-zero, and vector-bounds guards. Develop with
`--debug`; use `--opt` after the program's invariants are trusted.

Evaluate a short expression without creating a file:

```bash
que --eval '(+ 1 2)'
```

The CLI contains its own concise language guides:

```bash
que --learn
que --examples
que --style
que --pitfalls
```

## Language overview

Everything is an expression. Multiple forms in a function body are sequenced
automatically, and `block` creates a lexical scope when branch-local names are
needed.

```lisp
(let classify
  (lambda (x)
    (cond (< x 0) "negative"
          (= x 0) "zero"
          "positive")))

(let values [3 -1 0 8])
(map classify values)
; => ["positive" "negative" "zero" "positive"]
```

Mutation is explicit:

```lisp
(let increment-all!
  (lambda (xs)
    (loop i (< i (length xs))
      (set! xs i (+ (get xs i) 1)))
    xs))

(increment-all! [10 20 30])
; => [11 21 31]
```

Comments begin with `;`. Strings use double quotes and character literals use
single quotes.

```lisp
; a comment
(let greeting "hello")
(let newline '\n')
```

The main concrete types are:

```text
Int                 signed 32-bit integer
Dec                 fixed-point decimal backed by i32
Bool                true or false
Char                Unicode scalar value
()                  unit / nil
[T]                 vector of T; strings are [Char]
{A B}               tuple
A -> B              function
```

`Dec` uses a scaled signed `i32` at runtime (scale `1000` by default, selected
with `QUE_DECIMAL_SCALE` when compiling). Source literals are parsed as exact
base-10 values and quantized with round-half-even; they never pass through a
binary floating-point representation. Decimal multiplication and division use
`i64` intermediates and truncate their result toward zero at the configured
scale.

Use `sig` when an external boundary or polymorphic operation needs an explicit
type:

```lisp
(sig parse/int! ([Char] -> Int))
(let parse/int! (lambda (text) (deserialize text)))
```

### Closed enums and exhaustive matching

For a small fixed domain, declare zero-payload tags with `enum`. Variants have
their declared enum type, not `Int`, and `match` requires every variant unless
its final branch is `_`:

```lisp
(enum Letter A B C)

(sig score (Letter -> Int))
(let score (lambda letter
  (match letter
    A 1
    B 3
    C 4)))
```

Use `=~` to compare two values from the same enum. Comparing values from
different enums, or an enum and an `Int`, is a type error:

```lisp
(=~ A B) ; false
```

These first-class enums are deliberately compact: each value lowers to an
`i32` tag. Payload variants can be added later without changing this syntax.
The former string-dispatch macro is available explicitly as `match/string`.

Enums are also valid `Table` keys. The table keeps its normal structural
`[Char]` key support, while enum keys use their compact tag for hashing and
equality—no string conversion or allocation is involved:

```lisp
(sig points [[{ Letter Int }]])
(let points (Table/new))
(Table/set! points A 1)
(Table/set! points B 3)
```

## IO and permissions

IO is exposed through typed host imports and denied unless the matching
capability is granted:

```lisp
(let input (read! "input.txt"))
(println! input)
```

```bash
que program.que --allow read print
```

Available permission groups are:

```text
read  stdin  write  print  clock  delete  all
```

Large files and standard input can be processed incrementally with
`read/chunks!`, `read/lines!`, and `stdin/chunks!`.

See [FFI.md](FFI.md) for custom Wasm host imports and the current ABI.

## Projects and tests

A project is configured by `que.toml`:

```toml
entry = "main.que"
deps = [
  "./lib/math.que",
  "./lib/text.que",
]
```

Running `que` uses the configured entry. Native CLI and LSP processes discover
the nearest `que.toml` and load its dependencies.

Folder tests append `main.test.que` after the project entry, allowing tests to
call project definitions directly:

```bash
que test .
que test path/to/example.test.que
```

## Compiler output

Inspect every important compilation stage:

```bash
que program.que --emit source --out expanded.que
que program.que --opt --emit opt-source --out optimized.que
que program.que --opt --emit wat --out program.wat
que program.que --opt --emit wasm --out program.wasm
que program.que --emit types
```

`split-wat` emits a reusable runtime module and a user module that imports it.

Use `explain` for an optimization and correctness report:

```bash
que explain program.que --opt
que explain program.que --opt --json
```

The report separates correctness warnings, termination reasoning, effects and
permissions, performance observations, and generated-code details.

## Formatting and editor workflow

Format files from the CLI:

```bash
que fmt program.que
que fmt program.que --check
que fmt --stdin
```

Open the configured Calque Neovim scratch environment with:

```bash
que nvim
que nvim --code "$(cat program.que)"
```

The Neovim plugin provides LSP integration, formatting, hover and signature
help, completion, and shortcuts for running, debugging, explaining, and viewing
generated output. `<Space>g` (or `:QueLib`) opens a fuzzy browser for library
type signatures; patterns such as `map*` and names ending in `!` are passed
directly to `que --lib` without shell escaping.

## Native C output

Calque programs compile to standard Wasm and can be run by any compatible host.
For native experiments, the repository also contains a wasm2c host adapter:

```bash
./scripts/compile-native-c.sh program.que build/native
./build/native/main --allow read write print -- argument
```

This requires WABT's `wasm2c`. The maintained C host implementation lives in
[`miscs/native-c`](miscs/native-c/README.md).

## Lightweight compiler and native-C path

`que` is the single user-facing executable. It provides execution, compilation,
emitted source/WAT/Wasm/C/types, formatting, explanations, and optional native
execution without embedding a WebAssembly runtime:

```bash
que program.que --opt
que compile program.que --out program.wasm
que wat program.que > program.wat
que --eval "(+ 1 2)"
que --lib names "*map*"
que --lib types map
que --env
que explain program.que
que fmt program.que
```

Normal `que program.que` execution uses a runtime installed by the user. Calque
neither embeds nor installs a runtime. The optional native-C experiment remains
available as `./scripts/compile-native-c.sh`; it is not part of the installed
CLI.

The external-Wasmtime backend can also be invoked explicitly with:

```bash
que run-wasi program.que --opt                         # Wasmtime (default)
que run-wasi program.que --opt --runtime wasmer
que run-wasi program.que --opt --runtime iwasm
```

It lowers Calque IO, arguments, filesystem operations, chunked input, and concrete
serialization/deserialization to WASI. Use `--allow` to grant the corresponding
capabilities. Install one of Wasmtime, Wasmer, or WAMR separately and make its
CLI available on `PATH`. Wasmtime is the default; choose another with
`--runtime` or set `QUE_WASM_RUNTIME`. A path to a supported runtime executable
is accepted too.

## Build from source

Requirements:

- A current stable Rust toolchain
- A WASI runtime for executing Calque programs (Wasmtime, Wasmer, or WAMR)
- Wasmtime CLI specifically for running the repository's runtime tests
- Node.js and npm for building the VS Code extension
- Optional: WABT for WAT and native-C workflows
- Optional: Zig and `cargo-zigbuild` for cross-compilation scripts

Build the local toolchain:

```bash
./scripts/build-all.sh
```

Run the test suite:

```bash
./scripts/install-test-deps.sh # once, if wasmtime is not installed
cargo test
./miscs/native-c/test.sh
```

Run the cross-target differential corpus:

```bash
./scripts/test-differential.sh
```

This generates a deterministic set of Calque programs and checks that `--debug`,
`--opt`, and the `wasm2c`/native-C path have the same observable result. It
also checks that a program which fails at runtime fails on every target. The
runner needs Wasmtime, WABT's `wasm2c`, and `clang` or `cc`, so it remains
separate from the normal Rust test suite.

Build all configured release artifacts:

```bash
./scripts/build-everything.sh
```

The installed binaries are:

```text
que      compiler, tooling, and user-selected WASI runtime
quelsp   language server (installed separately with editor tooling)
```

## Repository layout

```text
eclisp/             parser and type system
lisp/               bundled language libraries and macros
src/                optimizer, Wasm compiler, runtime, IO host, LSP, analysis
miscs/neovim/       Neovim integration
miscs/extension/    VS Code extension
miscs/native-c/     optional wasm2c host
examples/           example Calque programs
scripts/            build, install, and release helpers
```

## License

Calque is available under the [MIT License](LICENSE).
