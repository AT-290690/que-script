# Vectors, tuples, strings, and characters

## Vectors

Vectors are homogeneous mutable collections written with square brackets:

```que
(let xs [10 20 30])
(get xs 1)       ; 20
(set! xs 1 99)
(push! xs 40)
(pop! xs)
```

The basic operations are:

| Form | Meaning |
|---|---|
| `(length xs)` | number of elements |
| `(get xs i)` | element at index `i` |
| `(set! xs i value)` | replace an existing element, or append at `length` |
| `(push! xs value)` | append |
| `(pop! xs)` | remove the final element |
| `(pop-val! xs)` | remove and return the final element |
| `(empty! xs)` | set length to zero while retaining allocated capacity |
| `(car xs)` | first element; equivalent to `(get xs 0)` |
| `(cdr xs)`, `(cdr xs n)` | tail window from offset `1` or explicit offset `n` |

`get` is guarded in debug mode when the compiler cannot prove the access safe.
Optimized code relies on the established invariant, so write an explicit guard
or structure the loop so that the proof is obvious:

```que
(loop i (< i (length xs))
  (println! (get xs i)))
```

`cdr` initially shares the source vector's backing storage and shifts the
logical start index; it does not copy every tail. Mutating that result
materializes an independent vector, so it cannot change the original. See
[Functions by example](../standard-library/by-example.md#car-cdr-and-tail-windows)
for a complete example and the retention trade-off.

## Tuples

Tuples are fixed-size heterogeneous values:

```que
(let person {"Ada" 37 true})
(fst person) ; "Ada"
(snd person) ; 37

(let {name age active?} person)
```

Use tuples for a small fixed record-like value. A tuple has positional fields;
names in a destructuring pattern are local variable names, not stored field
names.

## Strings and characters

Strings are sequences of `Char` values, so their language type is `[Char]`:

```que
(let title "Calque")
(get title 0)      ; 'Q'
(length title)     ; 3
(cons "Hello, " title)
```

Character literals use single quotes. Use `=#`, `<#`, `+#`, and related
operators for character arithmetic and comparison; use `Char->Int` and
`Int->Char` when deliberately converting code points.

The bundled library provides `split`, `split/lines`, `join`, `trim`, `slice`,
`String->Integer`, `Integer->String`, `String/contains?`, `String/starts?`,
and other text functions. Their exact signatures are available through:

```sh
que --lib types 'String/*'
que --lib types 'split*'
```
