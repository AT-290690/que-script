const { exec, get_output_len, __wasm } = require("./pkg/fez_rs.js");
const { readFileSync } = require("fs");
const memory = __wasm.memory;
const readWasmString = (ptr, len) =>
  new TextDecoder().decode(new Uint8Array(memory.buffer, ptr, len));
const execBiteCode = (program) =>
  readWasmString(exec(program), get_output_len());

const filePath = process.argv[2];

if (!filePath) {
  console.error("Please provide a file path.");
  process.exit(1);
}

console.log(execBiteCode(readFileSync(filePath, "utf-8")));
