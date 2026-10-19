# Complete library index

Calque ships a large library, including general-purpose functions, specialist
algorithms, compatibility aliases, and low-level building blocks used to
implement the higher-level APIs. The normal public starting points are shown
with runnable code in [Functions by example](by-example.md).

This page tells you where every exported name belongs. The compiler installed
on your machine is the authoritative source for exact signatures and source:

```sh
que --lib names '*'
que --lib types 'Table/*'
que --lib source Table/update-or!
```

## Core language operations

The compiler provides vector and tuple primitives: `length`, `get`, `set!`,
`push!`, `pop!`, `pop-val!`, `empty!`, `capacity!`, `car`, `cdr`, `cons`,
`fst`, `snd`, `box`, and `swap!`. It also provides arithmetic,
comparison, bitwise, boolean, character, decimal, I/O, serialization, and
filesystem primitives. These are described in the language chapters rather
than duplicated as library implementations.

Macros include `cond`, `when`, `when-not`, `unless`, `|>`, `<|`, `comp`,
`apply`, `loop`, `loop/for`, `loop/in`, `loop/in/i`, `times`, mutation sugar
such as `++` and `+=`, and compile-time helpers such as `test`.

## Functional and vector helpers

The unnamespaced functional collection surface is in `lisp/fp.lisp`:

- Construction and shape: `range`, `range/inclusive`, `range/exclusive`,
  `range/dec`, `fill`, `replicate`, `copy`, `reverse`, `flat`, `flat-map`,
  `zip`, `unzip`, `zip-with`, `transpose`, `interleave`, `intersperse`,
  `partition`, `window`, `slice`, `take/first`, `take/last`, `drop/first`,
  `drop/last`, `take/while`, `drop/while`, `cycle`, `subset`,
  `permutation`, `combination`, `combination/n`, and `scan`.
- Traversal: `map`, `map/i`, `filter`, `filter/i`, `select`, `exclude`,
  `for`, `for/i`, `each`, `each/i`, `reduce`, `reduce/i`, `map/until`,
  `map/until/i`, `reduce/until`, `reduce/until/i`, `for/until`, and
  `for/until/i`.
- Queries and aggregation: `empty?`, `not-empty?`, `in-bounds?`, `find`,
  `count`, `count/int`, `count/char`, `count/dec`, `count/bool`, `some?`,
  `some/i?`, `every?`, `every/i?`, `sum`, `sum/dec`, `product`,
  `product/dec`, `minimum`, `minimum/dec`, `maximum`, `maximum/dec`, `mean`,
  `mean/dec`, `median`, `median/dec`, `avg`, and `avg/dec`.
- Ordering and specialized operations: `sort`, `group`, `unique/int`,
  `unique/char`, `map/adjacent`, `map/tuple`, `map/fst`, `map/snd`,
  `resolve/option`, `autocorrect`, `match?`, `cartesian-product`, and
  `neighborhood`.

The `Vector/*` namespace supplies regular versions and lower-level helpers:
`Vector/new`, `Vector/of`, `Vector/first`, `Vector/last`, `Vector/at!`,
`Vector/get*`, `Vector/get-unsafe`, `Vector/in-bounds?`, `Vector/set!`,
`Vector/push!`, `Vector/pop!`, `Vector/pull!`, `Vector/append!`,
`Vector/concat`, `Vector/cons`, `Vector/reverse!`, `Vector/sort!`, and
`Vector/swap!`. Names ending in `/raw` are implementation-level variants;
prefer the non-raw name unless you are extending the library itself.

## Text, numbers, and conversion

- Text: `split`, `split/lines`, `split/words`, `split/commas`, `join`,
  `trim`, `trim/left`, `trim/right`, `lower`, `upper`, `digit?`, `space?`,
  `String/starts?`, `String/ends?`, `String/contains?`, `String/equal?`,
  `String/lt?`, `String/lte?`, `String/gt?`, `String/gte?`, `String/find`,
  `String/find/last`, `String/matches-at?`, `String/damerau-levenshtein`,
  `String/quote`, `String/dquote`, and `String/prepend-zeroes`.
