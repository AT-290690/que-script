# Enum-keyed table round trip

Closed enums remain typed values even though their runtime representation is a
compact `i32` tag. That makes them suitable for exhaustive dispatch and fast
table keys.

```que
{{#include ../../../examples/demo/04-enum-table-round-trip.que}}
```

The runnable source is
[`examples/demo/04-enum-table-round-trip.que`](https://github.com/AT-290690/que-script/blob/main/examples/demo/04-enum-table-round-trip.que).
From a checkout, run it with:

```sh
que examples/demo/04-enum-table-round-trip.que --debug
```

The `[[{ Shape Int }]]` annotation is only needed because `deserialize` needs
to know which decoder to generate. It is the current library representation of
a table: buckets containing key/value tuples. Normal table code should keep
using `Table/new`, `Table/set!`, `Table/get*`, and `Table/get-unsafe` rather
than accessing that representation directly.

Enum serialization uses declared names (`Rock`, `Paper`, `Scissors`), and the
decoder validates them against the expected enum. `=~` compares values of this
same enum; `match` checks coverage of its variants.
