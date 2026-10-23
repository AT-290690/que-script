# Recipes

These are compact, complete patterns rather than isolated API calls. Copy one
into a `.que` file, run it with `--debug`, and adapt it to your data.

## Parse lines and aggregate them

`comp` makes a reusable parser when the input simply flows through each stage.
The data-last collection API keeps each transformation readable:

```que
(let parse-and-sum
  (comp
    trim
    split/lines
    (map String->Integer)
    sum))

(parse-and-sum "10\n20\n12")
; 42
```

For file input, put `read!` at the front and grant the read permission:

```que
(let sum-file
  (comp
    read!
    trim
    split/lines
    (map String->Integer)
    sum))

(sum-file "numbers.txt")
```

```sh
que sum-file.que --debug --allow read
```

## Count occurrences with a Table

Use `Table/update-or!` for the common “insert an initial value or update the
existing value” pattern. The guard gives a defined result if a count reaches
the maximum `Int` value.

```que
(let frequency
  (lambda (words)
    (let counts (Table/new))
    (loop/in word words
      (Table/update-or! counts word
        (lambda () 1)
        (lambda (n)
          (if (+/safe? n 1) (+ n 1) n))))
    counts))

(let counts (frequency ["tea" "coffee" "tea" "tea"]))
(Table/get-unsafe "tea" counts)
; 3
```

If the approximate number of distinct keys is known, create the table with
`Table/new/capacity` instead. See [Tables](../standard-library/data-structures.md#tables).

## Model a closed state machine with enums

Enums prevent accidental use of arbitrary integers or strings as states.
`=~` compares values of the same enum, while `match` makes rendering a state
exhaustive:

```que
(enum Request Idle Loading Ready Failed)
(enum Event Start Succeed Fail Reset)

(let transition
  (lambda (state event)
    (cond
      (=~ event Reset) Idle
      (and (=~ state Idle) (=~ event Start)) Loading
      (and (=~ state Loading) (=~ event Succeed)) Ready
      (and (=~ state Loading) (=~ event Fail)) Failed
      state)))

(let describe
  (lambda (state)
    (match state
      Idle "idle"
      Loading "loading"
      Ready "ready"
      Failed "failed")))

(describe (transition Idle Start))
; "loading"
```

Adding a new `Request` variant makes each exhaustive `match` site a type error
until it has a deliberate branch.

## Reuse a scratch vector

Reserve storage once, refill it, then keep the capacity for the next pass.
This is useful in repeated simulations and parsers with a predictable maximum
batch size:

```que
(let scratch [])
(capacity! scratch 1000)

(loop pass (< pass 3)
  (empty! scratch)
  (loop i (< i 1000)
    (push! scratch (+ pass i)))
  (length scratch))

(length scratch)
; 1000
```

`empty!` changes the length, not the reserved backing capacity.

## Dynamic programming in one vector

Many dynamic-programming problems only require the prior row or column. Keep
that state in one mutable vector rather than building a full matrix. The grid
paths example counts right/down paths and demonstrates a proof-friendly indexed
loop, `capacity!`, and a checked arithmetic update.

See [Grid paths: mutable dynamic programming](grid-paths.md) for the complete
program and an explanation of its invariants.