- Conversions: `String->Integer`, `Integer->String`, `String->Dec`,
  `Dec->String`, `String->Bool`, `String->Vector`, `Char->Int`, `Int->Char`,
  `Char->Bool`, `Bool->Char`, `Bool->Int`, `Int->Bool`, `Int->Dec`,
  `Dec->Int`, `Integer->Bits`, `Bits->Integer`, `Integer->Digits`,
  `Digits->Integer`, `Digits->Chars`, `Chars->Digits`, `Chars->Integer`, and
  their `/base` or `/dec` variants.
- Integer math: `abs`, `square`, `gcd`, `lcm`, `min`, `max`, `clamp`,
  `clamp-range`, `sqrt`, `expt`, `floor`, `ceil`, `delta`, `divisible?`,
  `positive?`, `negative?`, `zero?`, `one?`, `negative-one?`, `even?`,
  `odd?`, `invert`, and `Int/euclidean-mod`.
- Decimal math mirrors the applicable integer functions with `.` or `/dec`
  spellings. Safety predicates are `+/safe?`, `-/safe?`, `*/safe?`,
  `/safe?`, `%/safe?`, plus decimal equivalents such as `+./safe?`.
- Deterministic pseudo-random helpers are `random`, `random/int`, `random.`,
  and `random/dec`; mathematical constants live under `const/dec/*`.

## Structured data

- Tables: `Table/new`, `Table/new/capacity`, `Table/set!`, `Table/get`,
  `Table/get*`, `Table/get-unsafe`, `Table/has?`, `Table/remove!`,
  `Table/drop!`, `Table/keep`, `Table/omit`, `Table/entries`, `Table/keys`,
  `Table/values`, `Table/update!`, `Table/update-or!`, `Table/push-or!`,
  `Table/count`, `Table/frequency`, `Table/merge`, `Table/merge!`,
  `Table/resize!`, `Table/compact!`, and `Table/count-entries`.
- Sets: `Set/new`, `Set/new/capacity`, `Set/add!`, `Set/remove!`,
  `Set/has?`, `Set/values`, `Set/size`, `Set/count`, `Set/intersection`,
  `Set/union`, `Set/difference`, `Set/xor`, `Set/resize!`, and `Set/compact!`.
- Heaps: `Heap/push!`, `Heap/pop!`, `Heap/peek!`, `Heap/replace!`,
  `Heap/empty!`, `Heap/empty?`, `Heap/not-empty?`, `Heap/parent`,
  `Heap/left`, `Heap/right`, `Heap/sift-up!`, and `Heap/sift-down!`.
- Deques: `Que/new`, `Que/length`, `Que/empty?`, `Que/not-empty?`,
  `Que/get`, `Que/at`, `Que/peek`, `Que/first`, `Que/last`, `Que/append!`,
  `Que/enque!`, `Que/push!`, `Que/prepend!`, `Que/pop!`, `Que/tail!`,
  `Que/deque!`, and `Que/empty!`.
- Matrices: `Matrix/new`, `Matrix/in-bounds?`, `Matrix/for/i`, and
  `Matrix->String`. `neighborhood/*` constants provide common coordinate
  kernels.

## Specialist representations

`BigInt/*` implements unsigned arbitrary-precision arithmetic, including
`new`, `add`, `sub`, `mul`, `div`, `mod`, comparisons, `pow`, `square`,
`sum`, `product`, and `range`. `SignedBigInt/*` supplies the corresponding
signed representation. `Date/*` extracts pieces of the date-vector format;
`Meta/*` parses lightweight metadata; `Payload/get` accesses embedded payload
text.

The following names are intentionally low-level: `*/raw`, `*/unchecked!`,
`Table/hash`, `Set/hash`, allocation/bucket helpers, implementation constants,
and aliases maintained for library implementation. They are exported because
the library is Calque source, not because new applications should depend on
their representation. Query their source before using them.
