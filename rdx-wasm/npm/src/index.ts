import type {
  RdxRoot,
  RdxNode,
  RdxSchema,
  RdxDiagnostic,
} from "@rdx-lang/types";

// Re-export all types for convenience
export type * from "@rdx-lang/types";

let wasmModule: typeof import("../wasm-pkg/rdx_wasm.js") | null = null;

/**
 * Initialize the WASM module. Must be called once before using any other function.
 *
 * In browsers, pass the URL to the `.wasm` file:
 * ```ts
 * await init(new URL("./rdx_wasm_bg.wasm", import.meta.url));
 * ```
 *
 * In Node.js/Bun, pass the buffer:
 * ```ts
 * import fs from "fs";
 * await init(fs.readFileSync("./rdx_wasm_bg.wasm"));
 * ```
 */
export async function init(input?: any): Promise<void> {
  const wasm = await import("../wasm-pkg/rdx_wasm.js");
  await wasm.default(input);
  wasmModule = wasm;
}

function getWasm() {
  if (!wasmModule) {
    throw new Error(
      "RDX WASM not initialized. Call `await init()` first."
    );
  }
  return wasmModule;
}

/** Parse an RDX document into a typed AST. */
export function parse(input: string): RdxRoot {
  return getWasm().parse(input) as RdxRoot;
}

/** Parse with default transforms (auto-slug + table of contents). */
export function parseWithDefaults(input: string): RdxRoot {
  return getWasm().parseWithDefaults(input) as RdxRoot;
}

/** Parse with a specific set of transforms. */
export function parseWithTransforms(
  input: string,
  transforms: string[]
): RdxRoot {
  return getWasm().parseWithTransforms(input, transforms) as RdxRoot;
}

/** Validate a parsed AST against a component schema. */
export function validate(
  ast: RdxRoot,
  schema: RdxSchema
): RdxDiagnostic[] {
  return getWasm().validate(ast, schema) as RdxDiagnostic[];
}

/** Extract all plain text from an AST. */
export function collectText(ast: RdxRoot): string {
  return getWasm().collectText(ast);
}

/** Find all nodes of a given type in the AST. */
export function queryAll<T extends RdxNode = RdxNode>(
  ast: RdxRoot,
  nodeType: T["type"]
): T[] {
  return getWasm().queryAll(ast, nodeType) as T[];
}

/** Get the RDX parser version. */
export function version(): string {
  return getWasm().version();
}
