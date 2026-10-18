# Eclisp

Eclisp is the language core used by Calque.

This folder is intentionally small and standalone. It contains:

- parser and syntax normalization
- macro/desugar support currently implemented in the parser layer
- core types
- Hindley-Milner type inference
- effect inference and validation that is part of type checking
- extern declarations and native `sig` signatures

It does not contain:

- Calque standard library source
- optimizer
- WebAssembly/WAT compiler
- runtime
- CLI
- LSP
- release scripts

## Use

```bash
cargo test
```

Minimal Rust API:

```rust
let (typ, typed_ast) = eclisp::check("(let add (lambda (a b) (+ a b))) (add 1 2)")?;
```

The crate has no external dependencies.
