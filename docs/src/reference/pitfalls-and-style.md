# Pitfalls and style

## Develop safely, release deliberately

Use `--debug` while building and testing. It enables configured runtime guards
and emits static-analysis findings. Use `--opt` once your program’s bounds and
arithmetic invariants are established, then benchmark the real workload.

## Common pitfalls

- `do` sequences but does not introduce lexical scope; use `block` for local
  branch/loop bindings.
- `get`, `car`, `pop!`, and `pop-val!` require a non-empty/in-bounds vector.
  In optimized code, prove or guard that fact yourself.
- `Int` is signed `i32`; large addition, multiplication, and division deserve
  an explicit invariant or a `*/safe?`, `+/safe?`, or `/safe?` guard.
- A missing function argument is partial application, not necessarily an
  immediate error.
- `deserialize` needs a concrete expected result type; put `sig` immediately
  before its binding.
- Use `=~` rather than `=` for values from an enum.
- `Table/get` exposes an entry-vector form. Prefer `Table/get*` or
  `Table/get-unsafe` after `Table/has?` if you want a value.

## Style

- Prefer pipelines for a readable data transformation.
- Prefer explicit loops and mutation for a hot, allocation-sensitive kernel.
- Cache repeated lengths or computed values in hot loops.
- Use names ending in `!` for mutating/effectful user functions.
- Put data last in transform-style helpers and collection first in mutators.
- Start a performance-sensitive file with `; use-strict-warnings!` and inspect
  `que explain` before treating an optimization as established.
- Keep a reduced program when you find a compiler bug; debug/opt/native target
  agreement is part of the compiler’s correctness contract.
