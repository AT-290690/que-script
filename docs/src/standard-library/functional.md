# Functional collections

The central functional transforms are data-last.

```que
(|> [1 2 3 4 5]
    (map (lambda (x) (* x x)))
    (select (lambda (x) (> x 10)))
    sum)
```

## Transform and traverse

| Function | Role |
|---|---|
| `map` / `map/i` | build a new vector; `/i` receives the index |
| `pairwise` | build overlapping adjacent `{left right}` tuples |
| `map/adjacent` | transform each neighboring pair directly |
| `filter`, `select`, `filter/i` | retain items satisfying a predicate |
| `each` / `each/i` | visit items and return the original vector |
| `for` / `for/i` | visit items for effects |
| `reduce` / `reduce/i` | fold with an explicit initial accumulator |
| `map/until`, `reduce/until`, `for/until` | stop according to a predicate |

```que
(reduce (lambda (total x) (+ total x)) 0 [10 20 30])
; 60

(map/i (lambda (x index) {index x}) ["a" "b"])
```

`pairwise` is data-last and returns one fewer tuple than its input. It is
empty for inputs shorter than two elements.

```que
(|> [1 4 9 16]
    pairwise
    (map (lambda ({left right}) (- right left))))
; [3 5 7]
```

## Construct and order

`range`, `range/inclusive`, and related helpers construct numeric sequences.
`sort` returns a sorted copy; `Vector/sort!` sorts a vector in place. Use a
comparison predicate that expresses the desired ordering.

```que
(sort (lambda (left right) (< left right)) [5 1 4 2])
```

For allocation-sensitive code, prefer explicit loops and a reserved output
vector when the result length is known. Pipeline fusion may optimize common
map/filter/reduce shapes, but clear source invariants remain the first goal.

## Numeric aggregates

The library has aggregate and statistical helpers including `sum`, `product`,
`avg`, `mean`, `median`, `minimum`, and `maximum`, with integer and decimal
variants where relevant. Query the exact expected element type rather than
relying on a name alone:

```sh
que --lib types 'sum*'
que --lib types 'avg*'
```
