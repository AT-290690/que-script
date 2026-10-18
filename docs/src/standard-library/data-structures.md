# Tables, sets, heaps, and queues

## Tables

`Table/new` creates a mutable hash table. Its public operations use
collection-first mutation and data-last lookup/predicate forms:

```que
(let scores (Table/new))
(Table/set! scores "Ada" 10)
(Table/set! scores "Lin" 20)

(Table/has? "Ada" scores)
(Table/get "Ada" scores)
```

`Table/get` returns the table’s entry-vector form. Use `Table/get*` for a
checked result or `Table/get-unsafe` when presence is already established:

```que
(let {found? values} (Table/get* "Ada" scores))
(if found? (get values 0) 0)

(if (Table/has? "Ada" scores)
    (Table/get-unsafe "Ada" scores)
    0)
```

`Table/update!` and `Table/update-or!` cover common count/accumulator patterns;
`Table/entries`, `Table/keys`, `Table/values`, and `Table/count` expose its
contents. Tables support their established key representations, including
strings and closed enum tags. Use `Table/new/capacity` when an approximate
entry count is known, and `Table/compact!` to rebuild after substantial
deletion.

## Sets

`Set/new`, `Set/add!`, `Set/remove!`, `Set/has?`, `Set/values`, and set algebra
are provided for the library’s set representation. As with tables, query the
specific function type for the accepted key kind:

```sh
que --lib types 'Set/*'
```

## Heap and deque

`Heap/*` implements a mutable binary heap using a caller-supplied comparison
predicate. `Que/*` provides a deque-like structure backed by vectors, with
operations such as `Que/push!`, `Que/prepend!`, `Que/first`, `Que/last`, and
`Que/deque!`.

These structures favor explicit mutation and predictable representation over
hidden GC behavior. Use the public namespace functions rather than relying on
their internal vector layout.

## Matrices

`Matrix/new`, `Matrix/in-bounds?`, `Matrix/for/i`, and related helpers cover
nested-vector matrix work. Matrix coordinates are explicit `Int` values; use
the bounds helper before dynamic access when static analysis cannot prove an
index safe.
