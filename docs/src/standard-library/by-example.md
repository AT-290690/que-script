# Functions by example

This is the practical first stop for the bundled library: short examples of
the functions you will use in normal programs. The [complete library
index](index.md) covers every exported family, including low-level helpers.

Functions that transform a collection usually take that collection last, so
they compose naturally with `|>`. Operations ending in `!` mutate the first
collection argument and normally return `nil`.

```que
(|> [1 2 3 4]
    (map (lambda (x) (* x x)))
    (filter (lambda (x) (> x 4)))
    sum)
; 25
```

## Core vectors

Vectors are homogeneous and mutable. Strings are vectors of `Char`, so the
same basic read operations work on both.

```que
(let xs [10 20 30])

(length xs)             ; 3
(get xs 1)              ; 20
(car xs)                ; 10 — first element
(cdr xs)                ; [20 30] — tail from offset 1
(cdr xs 2)              ; [30] — tail from an explicit offset
(first xs)              ; 10
(last xs)               ; 30
(at xs -1)              ; 30 — negative indexing helper
(cons [0] xs)           ; [0 10 20 30] — concatenate two vectors
```

`get`, `car`, `first`, `last`, and `pop-val!` require a valid, non-empty
input. For a dynamic index, use `in-bounds?`, `Vector/get*`, or structure the
loop so the analyser can prove the access.

```que
(if (in-bounds? xs index)
    (get xs index)
    fallback)

(let {found? values} (Vector/get* index xs))
(if found? (get values 0) fallback)
```

Mutation is direct and explicit:

```que
(let xs [10 20])
(set! xs 0 99)          ; [99 20]
(push! xs 30)           ; append, xs is now [99 20 30]
(let last-value (pop-val! xs))
(pop! xs)               ; remove the last value and return nil
(empty! xs)             ; length becomes zero; allocation remains available
```

`capacity!` reserves storage for later appends without changing length. It is
effect-only. `capacity` is the non-mutating version: it copies first, reserves
the copy, and returns it. `compact!` discards unused vector capacity.

```que
(let out [])
(capacity! out 1000)
(loop i (< i 1000) (push! out i))

(let copy-with-room (capacity 1000 out))
(compact! out)
```

Useful names in this family are `Vector/new`, `Vector/of`, `Vector/first`,
`Vector/last`, `Vector/at!`, `Vector/get-unsafe`, `Vector/set!`,
`Vector/push!`, `Vector/pop!`, `Vector/pull!`, `Vector/reverse!`,
`Vector/swap!`, `Vector/sort!`, `Vector/concat`, and `Vector/cons`. Their
namespace forms are useful when an explicit, regular API is clearer than the
short core spelling.

## Build and reshape sequences

`range` includes both endpoints. Use `range/exclusive` when the end should be
excluded; `range/dec` creates decimal values.

```que
(range 3 6)             ; [3 4 5 6]
(range/exclusive 3 6)   ; [3 4 5]
(range/dec 1 3)         ; [1.0 2.0 3.0]

(replicate 3 "go")     ; ["go" "go" "go"]
(fill 4 (lambda (i) (* i i)))
(zeroes 4)              ; [0 0 0 0]
(ones 4)                ; [1 1 1 1]
```

The common shape operations return fresh vectors:

```que
(reverse [1 2 3])                       ; [3 2 1]
(flat [[1 2] [3] [4 5]])                ; [1 2 3 4 5]
(flat-map (lambda (x) [x (* x 10)]) [1 2])
; [1 10 2 20]

(zip {[1 2] ["a" "b"]})                ; [{1 "a"} {2 "b"}]
(unzip [{1 "a"} {2 "b"}])              ; {[1 2] ["a" "b"]}
(zip-with + [1 2] [10 20])               ; [11 22]
(interleave [1 2 3] [10 20 30])          ; [1 10 2 20 3 30]
(intersperse 0 [1 2 3])                  ; [1 0 2 0 3]
```

For a contiguous part of a vector, `slice` uses an exclusive end. The
`take/*` and `drop/*` names describe which side is retained or removed.

```que
(slice 1 4 [0 1 2 3 4])                 ; [1 2 3]
(take/first 3 [0 1 2 3])                ; [0 1 2]
(take/last 2 [0 1 2 3])                 ; [2 3]
(drop/first 2 [0 1 2 3])                ; [2 3]
(drop/last 2 [0 1 2 3])                 ; [0 1]
(partition 2 [1 2 3 4 5])               ; [[1 2] [3 4] [5]]
(window 3 [1 2 3 4])                    ; [[1 2 3] [2 3 4]]
```

