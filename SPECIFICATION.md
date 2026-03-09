# **RDX (Reactive Document eXpressions) Specification**

**Version:** 1.0.0-rc

**Status:** Release Candidate

**File Extension:** `.rdx`

## **1. Introduction**

Reactive Document eXpressions (RDX) is a strictly typed, declarative extension to the CommonMark specification. It defines a standardized syntax and Abstract Syntax Tree (AST) for embedding interactive components and structured data within Markdown documents.

The primary goal of RDX is to provide a safe, parser-agnostic standard for "Docs as Data," allowing technical writers to author rich components without the security and performance implications of arbitrary JavaScript execution (as seen in imperative formats like MDX).

### **1.1. Terminology**

The key words **"MUST"**, **"MUST NOT"**, **"REQUIRED"**, **"SHALL"**, **"SHALL NOT"**, **"SHOULD"**, **"SHOULD NOT"**, **"RECOMMENDED"**, **"MAY"**, and **"OPTIONAL"** in this document are to be interpreted as described in [RFC 2119](https://datatracker.ietf.org/doc/html/rfc2119).

* **Parser**: A software library that consumes an `.rdx` string and outputs a compliant RDX AST.
* **Host Environment / Renderer**: The downstream application that consumes the RDX AST to produce a final output (e.g., HTML, native UI elements, or intermediate framework code).
* **Component**: A custom, interactive element defined by an uppercase tag within the document.

### **1.2. Versioning & Forward Compatibility**

This specification follows [Semantic Versioning 2.0.0](https://semver.org/). A Parser built for version `1.x` **SHOULD** gracefully handle documents authored for a later `1.y` (where `y > x`) by emitting an `RdxErrorNode` for any syntactic construct it does not recognize, rather than failing catastrophically. A major version increment (e.g., `2.0.0`) **MAY** introduce breaking changes to the AST schema or syntax grammar.

## **2. Syntax Grammar**

RDX is a superset of CommonMark. Any valid CommonMark document is a valid RDX document. RDX introduces new normative syntactic structures: Frontmatter, Components, Strictly-Typed Attributes, Escaping, and Context Variables.

A compliant Parser **MUST** also support the following GitHub Flavored Markdown (GFM) extensions:

* **Tables**: Pipe-based table syntax per the GFM specification.
* **Strikethrough**: `~~text~~` syntax, emitted as an `RdxStandardBlockNode` of type `"strikethrough"`.
* **Task Lists**: `- [x]` and `- [ ]` syntax within list items. The Parser **MUST** set the `checked` field on the corresponding `list_item` node.
* **Footnotes**: `[^label]` references and `[^label]: content` definitions. The Parser **MUST** emit `RdxFootnoteNode` nodes for both references and definitions.

### **2.1. Frontmatter (Document Metadata)**

Unlike standard CommonMark, RDX natively supports YAML frontmatter for document metadata.

The Parser **MUST** attempt to parse a YAML block at the absolute beginning of the document, delimited by `---`. If present, the Parser **MUST** extract this before evaluating any CommonMark or Component nodes.

To disambiguate frontmatter from CommonMark thematic breaks (`---`), the following lexical rules apply:

1. The opening `---` **MUST** begin at line 1, column 1 of the document. It **MUST NOT** be preceded by whitespace or any other character.
2. The closing `---` **MUST** be the sole non-whitespace content on its line, followed immediately by a newline character or end-of-file.
3. Any `---` appearing after line 1 that is not the closing delimiter of an active frontmatter block **MUST** be parsed as a standard CommonMark thematic break.

```rdx
---
title: Introduction
version: 2.1
---
```

### **2.2. Components (Tags)**

Component tags are HTML-like nodes. To strictly differentiate RDX Components from standard HTML elements, an RDX Component tag **MUST** begin with an uppercase ASCII letter (`A-Z`).

#### **2.2.1. Tag Name Grammar**

A valid RDX Component tag name **MUST** match the following pattern:

```
[A-Z][a-zA-Z0-9_]*
```

The name **MUST** begin with an uppercase ASCII letter and **MAY** be followed by any combination of ASCII letters (`a-z`, `A-Z`), digits (`0-9`), and underscores (`_`). Hyphens, dots, and other special characters are **NOT** permitted in tag names.

#### **2.2.2. Self-Closing and Block Tags**

* **Self-Closing Tags**: Components that do not contain child nodes **MUST** end with `/>`. A space before the `/>` is **OPTIONAL** (both `<Badge />` and `<Badge/>` are valid).

```rdx
<HeroImage src="/assets/hero.png" />
```

* **Block Tags**: Components that wrap child nodes **MUST** have an explicit opening tag and a matching closing tag.

```rdx
<Notice type="warning">
  This API is deprecated.
</Notice>
```

#### **2.2.3. Nesting**

Components **MAY** be nested to arbitrary depth. The Parser **MUST** enforce that closing tags match the most recently opened tag (i.e., strict LIFO/stack ordering). Misnested tags (e.g., `<A><B></A></B>`) **MUST** be treated as a fatal syntax error and the Parser **MUST** emit an `RdxErrorNode`.

A Parser implementation **MAY** impose a maximum nesting depth for resource protection, but this limit **MUST NOT** be lower than 128 levels.

#### **2.2.4. Block vs. Inline Placement**

A Parser **MUST** distinguish between block-level and inline-level components by following CommonMark's native block-parsing rules:

* **Block-Level**: A component is block-level if it is separated from surrounding content by blank lines (one or more empty lines before and/or after), or if it is the sole non-whitespace content on its line. A block-level component's closing tag **MUST** also be on its own line or be the sole non-whitespace content on its line.
* **Inline-Level**: A component is inline-level if it appears within a paragraph or other inline context alongside non-whitespace text, without being separated by blank lines. (e.g., `Here is a <Badge status="new" /> for this feature.`)

When a component is nested inside a standard HTML element (e.g., `<div><Notice>...</Notice></div>`), the component follows the same block/inline rules relative to its parent context.

#### **2.2.5. Boolean Shorthand Attributes**

An attribute name present on a component tag without an explicit value **MUST** be interpreted as a boolean `true`. This is syntactic sugar equivalent to `attributeName={true}`.

In the emitted `RdxAttributeNode`, the `position` **MUST** span only the attribute name token itself (e.g., the word `disabled`), since there is no `=` sign or value expression in the source.

```rdx
<Input disabled />
<!-- equivalent to: <Input disabled={true} /> -->
```

### **2.3. Strictly-Typed Attributes**

Attributes (properties) passed to components **MUST** be explicitly typed. The Parser **MUST NOT** evaluate attributes as executable code.

#### **2.3.1. Whitespace Around the Equals Sign**

For all attribute types, whitespace between the attribute name and the `=` sign, and between the `=` sign and the attribute value, is **NOT** permitted, with one exception: JSON Object Attributes (Section 2.3.4), where no whitespace is permitted between `=` and the opening `{{`.

Valid: `label="Click Me"`, `active={true}`
Invalid: `label = "Click Me"`, `active = {true}`

This zero-tolerance rule ensures unambiguous tokenization across all parser implementations.

#### **2.3.2. String Attributes**

String attributes **MUST** be enclosed in double quotes (`"`) or single quotes (`'`).

To include the enclosing quote character within a string value, the author **MAY** use either of the following strategies:

1. **Alternate quote type**: Wrap the string in the opposite quote character (e.g., `label='Click "Me"'`).
2. **Backslash escaping**: Escape the quote character with a backslash (e.g., `label="Click \"Me\""`). The Parser **MUST** strip the backslash and emit the unescaped quote in the resulting string value. The escape sequence `\\` **MUST** produce a literal backslash within the string.

No other escape sequences (e.g., `\n`, `\t`) are interpreted by the Parser inside string attributes. They **MUST** be passed through as-is.

```rdx
<Button label="Click Me" theme='dark' />
<Tooltip text="She said \"hello\"" />
<Tooltip text='She said "hello"' />
```

#### **2.3.3. Primitive Literal Attributes**

Boolean and Number literals **MUST** be enclosed in single curly braces `{}`. Valid contents are restricted strictly to: integers, floats (including negative values and scientific notation, e.g., `-3.14`, `2.5e10`), `true`, `false`, and `null`.

```rdx
<Pagination activePage={2} isInteractive={true} />
```

#### **2.3.4. JSON Object Attributes**

Complex data structures **MUST** be enclosed in double curly braces `{{ }}` and **MUST** conform to strict JSON syntax ([RFC 8259](https://datatracker.ietf.org/doc/html/rfc8259)). This applies to both JSON Objects and JSON Arrays. Single curly braces `{}` (Section 2.3.3) are reserved exclusively for primitive literals.

To ensure unambiguous tokenization, the boundary **MUST** be strictly `={{` with no whitespace permitted between the equals sign and the opening braces. Whitespace **IS** permitted between the opening `{{` and the JSON content, and between the JSON content and the closing `}}`.

```rdx
<!-- JSON Object -->
<Chart config={{
  "type": "bar",
  "data": [10, 20, 30]
}} />

<!-- JSON Array -->
<TagList items={{["alpha", "beta", "gamma"]}} />
```

#### **2.3.5. Variable Attributes**

Variables **MAY** be passed as attribute values. The Parser **MUST** identify these using the `{$variable_path}` syntax without surrounding quotes.

```rdx
<Button label={$frontmatter.buttonText} />
```

### **2.4. Context Variables (Interpolation)**

Variables inject contextual data into the document text. Variable interpolation **MUST** be formatted as `{$variable_path}`.

#### **2.4.1. Variable Path Grammar**

A valid variable path **MUST** match the following pattern:

```
[a-zA-Z_][a-zA-Z0-9_]*(\.[a-zA-Z_][a-zA-Z0-9_]*)*
```

Paths are dot-delimited property accessors. Array index access (e.g., `items[0]`) is **NOT** supported in version 1.x. Each segment **MUST** begin with a letter or underscore and **MAY** contain letters, digits, and underscores.

Examples of valid paths: `title`, `frontmatter.version`, `config.theme_name`

```rdx
# {$title}
Welcome to version {$version} of the documentation.
```

*Note: Variable resolution is the responsibility of the Host Environment. The Parser merely identifies and emits the variable path in the AST.*

#### **2.4.2. Variable Scope in Code Constructs**

The Parser **MUST NOT** evaluate `{$variable_path}` syntax inside CommonMark inline code spans (`` ` ` ``) or fenced code blocks (` ``` `). Text within these constructs **MUST** be emitted as literal `RdxTextNode` values with no variable interpolation.

### **2.5. Escaping**

To allow authors to render RDX syntax characters literally, the Parser **MUST** support backslash escaping for the following sequences:

| Source Text | Parsed Output | Description |
|---|---|---|
| `\{$path}` | `{$path}` | Escape variable interpolation |
| `\{{` | `{{` | Escape double-brace opening |
| `\}}` | `}}` | Escape double-brace closing |
| `\{` | `{` | Escape single-brace opening |
| `\\` | `\` | Literal backslash |

When the Parser encounters a backslash immediately preceding one of the above sequences, it **MUST** strip the backslash and emit the remaining characters as literal text in an `RdxTextNode`. A backslash preceding any other character **MUST** be passed through as-is (i.e., treated as a literal backslash), consistent with CommonMark's escaping behavior.

### **2.6. Math (LaTeX)**

RDX natively supports mathematical expressions using dollar-sign delimiters, without requiring external plugins.

#### **2.6.1. Inline Math**

Inline math expressions are delimited by single dollar signs: `$expression$`. The Parser **MUST** emit an `RdxTextNode` of type `"math_inline"` with the expression content as its `value`.

To avoid ambiguity with variable interpolation (`{$path}`), the `$` character **MUST NOT** be interpreted as a math delimiter when immediately preceded by `{`. The sequence `{$` always begins a variable expression.

Additional constraints for inline math:
* The content between delimiters **MUST NOT** be empty.
* The content **MUST NOT** start or end with a space character.

```rdx
The equation $x^2 + y^2 = z^2$ is well known.
```

#### **2.6.2. Display Math**

Display (block-level) math expressions are delimited by `$$` on their own lines. The opening `$$` **MUST** be the sole non-whitespace content on its line, as must the closing `$$`. The Parser **MUST** emit an `RdxTextNode` of type `"math_display"`.

```rdx
$$
E = mc^2
$$
```

Display math blocks **MUST NOT** be parsed inside fenced code blocks.

### **2.7. HTML Pass-Through**

Standard HTML elements (tags beginning with a lowercase letter, e.g., `<div>`, `<span>`, `<img>`) **MUST** be handled according to the CommonMark specification for raw HTML. The Parser **MUST NOT** interpret lowercase tags as RDX Components. They **MUST** be emitted as `RdxStandardBlockNode` (type `"html"`) or as inline raw HTML within an `RdxTextNode`, depending on their placement per CommonMark rules. Unlike MDX, which strips raw HTML and requires `rehype-raw` to restore it, RDX passes through HTML natively.

## **3. Parsing Rules & Error Handling**

To ensure deterministic output and high performance across different Parser implementations, the following strict error-handling rules **MUST** be applied:

1. **Malformed JSON Attributes**: If the contents of a `{{ }}` attribute fail standard JSON validation, the Parser **MUST NOT** silently fall back to emitting a string. Instead, it **MUST** emit an `RdxErrorNode` in place of the attribute value or component, allowing the Host Environment to render an explicit Error Boundary.
2. **Unclosed Tags (No Backtracking)**: To prevent infinite lookahead bottlenecks, if an RDX Component block tag is opened but not closed before the end of the document or its parent block, the Parser **MUST NOT** backtrack to treat the opening tag as literal text. The Parser **MUST** treat this as a fatal block syntax error and emit an `RdxErrorNode` for that specific block.
3. **Misnested Tags**: If a closing tag does not match the most recently opened tag (e.g., `<A><B></A>`), the Parser **MUST** emit an `RdxErrorNode` for the misnested block. The Parser **MUST NOT** attempt auto-correction or implicit closing.
4. **Unrecognized Attributes**: The Parser **MUST** process all structurally valid attributes and emit them to the AST. It is the responsibility of the Host Environment to ignore attributes it does not recognize.
5. **Invalid Variable Paths**: If a `{$...}` expression contains a path that does not conform to the grammar defined in Section 2.4.1, the Parser **MUST** emit an `RdxErrorNode` rather than silently treating it as literal text.

## **4. The RDX Abstract Syntax Tree (AST) Schema**

An RDX Parser **MUST** output an AST that conforms to the following formal schema. The AST is the canonical contract between the Parser and the Host Environment.

### **4.1. Positional Data (Source Maps)**

To support error boundaries and IDE tooling, every node **MUST** include an `RdxPosition` interface mapping back to the source `.rdx` file. Line numbers are 1-indexed. Column numbers are 1-indexed. Offsets are 0-indexed byte offsets from the beginning of the document.

```ts
interface RdxPosition {
  start: { line: number; column: number; offset: number };
  end: { line: number; column: number; offset: number };
}
```

### **4.2. TypeScript Interface Definition**

```ts
// The root of the RDX Document
interface RdxRoot {
  type: "root";
  frontmatter: Record<string, any> | null;
  children: RdxNode[];
  position: RdxPosition;
}

// A union type of all possible RDX nodes
type RdxNode =
  | RdxTextNode
  | RdxStandardBlockNode
  | RdxCodeBlockNode
  | RdxLinkNode
  | RdxImageNode
  | RdxFootnoteNode
  | RdxComponentNode
  | RdxVariableNode
  | RdxErrorNode;

// Standard CommonMark Block
type RdxStandardBlockType =
  | "paragraph"
  | "heading"
  | "list"
  | "list_item"
  | "blockquote"
  | "thematic_break"
  | "html"
  | "table"
  | "table_row"
  | "table_cell"
  | "emphasis"
  | "strong"
  | "strikethrough";

interface RdxStandardBlockNode {
  type: RdxStandardBlockType;
  depth?: number;    // Present on headings (1-6)
  ordered?: boolean; // Present on lists
  checked?: boolean; // Present on list_item when task list syntax is used
  children: RdxNode[];
  position: RdxPosition;
}

// Fenced Code Block
interface RdxCodeBlockNode {
  type: "code_block";
  value: string;      // The code content
  lang?: string;      // Language identifier from the info string (e.g., "rust", "js")
  meta?: string;      // Remainder of the info string after the language identifier
  position: RdxPosition;
}

// Link Node
interface RdxLinkNode {
  type: "link";
  url: string;        // The link destination
  title?: string;     // Optional link title (from `[text](url "title")`)
  children: RdxNode[];
  position: RdxPosition;
}

// Image Node
interface RdxImageNode {
  type: "image";
  url: string;        // The image source URL
  title?: string;     // Optional image title
  alt?: string;       // Alt text (may also appear as children)
  children: RdxNode[];
  position: RdxPosition;
}

// Footnote Node (definition or reference)
interface RdxFootnoteNode {
  type: "footnote_definition" | "footnote_reference";
  label: string;      // The footnote label (e.g., "1", "note")
  children: RdxNode[]; // Content for definitions; empty for references
  position: RdxPosition;
}

// RDX Component Definition
interface RdxComponentNode {
  type: "component";
  name: string; // The uppercase tag name, e.g., "Chart"
  isInline: boolean;
  attributes: RdxAttributeNode[];
  children: RdxNode[]; // MUST be an empty array [] for self-closing components
  position: RdxPosition;
}

// A single attribute with its own positional data
interface RdxAttributeNode {
  name: string;
  value: RdxAttributeValue;
  position: RdxPosition;
}

// Supported Attribute Values
type RdxAttributeValue =
  | string
  | number
  | boolean
  | null
  | Record<string, any> // Parsed JSON Objects
  | any[]               // Parsed JSON Arrays
  | RdxVariableNode;    // Passed Variables

// Literal Text Node
interface RdxTextNode {
  type: "text" | "code_inline" | "math_inline" | "math_display";
  value: string;
  position: RdxPosition;
}

// Variable Interpolation Node
interface RdxVariableNode {
  type: "variable";
  path: string; // The literal path, e.g., "frontmatter.title"
  position: RdxPosition;
}

// Explicit Error Node for Host-level Error Boundaries
interface RdxErrorNode {
  type: "error";
  message: string;
  rawContent: string; // The unparsed/failed raw string
  position: RdxPosition;
}
```

**Notes on dedicated node types:**

* `"code_block"`, `"link"`, `"image"`, and `"footnote_definition"` / `"footnote_reference"` use dedicated interfaces instead of `RdxStandardBlockNode` because they carry domain-specific fields (`lang`, `url`, `title`, `label`, etc.) that do not apply to generic block nodes.
* `"math_inline"` and `"math_display"` support LaTeX math expressions per Section 2.6. The Parser handles the `$` delimiter natively, disambiguating from `{$var}` variable syntax by the rule that `{$` always begins a variable expression.

### **4.3. Example AST Serialization**

**Source Document:**

```rdx
<Badge status="beta" active={true}>New Feature</Badge>
```

**Compliant AST Output:**

```json
{
  "type": "root",
  "frontmatter": null,
  "children": [
    {
      "type": "component",
      "name": "Badge",
      "isInline": false,
      "attributes": [
        {
          "name": "status",
          "value": "beta",
          "position": {
            "start": { "line": 1, "column": 8, "offset": 7 },
            "end": { "line": 1, "column": 22, "offset": 21 }
          }
        },
        {
          "name": "active",
          "value": true,
          "position": {
            "start": { "line": 1, "column": 23, "offset": 22 },
            "end": { "line": 1, "column": 36, "offset": 35 }
          }
        }
      ],
      "position": {
        "start": { "line": 1, "column": 1, "offset": 0 },
        "end": { "line": 1, "column": 55, "offset": 54 }
      },
      "children": [
        {
          "type": "text",
          "value": "New Feature",
          "position": {
            "start": { "line": 1, "column": 37, "offset": 36 },
            "end": { "line": 1, "column": 48, "offset": 47 }
          }
        }
      ]
    }
  ],
  "position": {
    "start": { "line": 1, "column": 1, "offset": 0 },
    "end": { "line": 1, "column": 55, "offset": 54 }
  }
}
```

## **5. Security Considerations**

By design, RDX is a secure data transport format, not an executable language.

1. **No Code Execution**: A compliant Parser **MUST NOT** include JavaScript engines (like V8) or invoke `eval()` during parsing.
2. **XSS Mitigation**: Because the AST enforces a strict separation between attribute definitions (data) and executable logic (handled by the Host Environment), attackers cannot inject cross-site scripting (XSS) payloads via component properties.
3. **Renderer Responsibility**: The Host Environment **MUST** sanitize any `RdxTextNode`, `RdxStandardBlockNode`, `RdxLinkNode`, or `RdxImageNode` before rendering it as raw HTML to prevent standard Markdown-based XSS vectors.
4. **Resource Limits**: Parser implementations **SHOULD** enforce reasonable limits on document size, nesting depth, and attribute count to prevent denial-of-service via pathologically crafted documents.

## **6. Conformance**

An implementation is considered a **Compliant RDX Parser** if it:

1. Successfully parses standard CommonMark.
2. Supports GFM extensions: tables, strikethrough, task lists, and footnotes per Section 2.
3. Identifies and parses YAML frontmatter delimited by `---` at line 1 of the document, prior to standard evaluation.
4. Correctly identifies all tags matching the grammar `[A-Z][a-zA-Z0-9_]*` as `RdxComponentNode` types, distinguishing between block and inline placement per Section 2.2.4.
5. Correctly implements backslash escaping per Section 2.5, and does not interpolate variables within code spans or code blocks per Section 2.4.2.
6. Emits `RdxErrorNode` elements rather than silently failing or backtracking infinitely on unclosed tags, misnested tags, malformed JSON properties, or invalid variable paths.
7. Outputs an AST structure conforming exactly to the schema defined in Section 4, including required positional mapping on every node. Dedicated node types (`RdxCodeBlockNode`, `RdxLinkNode`, `RdxImageNode`, `RdxFootnoteNode`) **MUST** be used where specified.
8. Passes through standard HTML elements (lowercase tags) per CommonMark rules without interpreting them as RDX Components.
9. Complies with the zero-execution security mandates outlined in Section 5.
