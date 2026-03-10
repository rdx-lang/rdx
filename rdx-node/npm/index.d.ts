import type {
  RdxRoot,
  RdxNode,
  RdxSchema,
  RdxDiagnostic,
} from "@rdx-lang/types";

/** Parse an RDX document into a typed AST. */
export function parse(input: string): RdxRoot;

/** Parse with default transforms (auto-slug + table of contents). */
export function parseWithDefaults(input: string): RdxRoot;

/** Parse with a specific set of transforms. */
export function parseWithTransforms(
  input: string,
  transforms: string[]
): RdxRoot;

/** Validate a parsed AST against a component schema. */
export function validate(ast: RdxRoot, schema: RdxSchema): RdxDiagnostic[];

/** Extract all plain text from an AST. */
export function collectText(ast: RdxRoot): string;

/** Find all nodes of a given type in the AST. */
export function queryAll<T extends RdxNode = RdxNode>(
  ast: RdxRoot,
  nodeType: T["type"]
): T[];

/** Get the RDX parser version. */
export function version(): string;
