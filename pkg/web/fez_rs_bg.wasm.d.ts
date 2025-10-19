/* tslint:disable */
/* eslint-disable */
export const memory: WebAssembly.Memory;
export const cons_str: (a: number, b: number, c: number, d: number) => [number, number];
export const exec_str: (a: number, b: number) => [number, number];
export const comp_str: (a: number, b: number) => [number, number];
export const run: (a: number, b: number) => [number, number];
export const js: (a: number, b: number) => [number, number];
export const check: (a: number, b: number) => [number, number];
export const get_output_ptr: () => number;
export const get_output_len: () => number;
export const exec_to_buffer: (a: number, b: number) => number;
export const comp_to_buffer: (a: number, b: number) => number;
export const cons_to_buffer: (a: number, b: number, c: number, d: number) => number;
export const __wbindgen_export_0: WebAssembly.Table;
export const __wbindgen_malloc: (a: number, b: number) => number;
export const __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
export const __wbindgen_free: (a: number, b: number, c: number) => void;
export const __wbindgen_start: () => void;
