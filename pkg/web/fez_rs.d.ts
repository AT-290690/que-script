/* tslint:disable */
/* eslint-disable */
export function cons_str(a: string, b: string): string;
export function exec_str(program: string): string;
export function comp_str(program: string): string;
export function run(program: string): string;
export function js(program: string): string;
export function check(program: string): string;
export function get_output_ptr(): number;
export function get_output_len(): number;
export function exec_to_buffer(program: string): number;
export function comp_to_buffer(program: string): number;
export function cons_to_buffer(a: string, b: string): number;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
  readonly memory: WebAssembly.Memory;
  readonly cons_str: (a: number, b: number, c: number, d: number) => [number, number];
  readonly exec_str: (a: number, b: number) => [number, number];
  readonly comp_str: (a: number, b: number) => [number, number];
  readonly run: (a: number, b: number) => [number, number];
  readonly js: (a: number, b: number) => [number, number];
  readonly check: (a: number, b: number) => [number, number];
  readonly get_output_ptr: () => number;
  readonly get_output_len: () => number;
  readonly exec_to_buffer: (a: number, b: number) => number;
  readonly comp_to_buffer: (a: number, b: number) => number;
  readonly cons_to_buffer: (a: number, b: number, c: number, d: number) => number;
  readonly __wbindgen_export_0: WebAssembly.Table;
  readonly __wbindgen_malloc: (a: number, b: number) => number;
  readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
  readonly __wbindgen_free: (a: number, b: number, c: number) => void;
  readonly __wbindgen_start: () => void;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;
/**
* Instantiates the given `module`, which can either be bytes or
* a precompiled `WebAssembly.Module`.
*
* @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
*
* @returns {InitOutput}
*/
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
* If `module_or_path` is {RequestInfo} or {URL}, makes a request and
* for everything else, calls `WebAssembly.instantiate` directly.
*
* @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
*
* @returns {Promise<InitOutput>}
*/
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
