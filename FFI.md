# Calque FFI

Calque’s current foreign-function interface is a typed **WebAssembly host-import
FFI**. The complete, maintained guide is in the mdBook source:

- [FFI and host imports](docs/src/compiler/ffi.md)

In short:

```lisp
(extern env add_one add-one! (Int -> Int))
(add-one! 41)
```

The generated Wasm imports `env.add_one`; an embedding host must supply it.
Extern declarations are top-level, use concrete types, require a local name
ending in `!`, and are treated as effectful.

The default `que` command uses an external WASI runtime. It provides Calque’s
built-in IO imports through its WASI adapter, but it does not load arbitrary
user host modules from CLI configuration. Custom imports therefore require an
embedding host.

This is not a C ABI, generic plugin loader, callback system, or async runtime.
Managed Calque values use an internal linear-memory representation; hosts that
interoperate with vectors, tuples, or strings must respect that runtime ABI.
