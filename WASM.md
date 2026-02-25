# WASM

This guide covers how to run Que programs as generated WebAssembly modules and how to pass optional `ARGV` from the host.

It focuses on two host environments:
- JavaScript
- Rust (`wasmtime`)

## What The Generated Module Exports

The generated program module exports:

- `main() -> i32`
- `memory`

ARGV interop:
- `get_argv() -> i32`
- `set_argv(ptr: i32) -> i32`
- `argv_clear() -> i32`
- `argv_push(v: i32) -> i32`

Value constructors/helpers:
- `make_vec(elem_ref: i32) -> i32`
- `vec_push(ptr: i32, v: i32) -> i32`
- `make_tuple(a: i32, b: i32) -> i32`

RC helpers:
- `retain(ptr: i32) -> i32`
- `release(ptr: i32) -> i32`

## Host Value Model

For `ARGV`, host-side values should be encoded as:
- `Int` => plain `i32`
- `Vector` => wasm vector pointer
- `Tuple` => wasm tuple pointer (2-element vector under the hood)

In JS examples below:
- arrays map to Que vectors
- tuples use helper `qTuple(a, b)`

## JavaScript

### 1) Compile Que to WAT (using your existing compiler wasm)

```js
import binaryen from "binaryen";
import fez from "./node/fez_rs.js";

const { __wasm, get_output_len, wat } = fez;
const compilerMemory = __wasm.memory;

const readWasmString = (ptr, len) =>
  new TextDecoder().decode(new Uint8Array(compilerMemory.buffer, ptr, len));

export const compileWat = (program) =>
  readWasmString(wat(program), get_output_len());

function assertValidWat(watText) {
  if (!watText || watText[0] !== ";") {
    const err = watText ? watText.slice(2) : "Unknown compile error";
    throw new Error(err.trim());
  }
}
```

### 2) Encode ARGV values

```js
export const qTuple = (a, b) => ({ __tuple: [a, b] });

const isTupleValue = (v) =>
  v && typeof v === "object" && Array.isArray(v.__tuple) && v.__tuple.length === 2;

function encodeQueValue(value, e) {
  if (Number.isInteger(value)) return { raw: value | 0, managed: false };

  if (Array.isArray(value)) {
    const vec = e.make_vec(1); // elem_ref=1 => nested refs safe
    for (const item of value) {
      const enc = encodeQueValue(item, e);
      e.vec_push(vec, enc.raw);
      if (enc.managed) e.release(enc.raw); // transferred into vec
    }
    return { raw: vec, managed: true };
  }

  if (isTupleValue(value)) {
    const [a, b] = value.__tuple;
    const ea = encodeQueValue(a, e);
    const eb = encodeQueValue(b, e);
    const tup = e.make_tuple(ea.raw, eb.raw);
    if (ea.managed) e.release(ea.raw); // transferred into tuple
    if (eb.managed) e.release(eb.raw);
    return { raw: tup, managed: true };
  }

  throw new Error(`Unsupported ARGV value: ${JSON.stringify(value)}`);
}

function setArgv(argv, instance) {
  const e = instance.exports;

  if (argv == null) {
    e.argv_clear();
    return;
  }
  if (!Array.isArray(argv)) {
    throw new Error("options.argv must be an array");
  }

  const enc = encodeQueValue(argv, e); // whole argv vector
  e.set_argv(enc.raw);
  if (enc.managed) e.release(enc.raw); // set_argv retained it
}
```

### 3) Optional decode helpers (for nested return values)

```js
const i32 = (mem, addr) => new DataView(mem.buffer).getInt32(addr, true);

function readVec(ptr, memory) {
  const len = i32(memory, ptr + 0);
  const dataPtr = i32(memory, ptr + 16);
  const items = [];
  for (let i = 0; i < len; i++) items.push(i32(memory, dataPtr + i * 4));
  return { len, dataPtr, items };
}

function readTuple(ptr, memory) {
  const v = readVec(ptr, memory);
  if (v.len !== 2) throw new Error(`tuple len != 2 (${v.len})`);
  return [v.items[0], v.items[1]];
}

function splitTupleTypes(str) {
  let parts = [];
  let current = "";
  let depth = 0;
  for (const ch of str) {
    if (ch === "[" || ch === "{") depth += 1;
    if (ch === "]" || ch === "}") depth -= 1;
    if (ch === "*" && depth === 0) {
      parts.push(current.trim());
      current = "";
    } else {
      current += ch;
    }
  }
  parts.push(current.trim());
  return parts;
}

function getValue(ptr, typ, instance) {
  const memory = instance.exports.memory;
  const t = typ.trim();
  const scratch = new DataView(new ArrayBuffer(4));

  if (t === "Bool") return ptr === 1;
  if (t === "Float") {
    scratch.setInt32(0, ptr, true);
    return scratch.getFloat32(0, true);
  }
  if (t === "Char") return String.fromCharCode(ptr);
  if (t === "[Char]") {
    return readVec(ptr, memory).items.map((x) => String.fromCharCode(x)).join("");
  }

  if (t.startsWith("[") && t.endsWith("]")) {
    const innerTyp = t.slice(1, -1).trim();
    return readVec(ptr, memory).items.map((itemPtr) =>
      getValue(itemPtr, innerTyp, instance)
    );
  }

  if (t.startsWith("{") && t.endsWith("}")) {
    const parts = splitTupleTypes(t.slice(1, -1).trim());
    const rawTuple = readTuple(ptr, memory);
    return rawTuple.map((itemPtr, i) => getValue(itemPtr, parts[i], instance));
  }

  return ptr; // Int / fallback
}
```

