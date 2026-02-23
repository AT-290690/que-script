#!/bin/bash
cargo build --release --no-default-features --features parser,js-compiler,ocaml-compiler,rust-compiler,python-compiler,kotlin-compiler,wasm-compiler,deref-wasm,type-ast,type-checker,vm,repl
