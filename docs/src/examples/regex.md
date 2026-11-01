# Regular expressions

Calque’s regular-expression engine is a Thompson NFA: matching time stays
predictable even for patterns that cause catastrophic backtracking in engines
based on recursive backtracking.

Compile a pattern once if it is reused. `String/regex/matches?` requires the
whole input to match, while `String/regex/contains?` searches for a match
anywhere in the input. Both take their text argument last.

```que
{{#include ../../../examples/demo/05-regex.que}}
```

Run it from a checkout with:

```sh
que examples/demo/05-regex.que --debug
```

Supported syntax is grouping, alternation (`|`), `*`, `+`, `?`, `.`, `^`, `$`,
character classes and ranges, negated classes, and the `\\d`, `\\D`, `\\w`,
`\\W`, `\\s`, `\\S`, `\\n`, `\\r`, `\\t`, `\\f`, and `\\v` escapes. It is not a
PCRE implementation: counted repetition, word boundaries, backreferences,
look-around, and hexadecimal escapes are unsupported.

`String/regex` never throws for invalid pattern syntax. Instead,
`String/regex/valid?` reports `false`, and the matching predicates return
`false`. This permits dynamic patterns without making malformed input a
runtime failure.