### 4) End-to-end run with optional ARGV

```js
export async function runQueProgram(source, options = {}) {
  const watText = options.watText ?? compileWat(source);
  assertValidWat(watText);

  const module = binaryen.parseText(watText);
  module.optimize();
  const wasmBytes = module.emitBinary();

  const { instance } = await WebAssembly.instantiate(wasmBytes, {});

  if (options.argv !== undefined) {
    setArgv(options.argv, instance);
  }

  const typ = watText.match(/Type:\s*([^\r\n]*)/)?.[1]?.trim() || "Int";
  const rawResult = instance.exports.main();
  const result = getValue(rawResult, typ, instance);

  return { typ, result };
}
```

### JS usage examples

```js
// ARGV omitted => defaults to []
await runQueProgram("(length ARGV)"); // => 0

// Simple ints
await runQueProgram("(get ARGV 0)", { argv: [42] }); // => 42

// Nested values (vector + tuple)
await runQueProgram(
  "(get (get ARGV 1) 0)",
  { argv: [7, [99, qTuple(1, 2)]] }
); // => 99
```

## Rust (wasmtime)

The example below assumes you already have WAT text generated (for example via CLI `--wat`).

`Cargo.toml` dependencies:

```toml
[dependencies]
anyhow = "1"
wasmtime = "*"
wat = "*"
```

Rust host example:

```rust
use anyhow::Result;
use wasmtime::{Engine, Instance, Module, Store, TypedFunc};

#[derive(Clone, Debug)]
enum QValue {
    Int(i32),
    Vec(Vec<QValue>),
    Tuple(Box<QValue>, Box<QValue>),
}

fn encode_qvalue(
    store: &mut Store<()>,
    make_vec: &TypedFunc<i32, i32>,
    vec_push: &TypedFunc<(i32, i32), i32>,
    make_tuple: &TypedFunc<(i32, i32), i32>,
    release: &TypedFunc<i32, i32>,
    v: &QValue,
) -> Result<(i32, bool)> {
    match v {
        QValue::Int(n) => Ok((*n, false)),
        QValue::Vec(xs) => {
            let vec_ptr = make_vec.call(store, 1)?; // elem_ref=1
            for x in xs {
                let (raw, managed) = encode_qvalue(store, make_vec, vec_push, make_tuple, release, x)?;
                vec_push.call(store, (vec_ptr, raw))?;
                if managed {
                    release.call(store, raw)?; // ownership transferred into vec
                }
            }
            Ok((vec_ptr, true))
        }
        QValue::Tuple(a, b) => {
            let (ra, ma) = encode_qvalue(store, make_vec, vec_push, make_tuple, release, a)?;
            let (rb, mb) = encode_qvalue(store, make_vec, vec_push, make_tuple, release, b)?;
            let t = make_tuple.call(store, (ra, rb))?;
            if ma { release.call(store, ra)?; }
            if mb { release.call(store, rb)?; }
            Ok((t, true))
        }
    }
}

fn main() -> Result<()> {
    // Load WAT text from file generated by your compiler
    let wat_text = std::fs::read_to_string("./example/dist/main.wat")?;
    let wasm_bytes = wat::parse_str(&wat_text)?;

    let engine = Engine::default();
    let module = Module::new(&engine, wasm_bytes)?;
    let mut store = Store::new(&engine, ());
    let instance = Instance::new(&mut store, &module, &[])?;

    let main: TypedFunc<(), i32> = instance.get_typed_func(&mut store, "main")?;
    let set_argv: TypedFunc<i32, i32> = instance.get_typed_func(&mut store, "set_argv")?;
    let argv_clear: TypedFunc<(), i32> = instance.get_typed_func(&mut store, "argv_clear")?;
    let make_vec: TypedFunc<i32, i32> = instance.get_typed_func(&mut store, "make_vec")?;
    let vec_push: TypedFunc<(i32, i32), i32> = instance.get_typed_func(&mut store, "vec_push")?;
    let make_tuple: TypedFunc<(i32, i32), i32> = instance.get_typed_func(&mut store, "make_tuple")?;
    let release: TypedFunc<i32, i32> = instance.get_typed_func(&mut store, "release")?;

    // Optional ARGV setup
    argv_clear.call(&mut store, ())?;
    let argv = QValue::Vec(vec![
        QValue::Int(42),
        QValue::Vec(vec![QValue::Int(7), QValue::Int(8)]),
        QValue::Tuple(Box::new(QValue::Int(10)), Box::new(QValue::Int(23))),
    ]);

    let (argv_ptr, managed) = encode_qvalue(
        &mut store,
        &make_vec,
        &vec_push,
        &make_tuple,
        &release,
        &argv,
    )?;
    set_argv.call(&mut store, argv_ptr)?;
    if managed {
        release.call(&mut store, argv_ptr)?; // set_argv retained it
    }

    let raw = main.call(&mut store, ())?;
    println!("main returned raw i32: {}", raw);
    Ok(())
}
```

## Ownership Rules (Important)

- `set_argv(ptr)` retains `ptr` internally.
- If host created a managed value and passed it to `set_argv`, host should `release(ptr)` once after setting.
- `vec_push` / `make_tuple` retain managed elements internally.
- After pushing/constructing, host should release temporary managed children it still owns.

If you only pass small integer-only argv values, you can ignore retain/release complexity.

## Practical Recommendation

For simple apps:
- keep ARGV to integer-only values initially
- then add nested vectors/tuples once your decode path is stable

For production:
- prefer a stable encoded-blob ABI (`set_argv_encoded(ptr,len)`) so hosts do not depend on runtime constructors.
