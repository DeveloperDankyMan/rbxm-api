/* tslint:disable */
/* eslint-disable */
export const memory: WebAssembly.Memory;
export const decode: (a: number, b: number) => [number, number, number, number];
export const decodeB64: (a: number, b: number) => [number, number, number, number];
export const encode: (a: number, b: number) => [number, number, number, number];
export const encodeB64: (a: number, b: number) => [number, number, number, number];
export const schema: (a: number, b: number) => [number, number, number, number];
export const start: () => void;
export const rust_zstd_wasm_shim_calloc: (a: number, b: number) => number;
export const rust_zstd_wasm_shim_free: (a: number) => void;
export const rust_zstd_wasm_shim_malloc: (a: number) => number;
export const rust_zstd_wasm_shim_memcmp: (a: number, b: number, c: number) => number;
export const rust_zstd_wasm_shim_memcpy: (a: number, b: number, c: number) => number;
export const rust_zstd_wasm_shim_memmove: (a: number, b: number, c: number) => number;
export const rust_zstd_wasm_shim_memset: (a: number, b: number, c: number) => number;
export const rust_zstd_wasm_shim_qsort: (a: number, b: number, c: number, d: number) => void;
export const __wbindgen_exn_store: (a: number) => void;
export const __externref_table_alloc: () => number;
export const __wbindgen_externrefs: WebAssembly.Table;
export const __wbindgen_free: (a: number, b: number, c: number) => void;
export const __wbindgen_malloc: (a: number, b: number) => number;
export const __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
export const __externref_table_dealloc: (a: number) => void;
export const __wbindgen_start: () => void;
