# Calque documentation

This directory is an [mdBook](https://rust-lang.github.io/mdBook/) for Calque,
a statically typed Lisp and WebAssembly toolchain.

```sh
cargo install mdbook
mdbook serve docs
```

Open the local URL printed by mdBook. Build the static site with:

```sh
mdbook build docs
```

The generated `docs/book/` directory is intentionally ignored; publish that
directory with GitHub Pages, Cloudflare Pages, or another static host.

The compiler and bundled library are the source of truth. When changing a
language feature or exported library API, update the relevant page here and
validate the examples with `que --debug` and `que --opt`.
