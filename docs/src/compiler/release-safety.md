# Debug, optimized, and native execution

Calque has one source language and one type checker. The execution mode changes
which runtime guards remain in generated code and which runtime executes the
result; it does not loosen type checking, make reference cycles valid, or turn
warnings into type errors.

## Development: `--debug`

Use debug mode while writing and testing a program:

```sh
que program.que --debug --allow read print
```

It emits static-analysis findings and keeps configured runtime checks for facts
the compiler could not prove, including vector bounds, integer arithmetic, and
division by zero. A debug trap is evidence that an execution violated a Calque
invariant. Fix the input validation or source invariant rather than treating a
trap as an expected control-flow mechanism.

## Release: `--opt`

Use optimized mode only after exercising the program in debug mode and reading
relevant warnings:

```sh
que explain program.que --opt
que program.que --opt --allow read
```

`--opt` enables source and Wasm optimizations and omits optional Calque runtime
guards. It uses static proofs when available, but it also assumes that source
invariants you chose not to guard are true. For example, an unguarded dynamic
`get` can be unchecked in optimized code.

WebAssembly still bounds every access to its linear memory. An access outside
linear memory traps and cannot escape the guest sandbox. That is not the same
as a valid vector access: an incorrect unchecked index that remains inside
linear memory can read or overwrite unrelated Calque runtime data. The result
may be wrong or may later fail due to corrupted Calque-level state. Treat
`--opt` as release performance mode, not as permission to ignore correctness.

## Native C path

The repository's optional native path first compiles the same typed Calque
program to Wasm, then translates that Wasm through `wasm2c` and a C compiler:

```sh
./scripts/compile-native-c.sh program.que
```

It is useful for finished compute-heavy programs, low RSS, and native
benchmarking. It is not a separate compiler, a different ownership model, or a
replacement for debug validation. Compare its output with both debug and
optimized WASI runs before relying on it for a workload.

## Recommended release checklist

1. Run representative inputs with `--debug`.
2. Address warnings, or make the invariant visible with a guard.
3. Inspect `que explain program.que --opt` for hot-path bounds checks,
   allocations, and unknown calls.
4. Compare debug and optimized output on representative inputs.
5. Measure complete optimized runs; compare the native path only when it is a
   realistic deployment option.
6. Pin the Calque version used for a long-lived project or reproducible build.

If debug, optimized, or native output differs for the same valid input, reduce
the program and report it as a compiler bug. See the [debugging
cookbook](debugging.md).
