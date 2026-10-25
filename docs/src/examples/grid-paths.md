# Grid paths: mutable dynamic programming

This program counts right/down paths through a square grid. It demonstrates a
small mutable vector, capacity reservation, an analyzable indexed loop, and an
explicit arithmetic safety guard.

```que
; use-strict-warnings!

; How many routes go from the top-left to the bottom-right of a 10 × 10 grid
; when each step goes only right or down?
(let SIZE 10)

(let paths [])
(capacity! (+ SIZE 1) paths)
(loop i (<= i SIZE)
  (push! 1 paths))

(loop row (< row SIZE)
  (loop/for col 1 (<= col SIZE) (++ col)
    (let left (get (- col 1) paths))
    (let current (get col paths))
    (when (+/safe? current left)
      (set! col (+ current left) paths))))

; 184756
(get (- (length paths) 1) paths)
```

The vector holds one dynamic-programming row. Initially every entry is one;
each update adds the number of paths from the cell to the left to the number
already stored for the cell above.

`capacity!` reserves room for the initial `SIZE + 1` appends but does not alter
the vector’s length. The analyser can see the loop access bounds, and the
`+/safe?` guard gives the source-level condition for the arithmetic update.

Run the file during development with:

```sh
que grid-paths.que --debug
que explain grid-paths.que --opt
```