`take/while` retains the initial matching prefix; `drop/while` removes that
prefix. `cycle`, `subset`, `permutation`, `combination`, and `combination/n`
are combinatoric builders. `transpose` turns rows into columns; `Matrix/new`
and `Matrix/for/i` are the more direct matrix tools.

## Map, filter, visit, and fold

`map` transforms every element. The `/i` forms pass `(value index)`. `filter`
and its alias `select` retain matching values; `exclude` removes them.

```que
(map (lambda (x) (* x 2)) [1 2 3])
; [2 4 6]

(map/i (lambda (x i) {i x}) ["a" "b"])
; [{0 "a"} {1 "b"}]

(filter even? [1 2 3 4])
; [2 4]

(exclude even? [1 2 3 4])
; [1 3]
```

`for` visits for effects and returns `nil`. `each` performs the same visit but
returns the original vector, which is useful in a pipeline. Their `/i` forms
also receive an index.

```que
(let seen [])
(for (lambda (x) (push! seen x)) [1 2 3])
; seen is [1 2 3]

(|> [1 2 3]
    (each (lambda (x) (println! x)))
    reverse)
; [3 2 1]
```

`reduce` combines a vector from left to right with an explicit initial value.
`reduce/i` also supplies the index. `scan` returns all running reductions.

```que
(reduce + 0 [10 20 30])                 ; 60
(reduce/i (lambda (total x i) (+ total (* x i))) 0 [2 3 4])
; 11
(scan + [1 2 3 4])                      ; [1 3 6 10]
```

The `/until` variants stop before the first value whose stop predicate is
true. They exist for `map`, `reduce`, and `for`, with index-aware `/i` forms.

```que
(map/until (lambda (x) (* x 2)) (lambda (x) (> x 3)) [1 2 3 4])
; [2 4 6]
```

## Search, tests, and aggregates

```que
(find (lambda (x) (> x 10)) [3 7 12 9]) ; 2, or -1 when absent
(count odd? [1 2 3 4])                  ; 2
(some? odd? [2 4 5])                    ; true
(every? positive? [1 2 3])              ; true
(some/i? (lambda (x i) (= x i)) [0 9]) ; true

(sum [1 2 3])                           ; 6
(product [2 3 4])                       ; 24
(minimum [8 2 5])                       ; 2
(maximum [8 2 5])                       ; 8
(mean [2 4 6])                          ; 4
(median [9 1 5])                        ; 5
(avg 5 8)                               ; 6 — integer average
```

Decimal equivalents use `/dec`, `.` suffixes, or the matching decimal
operator: `sum/dec`, `product/dec`, `minimum/dec`, `maximum/dec`,
`mean/dec`, `median/dec`, `avg/dec`, `min.`, and `max.`. `count/int`,
`count/char`, `count/dec`, and `count/bool` count a particular value using the
appropriate equality operation.

## Ordering and grouping

`sort` returns a sorted copy. `Vector/sort!` sorts the supplied vector in
place. A comparator returns true when its first argument belongs before its
second.

```que
(sort (lambda (a b) (< a b)) [5 1 4 2])
; [1 2 4 5]

(let xs [5 1 4 2])
(Vector/sort! xs (lambda (a b) (< a b)))
; xs is [1 2 4 5]
```

`unique/int` and `unique/char` remove repeated values for their respective
element types. `group` builds a table keyed by a function result:

```que
(group (lambda (x) (% x 2)) [1 2 3 4])
; table with key 1 -> [1 3], key 0 -> [2 4]
```

## Text and characters

Strings have type `[Char]`. `split` takes the separator first and text last;
`join` takes the separator first and a vector of strings last.

```que
(split "," "red,green,blue")             ; ["red" "green" "blue"]
(split/lines "one\ntwo")                  ; ["one" "two"]
(split/words "one two")                   ; ["one" "two"]
(join "/" ["usr" "local" "bin"])        ; "usr/local/bin"
(trim "  Calque  ")                       ; "Calque"
(slice 1 4 "Calque")                      ; "alq"
```

Use `lower`, `upper`, `digit?`, `space?`, and the `String/*` predicates for
text logic:

```que
(lower 'Q')                               ; 'q'
(upper 'q')                               ; 'Q'
(digit? '7')                              ; true
(String/starts? "Cal" "Calque")           ; true
(String/contains? "lq" "Calque")          ; true
(String/ends? "que" "Calque")             ; true
(String/lt? "Ada" "Lin")                  ; true
```

Conversions are explicit:

```que
(String->Integer "42")                    ; 42
(Integer->String 42)                      ; "42"
(String->Dec "3.5")                       ; 3.5
(Dec->String 3.5)                         ; "3.5"
(Char->Int 'A')                           ; 65
(Int->Char 65)                            ; 'A'
```

