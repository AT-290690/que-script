# Que native C host

This directory contains the optional host-import implementation used when a
Que Wasm module is translated to C with `wasm2c`. It is not part of the Que
compiler and does not add a C dependency to ordinary Que builds.

Build a program with:

```sh
./scripts/compile-native-c.sh program.que build/native
```

Pure programs run normally. Programs using host IO require permissions when
the resulting executable is launched. Permissions can be supplied through the
native executable or through the environment:

```sh
./build/native/main --allow read write print -- program-arg
QUE_ALLOW=print ./build/native/main
QUE_ALLOW=read,write,print ./build/native/main
QUE_ALLOW=all ./build/native/main
```

Implemented imports:

- `print!`, `clear!`
- `read!`, `stdin!`, `list-dir!`
- `write!`, `mkdir!`, `move!`, `delete!`
- `sleep!`, `time!`, `random!`
- `read/chunks!`, `stdin/chunks!`, `read/lines!`
- `serialize`, `deserialize`

Non-option executable arguments populate Que's `ARGV`. Use `--` when a program
argument itself begins with an option-like prefix.

File paths are relative to the executable's working directory. Absolute paths,
paths containing a `..` component, and symbolic-link traversal are rejected.
Directory results are sorted consistently with the standard Que host.

Run the native host integration checks with:

```sh
./miscs/native-c/test.sh
```
