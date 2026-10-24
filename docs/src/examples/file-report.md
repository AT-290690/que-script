# Build a file-report program

This short program reads a text file, parses one integer per non-empty line,
and reports the count and sum. It demonstrates the normal Calque workflow:
write direct code, run it safely, inspect facts, then use optimized execution.

Create `numbers.txt`:

```text
12
14
1969
100756
```

Then create `report.que`:

```que
; use-strict-warnings!

(let parse
  (comp
    split/lines
    (select (lambda (line) (> (length line) 0)))
    (map String->Integer)))

(let report
  (lambda (text)
    (let values (parse text))
    { (length values) (sum values) }))

(let input (read! "numbers.txt"))
(report input)
```

Run it during development with the smallest permission it needs:

```sh
que report.que --debug --allow read
; {4 102751}
```

If the file is absent, `read!` reports an ordinary host error such as
`read!: numbers.txt: file not found`; it is not a Calque safety trap.

## Read what the compiler knows

Ask for the optimized report before treating this as a hot path:

```sh
que explain report.que --opt
```

The report shows the final result type, effects (the `read!` is host I/O), and
any bounds, arithmetic, or termination facts that were or were not proven. It
is static information, not a timing measurement.

## Release run

Once representative inputs run cleanly in debug mode and the warnings have
been addressed deliberately:

```sh
que report.que --opt --allow read
```

The pipeline is appropriate here because the transformation is a direct
data-last sequence. For a tight indexed algorithm, use a mutable vector and a
`loop` instead; see the [grid-paths example](grid-paths.md).

## Extend it

The next natural changes each introduce one Calque feature rather than a new
framework:

- return an enum such as `Empty` or `Numbers` when an empty file needs a
  distinct result;
- use a `Table` to report frequencies rather than one aggregate;
- add a `sig` when this becomes a public project boundary;
- put parsing helpers in `lib/parse.que`, list that file in `que.toml`, and
  keep `report.que` as the executable entry.

The [projects chapter](../compiler/projects-and-editors.md) explains that
last step.
