/* tslint:disable */
/* eslint-disable */

/**
 * Raw .rbxm bytes -> JSON tree string (same shape POST /decode returns).
 */
export function decode(rbxm_bytes: Uint8Array): string;

/**
 * Same as `decode`, but takes a base64 string instead of raw bytes.
 */
export function decodeB64(rbxm_b64: string): string;

/**
 * JSON tree (same shape POST /encode expects) -> raw .rbxm bytes.
 */
export function encode(tree_json: string): Uint8Array;

/**
 * Same as `encode`, but returns a base64 string instead of raw bytes — matching the
 * server's `?b64=1` mode. Useful when the bytes need to pass through something
 * text-only (JSON, a text field, copy-paste) rather than staying binary.
 */
export function encodeB64(tree_json: string): string;

/**
 * Same data GET /schema/:class returns, as a JSON string.
 */
export function schema(class_name: string): string;

/**
 * Call once, right after `init()` in JS, for readable panic messages in the browser console.
 */
export function start(): void;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly decode: (a: number, b: number) => [number, number, number, number];
    readonly decodeB64: (a: number, b: number) => [number, number, number, number];
    readonly encode: (a: number, b: number) => [number, number, number, number];
    readonly encodeB64: (a: number, b: number) => [number, number, number, number];
    readonly schema: (a: number, b: number) => [number, number, number, number];
    readonly start: () => void;
    readonly rust_zstd_wasm_shim_calloc: (a: number, b: number) => number;
    readonly rust_zstd_wasm_shim_free: (a: number) => void;
    readonly rust_zstd_wasm_shim_malloc: (a: number) => number;
    readonly rust_zstd_wasm_shim_memcmp: (a: number, b: number, c: number) => number;
    readonly rust_zstd_wasm_shim_memcpy: (a: number, b: number, c: number) => number;
    readonly rust_zstd_wasm_shim_memmove: (a: number, b: number, c: number) => number;
    readonly rust_zstd_wasm_shim_memset: (a: number, b: number, c: number) => number;
    readonly rust_zstd_wasm_shim_qsort: (a: number, b: number, c: number, d: number) => void;
    readonly __wbindgen_exn_store: (a: number) => void;
    readonly __externref_table_alloc: () => number;
    readonly __wbindgen_externrefs: WebAssembly.Table;
    readonly __wbindgen_free: (a: number, b: number, c: number) => void;
    readonly __wbindgen_malloc: (a: number, b: number) => number;
    readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
    readonly __externref_table_dealloc: (a: number) => void;
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