Other useful text functions are `String/find`, `String/find/last`,
`String/matches-at?`, `String/damerau-levenshtein`, `String/quote`,
`String/dquote`, and `String/prepend-zeroes`. `match?` compares two character
vectors exactly.

## Numeric helpers

Normal `Int` arithmetic is signed `i32`; decimals use `Dec` and dotted
operators. The predicate variants let a program establish a safety condition
before using arithmetic in an optimized program.

```que
(abs -9)                                  ; 9
(square 12)                               ; 144
(gcd 18 24)                               ; 6
(lcm 6 8)                                 ; 24
(clamp 10 14)                             ; 10
(clamp-range 0 10 -4)                     ; 0
(even? 8)                                 ; true
(odd? 9)                                  ; true
(divisible? 3 12)                         ; true
(expt 3 2)                                ; 8 — exponent first
(sqrt 81)                                 ; 9
```

Use `positive?`, `negative?`, `zero?`, `one?`, `negative-one?`, `invert`,
`delta`, `min`, `max`, `floor`, `ceil`, and `Int/euclidean-mod` for the
corresponding small operations. Decimal versions include `abs.`, `sqrt.`,
`expt.`, `clamp/dec`, `delta/dec`, and `/dec?` predicates. `Integer->Bits`,
`Bits->Integer`, `Integer->Digits`, `Digits->Integer`, and their character
versions convert representations. `random/int` and `random/dec` are
deterministic seed-to-value functions.

## Tables, sets, heaps, deques, and matrices

Tables are mutable hash maps. Put the table first for mutation; use the key
first for lookup and membership so those work in a pipeline.

```que
(let scores (Table/new))
(Table/set! scores "Ada" 10)
(Table/update-or! scores "Ada" (lambda () 0) (lambda (n) (+ n 1)))

(Table/has? "Ada" scores)                ; true
(Table/get-unsafe "Ada" scores)          ; 11
(Table/entries scores)                    ; [{"Ada" 11}]
(Table/keys scores)                       ; ["Ada"]
(Table/values scores)                     ; [11]
```

`Table/get` returns the internal matching entry vector; prefer `Table/get*`
for a `{found? values}` result or `Table/get-unsafe` after a `Table/has?`
check. `Table/remove!`, `Table/drop!`, `Table/merge!`, `Table/resize!`, and
`Table/compact!` mutate; `Table/keep`, `Table/omit`, and `Table/merge` return
new tables. `Table/count` and `Table/frequency` build counting tables.

```que
(let tags (Set/new))
(Set/add! tags "fast")
(Set/add! tags "typed")
(Set/has? "fast" tags)                   ; true
(Set/remove! tags "fast")
(Set/values tags)                         ; ["typed"]
```

Sets provide `intersection`, `union`, `difference`, and `xor`, as well as
`Set/new/capacity`, `Set/resize!`, and `Set/compact!` for allocation control.

```que
(let heap [])
(let before? (lambda (a b) (< a b)))
(Heap/push! heap 4 before?)
(Heap/push! heap 1 before?)
(Heap/peek! heap)                         ; 1
(Heap/pop! heap before?)
```

`Que/new` creates a deque with a default value. Use `Que/append!`/`Que/push!`
and `Que/prepend!` to add at either end; `Que/pop!` and `Que/tail!` remove;
`Que/first`, `Que/last`, `Que/peek`, `Que/get`, and `Que/at` read it.

```que
(let matrix (Matrix/new (lambda (y x) (+ (* y 10) x)) 3 2))
; [[0 1] [10 11] [20 21]]
(Matrix/in-bounds? 1 2 matrix)            ; true
```

## Big integers, dates, metadata, and options

`BigInt/*` represents an unsigned arbitrary-precision integer as a digit
vector; `SignedBigInt/*` uses `{Bool [Int]}`. Construct them from text and
stay within the corresponding namespace for arithmetic:

```que
(let a (BigInt/new "999999999999999999"))
(Digits->Chars (BigInt/add a (BigInt/new "1")))
; "1000000000000000000"
```

`Date/iso->Vector`, `Date/iso-z->Vector`, `Date/year`, `Date/month`,
`Date/day`, and `Date/hour` work with the library's date-vector representation.
`Meta/parse`, `Meta/table`, `Meta/get`, and `Meta/has?` support simple
metadata tokens. `true/option`, `false/option`, and `resolve/option` use the
ordinary tuple representation `{Bool value}` for success-or-default flows.

## Discover anything not shown here

The installed compiler is the exact reference for the library bundled with
that installation. Search by shell-style glob and inspect a definition when
needed:

```sh
que --lib names 'Table/*'
que --lib types 'map*'
que --lib source Table/update-or!
```

The [complete library index](index.md) explains how the remaining specialist
and implementation-level names are organized.
