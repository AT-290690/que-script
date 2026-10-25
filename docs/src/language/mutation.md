# Mutation and capacity

Mutation is explicit in both spelling and type/effect analysis. Use it for
local counters, in-place algorithms, and predictable allocation behavior.

## Locals and vectors

```que
(mut total 0)
(loop i (< i 10)
  (+= total i))

(let xs [])
(push! total xs)
(set! 0 42 xs)
```

The caller-visible mutable value of a function is normally its final
parameter. This keeps mutation APIs pipeline-friendly:

```que
(let fill-zeroes!
  (lambda (xs)
    (loop i (< i (length xs))
      (set! i 0 xs))
    xs))
```

## Reserving vector storage

`capacity!` reserves backing storage without changing a vector’s length:

```que
(let paths [])
(capacity! 1000 paths)

(loop i (< i 1000)
  (push! i paths))
```

It is useful before a known burst of appends. It returns unit, deliberately:
the vector itself remains the same mutable value. `empty!` similarly resets
the vector’s length while retaining storage for reuse.

```que
(empty! paths)
; paths can be refilled without reallocating until it grows past capacity
```

`capacity!` is a reservation, not a maximum length. Appending more than the
requested capacity remains valid and grows the vector as needed.

## Safe, fast code

For debug builds, Calque can trap unproven bad indices and checked arithmetic.
For optimized builds, organize loops so `que explain` can see basic facts such
as `0 <= i < length(xs)`. This lets the compiler remove checks where safe,
while the source retains its clear invariant.
