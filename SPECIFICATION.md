# **RDX (Reactive Document eXpressions) Specification**

**Version:** 1.0.0-rc2

**Status:** Release Candidate

**File Extension:** `.rdx`

## **1. Introduction**

Reactive Document eXpressions (RDX) is a strictly typed, declarative extension to the CommonMark specification. It defines a standardized syntax and Abstract Syntax Tree (AST) for embedding interactive components and structured data within Markdown documents.

The primary goal of RDX is to provide a safe, parser-agnostic standard for "Docs as Data," allowing technical writers to author rich components without the security and performance implications of arbitrary JavaScript execution (as seen in imperative formats like MDX).

### **1.1. Terminology**

The key words **"MUST"**, **"MUST NOT"**, **"REQUIRED"**, **"SHALL"**, **"SHALL NOT"**, **"SHOULD"**, **"SHOULD NOT"**, **"RECOMMENDED"**, **"MAY"**, and **"OPTIONAL"** in this document are to be interpreted as described in [RFC 2119](https://datatracker.ietf.org/doc/html/rfc2119).

- **Parser**: A software library that consumes an `.rdx` string and outputs a compliant RDX AST.
- **Host Environment / Renderer**: The downstream application that consumes the RDX AST to produce a final output (e.g., HTML, native UI elements, or intermediate framework code).
- **Component**: A custom, interactive element defined by an uppercase tag within the document.

### **1.2. Versioning & Forward Compatibility**

This specification follows [Semantic Versioning 2.0.0](https://semver.org/). A Parser built for version `1.x` **SHOULD** gracefully handle documents authored for a later `1.y` (where `y > x`) by emitting an `RdxErrorNode` for any syntactic construct it does not recognize, rather than failing catastrophically. A major version increment (e.g., `2.0.0`) **MAY** introduce breaking changes to the AST schema or syntax grammar.

## **2. Syntax Grammar**

RDX is a superset of CommonMark. Any valid CommonMark document is a valid RDX document. RDX introduces new normative syntactic structures: Frontmatter, Components, Strictly-Typed Attributes, Escaping, and Context Variables.

A compliant Parser **MUST** also support the following GitHub Flavored Markdown (GFM) extensions:

- **Tables**: Pipe-based table syntax per the GFM specification.
- **Strikethrough**: `~~text~~` syntax, emitted as an `RdxStandardBlockNode` of type `"strikethrough"`.
- **Task Lists**: `- [x]` and `- [ ]` syntax within list items. The Parser **MUST** set the `checked` field on the corresponding `list_item` node.
- **Footnotes**: `[^label]` references and `[^label]: content` definitions. The Parser **MUST** emit `RdxFootnoteNode` nodes for both references and definitions.

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

- **Self-Closing Tags**: Components that do not contain child nodes **MUST** end with `/>`. A space before the `/>` is **OPTIONAL** (both `<Badge />` and `<Badge/>` are valid).

```rdx
<HeroImage src="/assets/hero.png" />
```

- **Block Tags**: Components that wrap child nodes **MUST** have an explicit opening tag and a matching closing tag.

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

- **Block-Level**: A component is block-level if it is separated from surrounding content by blank lines (one or more empty lines before and/or after), or if it is the sole non-whitespace content on its line. A block-level component's closing tag **MUST** also be on its own line or be the sole non-whitespace content on its line.
- **Inline-Level**: A component is inline-level if it appears within a paragraph or other inline context alongside non-whitespace text, without being separated by blank lines. (e.g., `Here is a <Badge status="new" /> for this feature.`)

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

Boolean and Number literals **MUST** be enclosed in single curly braces `{}`. Valid contents are restricted strictly to: integers, floats (including negative values and scientific notation, e.g., `-2.75`, `2.5e10`), `true`, `false`, and `null`.

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

_Note: Variable resolution is the responsibility of the Host Environment. The Parser merely identifies and emits the variable path in the AST._

#### **2.4.2. Variable Scope in Code Constructs**

The Parser **MUST NOT** evaluate `{$variable_path}` syntax inside CommonMark inline code spans (`` ` ` ``) or fenced code blocks (` ``` `). Text within these constructs **MUST** be emitted as literal `RdxTextNode` values with no variable interpolation.

### **2.5. Escaping**

To allow authors to render RDX syntax characters literally, the Parser **MUST** support backslash escaping for the following sequences:

| Source Text | Parsed Output | Description                   |
| ----------- | ------------- | ----------------------------- |
| `\{$path}`  | `{$path}`     | Escape variable interpolation |
| `\{@ref}`   | `{@ref}`      | Escape cross-reference        |
| `\[@`       | `[@`          | Escape citation opening       |
| `\$`        | `$`           | Escape math delimiter         |
| `\{{`       | `{{`          | Escape double-brace opening   |
| `\}}`       | `}}`          | Escape double-brace closing   |
| `\{`        | `{`           | Escape single-brace opening   |
| `\\`        | `\`           | Literal backslash             |

When the Parser encounters a backslash immediately preceding one of the above sequences, it **MUST** strip the backslash and emit the remaining characters as literal text in an `RdxTextNode`. A backslash preceding any other character **MUST** be passed through as-is (i.e., treated as a literal backslash), consistent with CommonMark's escaping behavior.

_Note:_ The `\$` escape is critical for documents containing currency values adjacent to mathematical notation. Without it, text like `The cost is \$10 per unit` would be ambiguously parsed. The `$` character is also escapable under standard CommonMark (it is ASCII punctuation per the CommonMark spec, Section 6.1), but RDX lists it explicitly because `$` has additional semantic meaning as a math delimiter.

### **2.6. Math (LaTeX)**

RDX natively supports mathematical expressions using dollar-sign delimiters, without requiring external plugins.

#### **2.6.1. Inline Math**

Inline math expressions are delimited by single dollar signs: `$expression$`. The Parser **MUST** emit an `RdxTextNode` of type `"math_inline"` with the expression content as its `value`.

To avoid ambiguity with variable interpolation (`{$path}`), the `$` character **MUST NOT** be interpreted as a math delimiter when immediately preceded by `{`. The sequence `{$` always begins a variable expression.

Additional constraints for inline math:

- The content between delimiters **MUST NOT** be empty.
- The content **MUST NOT** start or end with a space character.

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

#### **2.6.3. Lexical Scope Precedence**

Math delimiters establish a **lexical boundary**. Once the Parser enters a math expression (upon encountering an opening `$` or `$$`), the following rules apply until the matching closing delimiter:

1. **No Component parsing**: The characters `<`, `>`, `/>` **MUST NOT** be interpreted as component tag syntax inside math. They are literal characters (less-than, greater-than, etc.) in the mathematical expression.
2. **No Variable interpolation**: The sequence `{$...}` inside math **MUST** be interpreted as LaTeX grouping (`{`) followed by a literal `$` and subsequent characters. It **MUST NOT** trigger variable parsing.
3. **No Citation or Cross-Reference parsing**: The sequences `[@` and `{@` inside math **MUST** be treated as literal characters.
4. **No JSON attribute parsing**: Curly braces `{` and `}` inside math **MUST** be treated as LaTeX grouping delimiters.

In summary: math expressions are **opaque** to all other RDX syntax extensions. Only the LaTeX math grammar (Section 2.11) applies within math delimiters.

### **2.7. Citations**

RDX natively supports academic citation references using bracket-at syntax, without requiring external plugins or components.

#### **2.7.1. Inline Citation Syntax**

A citation reference is delimited by `[@` and `]`. The Parser **MUST** emit an `RdxCitationNode` when it encounters this pattern in text content.

```rdx
Recent work [@smith2024] has shown significant improvements.
```

To disambiguate from standard CommonMark link syntax (`[text](url)`), the following lexical rules apply:

1. If a `[@...]` sequence is immediately followed by `(` (forming the pattern `[@...](...)` with no intervening whitespace), the Parser **MUST** treat the entire construct as a standard CommonMark link, **NOT** a citation. This prevents ambiguity with social media profile links such as `[@username](https://example.com)`.
2. If a `[@...]` sequence is **NOT** immediately followed by `(`, the Parser **MUST** interpret it as a citation reference and emit an `RdxCitationNode`.

#### **2.7.2. Citation Key Grammar**

A citation key **MUST** match the following pattern:

```
[a-zA-Z][a-zA-Z0-9_:./-]*
```

The key **MUST** begin with an ASCII letter and **MAY** contain letters, digits, underscores, colons, dots, slashes, and hyphens. This grammar is compatible with BibTeX and CSL citation key conventions.

#### **2.7.3. Citation Groups**

Multiple citations **MAY** be grouped within a single bracket pair, separated by semicolons. The Parser **MUST** emit a single `RdxCitationNode` containing all keys.

```rdx
This has been widely studied [@smith2024; @jones2023; @chen2025].
```

#### **2.7.4. Locators and Affixes**

Each citation key **MAY** be followed by a comma and a locator string. The locator is free-form text that specifies a page, chapter, section, or other subdivision within the cited work.

```rdx
As noted by [@smith2024, p. 42] and [@jones2023, ch. 3, pp. 100-115].
```

Each citation key **MAY** also carry a prefix string, placed before the `@` sign within the citation group:

```rdx
[see @smith2024, p. 42; also @jones2023]
```

The Parser **MUST** extract the prefix (text before `@`), the key (text after `@` up to `,` or `;` or `]`), and the locator (text after the first `,` up to `;` or `]`) for each entry.

#### **2.7.5. Escaping**

The sequence `\[@` **MUST** be treated as literal text `[@`, consistent with the escaping rules defined in Section 2.5. This prevents unintended citation parsing.

#### **2.7.6. Scope Restrictions**

The Parser **MUST NOT** interpret citation syntax inside:

- Inline code spans (`` ` ` ``)
- Fenced code blocks (` ``` `)
- String attribute values

This is consistent with the scope restrictions on variables (Section 2.4.2).

### **2.8. Cross-References**

RDX supports internal cross-references to labeled elements within the same document. Cross-references allow authors to refer to headings, figures, equations, and other labeled elements by their identifier.

#### **2.8.1. Cross-Reference Syntax**

A cross-reference is delimited by `{@` and `}`. The Parser **MUST** emit an `RdxCrossRefNode` when it encounters this pattern in text content.

```rdx
As shown in {@fig:architecture}, the system consists of three layers.
See {@eq:euler} for the derivation.
```

To disambiguate from variable interpolation (`{$path}`), the following lexical rule applies: the sequence `{@` **MUST** be interpreted as the start of a cross-reference. The sequence `{$` **MUST** be interpreted as the start of a variable. All other `{` characters follow existing escaping and literal rules.

#### **2.8.2. Cross-Reference Target Grammar**

A cross-reference target **MUST** match the following pattern:

```
[a-zA-Z_][a-zA-Z0-9_:.-]*
```

The target **MUST** begin with an ASCII letter or underscore and **MAY** contain letters, digits, underscores, colons, dots, and hyphens. The colon character is **RECOMMENDED** as a namespace separator for conventionally-typed references (e.g., `fig:`, `eq:`, `tbl:`, `lst:`, `thm:`, `sec:`).

#### **2.8.3. Resolution**

The Parser **MUST NOT** resolve cross-references. Resolution — mapping a target identifier to a display string such as "Figure 3" or "Equation 7 on page 42" — is the responsibility of the Host Environment or a downstream transform. The Parser emits the raw target string in the AST.

#### **2.8.4. Escaping**

The sequence `\{@` **MUST** be treated as literal text `{@`, consistent with Section 2.5.

#### **2.8.5. Scope Restrictions**

The Parser **MUST NOT** interpret cross-reference syntax inside inline code spans, fenced code blocks, or string attribute values.

### **2.9. Definition Lists**

RDX extends CommonMark with definition list syntax for structured term-definition pairs. This syntax is derived from PHP Markdown Extra and is compatible with Pandoc's definition list extension.

#### **2.9.1. Definition List Syntax**

A definition list consists of one or more term-definition pairs. A **term** is one or more consecutive non-blank lines. A **definition** is a block-level construct that begins with a colon-space (`: `) at the start of a line, immediately following the term (with no intervening blank line between term and first definition).

```rdx
Algorithm
: A finite sequence of well-defined instructions.

Parser
: A software component that analyzes a string of symbols
  according to the rules of a formal grammar.
: In the context of RDX, the component that produces the AST.
```

#### **2.9.2. Parsing Rules**

1. A definition list **MUST** be preceded by a blank line (or appear at the start of the document or the start of a container block such as a blockquote or component). This prevents accidental triggering by lines that happen to begin with `: ` in running prose, dialogue, or transcripts.
2. The term line **MUST** contain at least one non-whitespace character.
3. The definition marker `: ` (colon followed by at least one space) **MUST** appear at the start of a line (column 1), or indented by up to 3 spaces, immediately following the term or a preceding definition. There **MUST NOT** be a blank line between the term and its first definition.
4. A term **MAY** have multiple definitions. Each definition begins with its own `: ` marker.
5. Definition content **MAY** span multiple lines. Continuation lines **MUST** be indented by at least 2 spaces (or 1 tab) from the definition marker.
6. A blank line between consecutive term-definition pairs is **OPTIONAL**. A blank line within a definition triggers "loose" rendering (each definition body wrapped in a paragraph), consistent with CommonMark list behavior.
7. The Parser **MUST** emit an `RdxStandardBlockNode` of type `"definition_list"` containing children of type `"definition_term"` and `"definition_description"`.

**Rationale for Rule 1:** Without the blank line requirement, a line starting with `: ` after ordinary paragraph text would ambiguously match both "continuation of a paragraph" and "start of a definition." Requiring a blank line before the term ensures that definition lists are always intentional. This is consistent with Pandoc's definition list behavior.

#### **2.9.3. Inline Content in Terms**

Terms **MAY** contain inline CommonMark formatting (emphasis, strong, code spans, links) and inline RDX constructs (variables, inline components, inline math). Terms **MUST NOT** contain block-level content.

### **2.10. Display Math Labels**

Display math blocks (Section 2.6.2) **MAY** carry a label for cross-referencing. The label syntax is `{#identifier}` appended to the opening `$$` line.

```rdx
$$ {#eq:euler}
e^{i\pi} + 1 = 0
$$
```

#### **2.10.1. Label Grammar**

The label identifier **MUST** conform to the cross-reference target grammar defined in Section 2.8.2. The `{#` sequence and the closing `}` are delimiters and **MUST NOT** be included in the emitted label string.

#### **2.10.2. Parsing Rules**

1. The label **MUST** appear on the same line as the opening `$$`, separated by optional whitespace.
2. The `$$` and the `{#identifier}` together **MUST** be the sole non-whitespace content on the opening line.
3. If the label syntax is malformed (e.g., `{#}` with empty identifier, or unclosed `{#`), the Parser **MUST** emit an `RdxErrorNode`.
4. The closing `$$` line **MUST NOT** carry a label.

#### **2.10.3. Interaction with Cross-References**

A display math label creates a referenceable anchor. The cross-reference `{@eq:euler}` targets the display math block labeled `{#eq:euler}`. Resolution of this reference to display text (e.g., "Equation 3") is the Host Environment's responsibility.

### **2.11. Structured Math AST**

A compliant Parser **MUST** parse the content of math expressions (both inline and display) into a structured `RdxMathExpr` tree, in addition to preserving the raw LaTeX source string. The structured tree enables downstream consumers to perform glyph layout, accessibility annotation, and semantic analysis without re-parsing LaTeX.

#### **2.11.1. Dual Representation**

Each math node **MUST** contain two representations of the same expression:

1. **`raw`**: The original LaTeX source string, verbatim. This field exists for backward-compatible rendering by Host Environments that delegate to client-side math libraries (e.g., KaTeX, MathJax).
2. **`tree`**: A structured `RdxMathExpr` tree produced by parsing the `raw` string. This field exists for Host Environments that perform native math layout (e.g., PDF renderers using OpenType MATH tables).

A Host Environment **MAY** use either or both representations. A compliant Parser **MUST** emit both.

#### **2.11.2. Math Expression Grammar**

The Parser **MUST** recognize and structurally parse the following LaTeX math constructs. Constructs are organized into three tiers of conformance:

**Tier 1 (REQUIRED):** A compliant Parser **MUST** parse these constructs into their corresponding `RdxMathExpr` nodes.

- **Identifiers**: Single ASCII letters (`a`–`z`, `A`–`Z`), Greek letters (`\alpha` through `\omega`, `\Alpha` through `\Omega`), and other standard mathematical symbols (`\infty`, `\partial`, `\nabla`, `\ell`, `\hbar`).
- **Numbers**: Sequences of digits, optionally containing a decimal point.
- **Operators**: Binary operators (`+`, `-`, `\times`, `\cdot`, `\pm`, `\mp`, `\div`), relational operators (`=`, `\neq`, `<`, `>`, `\leq`, `\geq`, `\approx`, `\equiv`, `\sim`, `\cong`, `\propto`), set operators (`\in`, `\notin`, `\subset`, `\supset`, `\cup`, `\cap`), and logical operators (`\land`, `\lor`, `\neg`, `\implies`, `\iff`).
- **Superscripts and subscripts**: `x^{expr}`, `x_{expr}`, `x_{expr}^{expr}`. A single-character exponent or subscript without braces (`x^2`, `x_i`) **MUST** also be recognized.
- **Fractions**: `\frac{numerator}{denominator}`, `\dfrac{...}{...}`, `\tfrac{...}{...}`.
- **Square roots**: `\sqrt{expr}`, `\sqrt[index]{expr}`.
- **Delimiters**: `\left(`, `\right)`, `\left[`, `\right]`, `\left\{`, `\right\}`, `\left|`, `\right|`, `\left\langle`, `\right\rangle`, and `\left.` / `\right.` (invisible delimiters).
- **Large operators**: `\sum`, `\prod`, `\int`, `\iint`, `\iiint`, `\oint`, `\bigcup`, `\bigcap`, with optional subscript and superscript limits.
- **Text within math**: `\text{...}`, `\mathrm{...}`, `\mathit{...}`.
- **Basic spacing**: `\,` (thin), `\;` (medium), `\:` (medium), `\!` (negative thin), `\quad`, `\qquad`.
- **Grouping**: Curly braces `{...}` for grouping subexpressions (braces are not emitted as delimiters; they control parse structure).

**Tier 2 (RECOMMENDED):** A compliant Parser **SHOULD** parse these constructs. If a Tier 2 construct is not supported, the Parser **MUST** emit an `RdxMathExpr` of variant `Error` containing the raw LaTeX fragment, rather than silently dropping it.

- **Matrices**: `\begin{pmatrix}...\end{pmatrix}`, `\begin{bmatrix}`, `\begin{vmatrix}`, `\begin{Bmatrix}`, `\begin{Vmatrix}`, `\begin{matrix}`. Rows separated by `\\`, cells by `&`.
- **Cases**: `\begin{cases}...\end{cases}`.
- **Alignment environments**: `\begin{align}...\end{align}`, `\begin{align*}`, `\begin{gather}`, `\begin{gather*}`, `\begin{alignat}`, `\begin{alignat*}`. Star variants suppress equation numbering.
- **Accents**: `\hat{x}`, `\tilde{x}`, `\vec{x}`, `\dot{x}`, `\ddot{x}`, `\bar{x}`, `\acute{x}`, `\grave{x}`, `\breve{x}`, `\check{x}`, `\widehat{...}`, `\widetilde{...}`.
- **Over/under constructions**: `\overline{...}`, `\underline{...}`, `\overbrace{...}`, `\underbrace{...}`, `\overset{above}{base}`, `\underset{below}{base}`, `\stackrel{above}{base}`.
- **Named operators**: `\lim`, `\max`, `\min`, `\sup`, `\inf`, `\log`, `\ln`, `\sin`, `\cos`, `\tan`, `\exp`, `\det`, `\gcd`, `\arg`, `\dim`, `\ker`, `\deg`, `\hom`, `\Pr`. The Parser **MUST** recognize these as operators (upright font in output), not identifiers.
- **Font overrides**: `\mathbb{...}`, `\mathcal{...}`, `\mathfrak{...}`, `\mathscr{...}`, `\boldsymbol{...}`, `\mathsf{...}`, `\mathtt{...}`, `\mathbf{...}`.
- **Arrows**: `\to`, `\rightarrow`, `\leftarrow`, `\Rightarrow`, `\Leftarrow`, `\leftrightarrow`, `\Leftrightarrow`, `\mapsto`, `\hookrightarrow`, `\hookleftarrow`, `\uparrow`, `\downarrow`.
- **Dots**: `\dots`, `\cdots`, `\ldots`, `\vdots`, `\ddots`.

**Tier 3 (OPTIONAL):** A compliant Parser **MAY** parse these constructs. If unsupported, the Parser **MUST** emit an `Error` variant.

- **Phantoms and smash**: `\phantom{...}`, `\hphantom{...}`, `\vphantom{...}`, `\smash{...}`, `\smash[t]{...}`, `\smash[b]{...}`.
- **Style overrides**: `\displaystyle`, `\textstyle`, `\scriptstyle`, `\scriptscriptstyle`.
- **Math choice**: `\mathchoice{D}{T}{S}{SS}`.
- **Color**: `\color{name}{expr}`, `\textcolor{name}{expr}`.
- **Chemistry**: `\ce{...}` (mhchem). The Parser **MAY** store the content as an opaque string in a `Chem` variant rather than parsing chemical formula syntax.
- **Custom operators**: `\operatorname{name}`.
- **Extensible arrows**: `\xrightarrow[below]{above}`, `\xleftarrow[below]{above}`.
- **Array environments**: `\begin{array}{col_spec}...\end{array}` with column alignment specifiers.
- **Commutative diagrams**: `\begin{CD}...\end{CD}`.

#### **2.11.3. Macro Expansion**

If the document's YAML frontmatter contains a `math-macros` field, the Parser **SHOULD** expand these macros within math expressions before constructing the `RdxMathExpr` tree. The `raw` field **MUST** contain the original unexpanded source. The `tree` field **MUST** reflect the expanded expression.

```yaml
---
math-macros:
  \R: \mathbb{R}
  \norm#1: \left\lVert #1 \right\rVert
---
```

Macro definitions **MUST** specify arity using `#N` suffixes on the macro name (e.g., `\norm#1` takes one argument). The Parser **MUST** enforce a maximum expansion depth of 64 to prevent infinite recursion. If expansion exceeds this depth, the Parser **MUST** emit an `Error` variant for the affected expression.

#### **2.11.4. Error Recovery**

When the Parser encounters an unrecognized LaTeX command or malformed construct within a math expression, it **MUST NOT** discard the expression or halt parsing. Instead, the Parser **MUST**:

1. Emit an `RdxMathExpr` of variant `Error` containing the raw fragment and a human-readable error message.
2. Continue parsing the remainder of the expression. Surrounding valid constructs **MUST** still be parsed into their correct `RdxMathExpr` variants.

The `raw` field on the enclosing `RdxMathNode` always contains the complete, unmodified LaTeX source, providing a fallback for Host Environments that delegate to external math renderers.

### **2.12. Code Block Info String**

The info string on a fenced code block (the text following the opening ` ``` ` fence) carries structured metadata in addition to the language identifier. A compliant Parser **MUST** extract structured fields from the info string.

#### **2.12.1. Parsing Rules**

The info string is parsed left-to-right as follows:

1. **Language identifier**: The first whitespace-delimited token. The Parser **MUST** emit this as the `lang` field on `RdxCodeBlockNode`. If the info string is empty, `lang` **MUST** be `null`.
2. **Structured fields**: The remainder of the info string after the language identifier **MUST** be parsed for key-value pairs and bare flags. The Parser **MUST** emit recognized fields as structured data on `RdxCodeBlockNode`.

Recognized fields:

| Field             | Syntax                  | Type             | Description                                                        |
| ----------------- | ----------------------- | ---------------- | ------------------------------------------------------------------ |
| `title`           | `title="filename"`      | `string`         | Display title or filename for the code block.                      |
| `highlight`       | `{3-5,12,18-20}`        | `number range[]` | Line numbers or ranges to visually emphasize.                      |
| `showLineNumbers` | `showLineNumbers`       | `boolean` (flag) | Toggle line number display.                                        |
| `diff`            | `diff`                  | `boolean` (flag) | Render as a unified diff (lines starting with `+`/`-` are styled). |
| `caption`         | `caption="Description"` | `string`         | Caption text for numbered code listings.                           |

The `highlight` field uses a compact range syntax enclosed in curly braces: `{start-end}` for ranges, `{line}` for single lines, separated by commas. The Parser **MUST** normalize ranges into a sorted, deduplicated list of line numbers.

Any fields not matching the above **MUST** be preserved verbatim in the existing `meta` field as a fallback string.

````rdx
```rust title="src/main.rs" {3-5,12} showLineNumbers
fn main() {
    let parser = Parser::new();
    let ast = parser.parse(input);  // highlighted
    let result = transform(ast);    // highlighted
    println!("{:?}", result);       // highlighted
}
````

````

#### **2.12.2. Inline Code Language Hint**

Inline code spans (`` ` ` ``) **MAY** carry a trailing language hint enclosed in curly braces immediately following the closing backtick, with no intervening whitespace.

```rdx
Use the `SELECT * FROM users`{sql} query to fetch all records.
````

The Parser **MUST** recognize the pattern `` `content`{identifier} `` where `identifier` matches `[a-zA-Z][a-zA-Z0-9_-]*`. When present, the Parser **MUST** emit an `RdxCodeInlineNode` (a new dedicated type, distinct from `RdxTextNode`) with both `value` and `lang` fields.

If the `{identifier}` is not present, the Parser **MUST** emit the inline code as an `RdxTextNode` of type `"code_inline"` per existing behavior (Section 4.2), for backward compatibility.

### **2.13. HTML Pass-Through**

Standard HTML elements (tags beginning with a lowercase letter, e.g., `<div>`, `<span>`, `<img>`) **MUST** be handled according to the CommonMark specification for raw HTML. The Parser **MUST NOT** interpret lowercase tags as RDX Components. They **MUST** be emitted as `RdxStandardBlockNode` (type `"html"`) or as inline raw HTML within an `RdxTextNode`, depending on their placement per CommonMark rules. Unlike MDX, which strips raw HTML and requires `rehype-raw` to restore it, RDX passes through HTML natively.

## **3. Parsing Rules & Error Handling**

To ensure deterministic output and high performance across different Parser implementations, the following strict error-handling rules **MUST** be applied:

1. **Malformed JSON Attributes**: If the contents of a `{{ }}` attribute fail standard JSON validation, the Parser **MUST NOT** silently fall back to emitting a string. Instead, it **MUST** emit an `RdxErrorNode` in place of the attribute value or component, allowing the Host Environment to render an explicit Error Boundary.
2. **Unclosed Tags (No Backtracking)**: To prevent infinite lookahead bottlenecks, if an RDX Component block tag is opened but not closed before the end of the document or its parent block, the Parser **MUST NOT** backtrack to treat the opening tag as literal text. The Parser **MUST** treat this as a fatal block syntax error and emit an `RdxErrorNode` for that specific block.
3. **Misnested Tags**: If a closing tag does not match the most recently opened tag (e.g., `<A><B></A>`), the Parser **MUST** emit an `RdxErrorNode` for the misnested block. The Parser **MUST NOT** attempt auto-correction or implicit closing.
4. **Unrecognized Attributes**: The Parser **MUST** process all structurally valid attributes and emit them to the AST. It is the responsibility of the Host Environment to ignore attributes it does not recognize.
5. **Invalid Variable Paths**: If a `{$...}` expression contains a path that does not conform to the grammar defined in Section 2.4.1, the Parser **MUST** emit an `RdxErrorNode` rather than silently treating it as literal text.
6. **Invalid Cross-Reference Targets**: If a `{@...}` expression contains a target that does not conform to the grammar defined in Section 2.8.2, the Parser **MUST** emit an `RdxErrorNode`.
7. **Malformed Citation References**: If `[@` is encountered but the content does not contain at least one valid citation key per Section 2.7.2, the Parser **MUST** emit an `RdxErrorNode`.
8. **Malformed Display Math Labels**: If the opening `$$` line contains `{#` but the label does not conform to Section 2.10.1, the Parser **MUST** emit an `RdxErrorNode`.
9. **Math Expression Parse Errors**: If a LaTeX construct within a math expression cannot be parsed (Section 2.11.4), the Parser **MUST** emit an `RdxMathExpr` of variant `Error` containing the raw fragment. The enclosing `RdxMathInlineNode` or `RdxMathDisplayNode` **MUST** still be emitted (not replaced by `RdxErrorNode`), since the `raw` field provides a complete fallback.

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
  | RdxCodeInlineNode
  | RdxMathInlineNode
  | RdxMathDisplayNode
  | RdxLinkNode
  | RdxImageNode
  | RdxFootnoteNode
  | RdxCitationNode
  | RdxCrossRefNode
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
  | "strikethrough"
  | "definition_list"
  | "definition_term"
  | "definition_description";

interface RdxStandardBlockNode {
  type: RdxStandardBlockType;
  depth?: number; // Present on headings (1-6)
  ordered?: boolean; // Present on lists
  checked?: boolean; // Present on list_item when task list syntax is used
  children: RdxNode[];
  position: RdxPosition;
}

// Fenced Code Block
interface RdxCodeBlockNode {
  type: "code_block";
  value: string; // The code content
  lang?: string; // Language identifier from the info string (e.g., "rust", "js")
  title?: string; // Display title or filename (from info string `title="..."`)
  highlight?: number[]; // Sorted, deduplicated line numbers to emphasize
  showLineNumbers?: boolean; // Whether to display line numbers
  diff?: boolean; // Whether to render as a unified diff
  caption?: string; // Caption text for numbered code listings
  meta?: string; // Unrecognized remainder of the info string (fallback)
  position: RdxPosition;
}

// Inline Code with Language Hint (Section 2.12.2)
interface RdxCodeInlineNode {
  type: "code_inline";
  value: string; // The code content
  lang?: string; // Language hint from trailing `{identifier}`, if present
  position: RdxPosition;
}

// Link Node
interface RdxLinkNode {
  type: "link";
  url: string; // The link destination
  title?: string; // Optional link title (from `[text](url "title")`)
  children: RdxNode[];
  position: RdxPosition;
}

// Image Node
interface RdxImageNode {
  type: "image";
  url: string; // The image source URL
  title?: string; // Optional image title
  alt?: string; // Alt text (may also appear as children)
  children: RdxNode[];
  position: RdxPosition;
}

// Footnote Node (definition or reference)
interface RdxFootnoteNode {
  type: "footnote_definition" | "footnote_reference";
  label: string; // The footnote label (e.g., "1", "note")
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
  | any[] // Parsed JSON Arrays
  | RdxVariableNode; // Passed Variables

// Literal Text Node
// Note: "code_inline" without a language hint uses RdxTextNode for backward compatibility.
// "code_inline" with a language hint uses the dedicated RdxCodeInlineNode (Section 2.12.2).
// "math_inline" and "math_display" have been promoted to dedicated node types (see below).
interface RdxTextNode {
  type: "text" | "code_inline";
  value: string;
  position: RdxPosition;
}

// Inline Math Expression (Section 2.6.1, 2.11)
interface RdxMathInlineNode {
  type: "math_inline";
  raw: string; // Original LaTeX source, verbatim
  tree: RdxMathExpr; // Structured parse tree (Section 2.11.2)
  position: RdxPosition;
}

// Display Math Expression (Section 2.6.2, 2.10, 2.11)
interface RdxMathDisplayNode {
  type: "math_display";
  raw: string; // Original LaTeX source, verbatim
  tree: RdxMathExpr; // Structured parse tree (Section 2.11.2)
  label?: string; // Cross-reference label from {#identifier} (Section 2.10)
  position: RdxPosition;
}

// Structured Math Expression Tree (Section 2.11)
// This is a recursive algebraic type representing the parsed structure of a LaTeX
// math expression. The full variant list is defined in Section 2.11.2. The following
// is the TypeScript encoding of the top-level discriminated union.
type RdxMathExpr =
  | { type: "ident"; value: string }
  | { type: "number"; value: string }
  | {
      type: "operator";
      symbol: string;
      kind:
        | "binary"
        | "relation"
        | "prefix"
        | "postfix"
        | "large"
        | "punctuation";
    }
  | { type: "text"; value: string }
  | { type: "row"; children: RdxMathExpr[] }
  | { type: "fenced"; open: string; close: string; body: RdxMathExpr[] }
  | { type: "superscript"; base: RdxMathExpr; script: RdxMathExpr }
  | { type: "subscript"; base: RdxMathExpr; script: RdxMathExpr }
  | {
      type: "subsuperscript";
      base: RdxMathExpr;
      sub: RdxMathExpr;
      sup: RdxMathExpr;
    }
  | {
      type: "frac";
      numerator: RdxMathExpr;
      denominator: RdxMathExpr;
      style: "display" | "text" | "auto";
    }
  | { type: "sqrt"; index?: RdxMathExpr; body: RdxMathExpr }
  | { type: "overline"; body: RdxMathExpr }
  | { type: "underline"; body: RdxMathExpr }
  | { type: "overbrace"; body: RdxMathExpr; annotation?: RdxMathExpr }
  | { type: "underbrace"; body: RdxMathExpr; annotation?: RdxMathExpr }
  | { type: "overset"; over: RdxMathExpr; base: RdxMathExpr }
  | { type: "underset"; under: RdxMathExpr; base: RdxMathExpr }
  | { type: "accent"; kind: string; body: RdxMathExpr } // kind: "hat", "tilde", "vec", "dot", etc.
  | {
      type: "big_operator";
      op: RdxMathExpr;
      limits: "display" | "limits" | "nolimits";
      lower?: RdxMathExpr;
      upper?: RdxMathExpr;
    }
  | { type: "matrix"; rows: RdxMathExpr[][]; delimiters: string } // "pmatrix", "bmatrix", etc.
  | {
      type: "cases";
      rows: Array<{ expr: RdxMathExpr; condition?: RdxMathExpr }>;
    }
  | { type: "array"; columns: ("l" | "c" | "r")[]; rows: RdxMathExpr[][] }
  | {
      type: "align";
      rows: Array<{ cells: RdxMathExpr[]; label?: string }>;
      numbered: boolean;
    }
  | { type: "gather"; rows: RdxMathExpr[]; numbered: boolean }
  | {
      type: "space";
      kind: "thin" | "medium" | "thick" | "quad" | "qquad" | "negthin" | string;
    }
  | { type: "phantom"; body: RdxMathExpr }
  | {
      type: "style_override";
      style: "display" | "text" | "script" | "scriptscript";
      body: RdxMathExpr;
    }
  | { type: "font_override"; font: string; body: RdxMathExpr } // font: "mathbb", "mathcal", etc.
  | { type: "color"; color: string; body: RdxMathExpr }
  | { type: "chem"; value: string } // Raw mhchem content
  | { type: "error"; raw: string; message: string };

// Citation Reference Node (Section 2.7)
interface RdxCitationNode {
  type: "citation";
  keys: RdxCitationKey[];
  position: RdxPosition;
}

interface RdxCitationKey {
  id: string; // The citation key, e.g., "smith2024"
  prefix?: string; // Text before @, e.g., "see "
  locator?: string; // Text after comma, e.g., "p. 42"
}

// Cross-Reference Node (Section 2.8)
interface RdxCrossRefNode {
  type: "cross_ref";
  target: string; // The reference target, e.g., "fig:architecture"
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

- `"code_block"`, `"link"`, `"image"`, and `"footnote_definition"` / `"footnote_reference"` use dedicated interfaces instead of `RdxStandardBlockNode` because they carry domain-specific fields (`lang`, `url`, `title`, `label`, etc.) that do not apply to generic block nodes.
- `"code_inline"` uses `RdxTextNode` when no language hint is present (backward compatibility) and `RdxCodeInlineNode` when a trailing `{lang}` hint is detected (Section 2.12.2).
- `"math_inline"` and `"math_display"` use dedicated `RdxMathInlineNode` and `RdxMathDisplayNode` interfaces containing both a `raw` LaTeX string and a structured `RdxMathExpr` tree (Section 2.11). The `$` delimiter is disambiguated from `{$var}` variable syntax by the rule that `{$` always begins a variable expression.
- `"citation"` uses `RdxCitationNode` containing one or more citation keys with optional prefixes and locators (Section 2.7).
- `"cross_ref"` uses `RdxCrossRefNode` containing an unresolved target identifier (Section 2.8). Resolution is the Host Environment's responsibility.
- `"definition_list"`, `"definition_term"`, and `"definition_description"` use `RdxStandardBlockNode` (Section 2.9).

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
7. Outputs an AST structure conforming exactly to the schema defined in Section 4, including required positional mapping on every node. Dedicated node types (`RdxCodeBlockNode`, `RdxCodeInlineNode`, `RdxMathInlineNode`, `RdxMathDisplayNode`, `RdxCitationNode`, `RdxCrossRefNode`, `RdxLinkNode`, `RdxImageNode`, `RdxFootnoteNode`) **MUST** be used where specified.
8. Passes through standard HTML elements (lowercase tags) per CommonMark rules without interpreting them as RDX Components.
9. Complies with the zero-execution security mandates outlined in Section 5.
10. Parses citation references (`[@key]`) per Section 2.7 and emits `RdxCitationNode`.
11. Parses cross-references (`{@target}`) per Section 2.8 and emits `RdxCrossRefNode`.
12. Parses definition lists (`: ` marker) per Section 2.9 and emits the corresponding `RdxStandardBlockNode` types.
13. Parses display math labels (`$$ {#id}`) per Section 2.10 and emits `RdxMathDisplayNode` with the `label` field.
14. Parses math expression content into structured `RdxMathExpr` trees per Section 2.11. Tier 1 constructs are **REQUIRED**. Tier 2 constructs are **RECOMMENDED**. Tier 3 constructs are **OPTIONAL**. Unsupported constructs at any tier **MUST** emit an `Error` variant rather than being silently dropped.
15. Parses structured code block info strings per Section 2.12.1, extracting `title`, `highlight`, `showLineNumbers`, `diff`, and `caption` fields.
16. Recognizes inline code language hints (`{identifier}` after closing backtick) per Section 2.12.2 and emits `RdxCodeInlineNode` when present.

---

## **7. Standard Component Library (Informative)**

This section is **informative**, not normative. It defines a set of component names and schemas that conforming Host Environments **SHOULD** support to ensure portability of RDX documents across implementations. A Parser is not required to be aware of these components — they are validated by the schema system (see `rdx-schema`) and rendered by the Host Environment.

### **7.1. Admonitions**

The following admonition components provide semantic callout blocks. Each **SHOULD** accept a `title` attribute (optional string, overrides default heading) and a `collapsible` attribute (optional boolean, defaults to `false`).

| Component     | Default Title | Semantic Meaning                                                  |
| ------------- | ------------- | ----------------------------------------------------------------- |
| `<Note>`      | "Note"        | Supplementary information the reader should be aware of.          |
| `<Tip>`       | "Tip"         | Helpful suggestion or best practice.                              |
| `<Important>` | "Important"   | Key information the reader must not overlook.                     |
| `<Warning>`   | "Warning"     | Potential issue that could cause problems.                        |
| `<Caution>`   | "Caution"     | Action that could lead to data loss or irreversible consequences. |

```rdx
<Warning title="Breaking Change">
  The `v1` API will be removed in the next release.
</Warning>
```

### **7.2. Figures, Tables & Listings**

These components wrap content with a caption and cross-referenceable label. The Host Environment **SHOULD** auto-number them sequentially within the document.

| Component       | Wraps                           | Caption Position (Print) |
| --------------- | ------------------------------- | ------------------------ |
| `<Figure>`      | Images, diagrams, illustrations | Below content            |
| `<TableFigure>` | Markdown tables                 | Above content            |
| `<Listing>`     | Code blocks                     | Above content            |

Each **MUST** accept:

- `id` (optional string) — cross-reference label (e.g., `"fig:arch"`)
- `caption` (required string) — descriptive caption text

```rdx
<Figure id="fig:arch" caption="System architecture overview">
  ![Architecture diagram](arch.png)
</Figure>
```

### **7.3. Interactive Components**

These components provide interactive behavior on the web and degrade to static content in print.

| Component                         | Web Behavior            | Print Behavior                  |
| --------------------------------- | ----------------------- | ------------------------------- |
| `<Tabs>` / `<Tab>`                | Tabbed interface        | All tabs rendered sequentially  |
| `<Accordion>` / `<AccordionItem>` | Collapsible sections    | All items expanded              |
| `<Steps>`                         | Numbered step indicator | Numbered list with step headers |
| `<CodeGroup>`                     | Tabbed code variants    | All variants stacked            |

**`<Tabs>`** **MUST** contain only `<Tab>` children. Each `<Tab>` **MUST** have a `label` attribute (string).

**`<Steps>`** treats each child heading (any depth) as a step boundary. Content between headings is the step body.

**`<CodeGroup>`** **MUST** contain only `RdxCodeBlockNode` children. The `title` field from each code block's info string serves as the tab label.

### **7.4. Conditional Rendering**

| Component     | Behavior                                                |
| ------------- | ------------------------------------------------------- |
| `<WebOnly>`   | Content rendered only in web output. Stripped in print. |
| `<PrintOnly>` | Content rendered only in print output. Stripped on web. |

These are syntactic sugar for the `target` global attribute. Any component **MAY** accept a `target` attribute with values `"web"`, `"print"`, or `"all"` (default). Host Environments **MUST** strip nodes whose `target` does not match the current output format.

### **7.5. Page & Layout Control (Print)**

These components are meaningful only in print/PDF output. Web renderers **SHOULD** ignore them or render a minimal visual indicator (e.g., a horizontal rule for `<PageBreak />`).

| Component         | Print Behavior                                   |
| ----------------- | ------------------------------------------------ |
| `<PageBreak />`   | Force a new page.                                |
| `<ColumnBreak />` | Break to the next column in multi-column layout. |
| `<Spread>`        | Content spans both pages of an open book spread. |

### **7.6. Book Front & Back Matter (Print)**

| Component      | Print Behavior                                   |
| -------------- | ------------------------------------------------ |
| `<Abstract>`   | Indented abstract block with "Abstract" heading. |
| `<Dedication>` | Centered text on its own page.                   |
| `<Epigraph>`   | Right-aligned quote with attribution.            |
| `<Colophon>`   | Production notes, typically on the last page.    |

`<Epigraph>` **SHOULD** accept an `attribution` attribute (string) for the quote source.

### **7.7. Academic Environments**

These components support theorem-like environments for academic and mathematical writing. Each **SHOULD** accept `id` (optional string, for cross-referencing) and `title` (optional string, for named theorems).

| Component       | Default Label | Body Style (Print)                |
| --------------- | ------------- | --------------------------------- |
| `<Theorem>`     | "Theorem"     | Italic body                       |
| `<Lemma>`       | "Lemma"       | Italic body                       |
| `<Corollary>`   | "Corollary"   | Italic body                       |
| `<Proposition>` | "Proposition" | Italic body                       |
| `<Conjecture>`  | "Conjecture"  | Italic body                       |
| `<Definition>`  | "Definition"  | Upright body                      |
| `<Example>`     | "Example"     | Upright body                      |
| `<Remark>`      | "Remark"      | Upright body                      |
| `<Proof>`       | "Proof"       | Upright body, QED symbol appended |

```rdx
<Theorem id="thm:main" title="Main Result">
  For all $n > 0$, the algorithm terminates in $O(n \log n)$.
</Theorem>

<Proof>
  By induction on $n$. The base case $n = 1$ is trivial. ...
</Proof>
```

The Host Environment **SHOULD** auto-number theorem-like environments sequentially (e.g., "Theorem 1", "Lemma 2"). `<Proof>` environments are conventionally unnumbered.

### **7.8. Bibliography**

`<Bibliography />` is a self-closing component that marks the insertion point for a formatted reference list. It **MAY** accept a `style` attribute (string) specifying the citation style (e.g., `"apa"`, `"ieee"`, `"chicago"`).

If `<Bibliography />` is not present in the document, the Host Environment **SHOULD** append the bibliography at the document end when citations are present.

### **7.9. Content Reuse**

| Component                              | Behavior                                                |
| -------------------------------------- | ------------------------------------------------------- |
| `<Include src="path" />`               | Splice the AST of another `.rdx` file at this location. |
| `<Partial src="path" fragment="id" />` | Splice a labeled subtree from another file.             |

Resolution of `src` paths and fragment extraction is the Host Environment's responsibility. The Parser emits these as standard `RdxComponentNode` elements.

### **7.10. Diagrams**

`<Diagram>` renders embedded diagram source code as a visual diagram.

- `type` (required string): `"mermaid"`, `"d2"`, `"plantuml"`, `"graphviz"`
- `id` (optional string): cross-reference label
- `caption` (optional string): figure caption

The diagram source is the component's raw body content. The Host Environment **MUST** render the diagram using the appropriate engine on the web (e.g., mermaid.js) and **SHOULD** pre-render to SVG or PDF for print output.

```rdx
<Diagram type="mermaid" id="fig:flow" caption="Request processing flow">
  graph LR
    A[Client] --> B[Load Balancer]
    B --> C[Server]
</Diagram>
```

### **7.11. API Documentation**

| Component       | Purpose                                                                                |
| --------------- | -------------------------------------------------------------------------------------- |
| `<ApiEndpoint>` | Documents an API endpoint. Accepts `method` (string) and `path` (string).              |
| `<ApiParam>`    | Documents a parameter. Accepts `name` (string), `type` (string), `required` (boolean). |

```rdx
<ApiEndpoint method="GET" path="/api/users/:id">
  Retrieve a user by their unique identifier.

  <ApiParam name="id" type="string" required>
    The user's unique identifier.
  </ApiParam>
</ApiEndpoint>
```

---

## **8. Frontmatter Conventions (Informative)**

This section is **informative**. It defines standard frontmatter field names that Host Environments **SHOULD** recognize to enable portability.

### **8.1. Core Metadata**

| Field         | Type                | Description                                          |
| ------------- | ------------------- | ---------------------------------------------------- |
| `title`       | `string`            | Document title.                                      |
| `description` | `string`            | Short summary for SEO and previews.                  |
| `date`        | `string` (ISO 8601) | Publication or last-modified date.                   |
| `authors`     | `Author[]`          | List of authors (see below).                         |
| `lang`        | `string` (BCP 47)   | Document language (e.g., `"en-US"`, `"ja"`, `"ar"`). |
| `dir`         | `"ltr" \| "rtl"`    | Text direction. Defaults to `"ltr"`.                 |

**Author object:**

```yaml
authors:
  - name: "Farhan Ahmed"
    email: "farhan@example.com"
    affiliation: "Independent"
    orcid: "0000-0000-0000-0000"
    url: "https://example.com"
```

### **8.2. Academic Metadata**

| Field          | Type                     | Description                                                                    |
| -------------- | ------------------------ | ------------------------------------------------------------------------------ |
| `subtitle`     | `string`                 | Document subtitle.                                                             |
| `abstract`     | `string`                 | Plain-text abstract (for metadata; rich abstract uses `<Abstract>` component). |
| `keywords`     | `string[]`               | Subject keywords for indexing.                                                 |
| `bibliography` | `string`                 | Path to `.bib` or `.yaml` bibliography file.                                   |
| `csl`          | `string`                 | Path to CSL citation style file (e.g., `"ieee.csl"`, `"apa.csl"`).             |
| `math-macros`  | `Record<string, string>` | LaTeX macro definitions for math expressions (Section 2.11.3).                 |

### **8.3. Web Metadata**

| Field         | Type           | Description                                              |
| ------------- | -------------- | -------------------------------------------------------- |
| `canonical`   | `string` (URL) | Canonical URL for SEO.                                   |
| `og:image`    | `string` (URL) | Open Graph image for social sharing.                     |
| `robots`      | `string`       | Search engine directives (e.g., `"index, follow"`).      |
| `schema-type` | `string`       | Schema.org type hint (e.g., `"TechArticle"`, `"HowTo"`). |

### **8.4. Print Metadata**

| Field                | Type                                          | Description                                                |
| -------------------- | --------------------------------------------- | ---------------------------------------------------------- |
| `page-size`          | `string`                                      | Page dimensions (e.g., `"a4"`, `"letter"`, `"6in x 9in"`). |
| `binding`            | `"left" \| "right"`                           | Binding edge for recto/verso margin calculation.           |
| `page-start`         | `"recto" \| "verso"`                          | Which page the body content begins on.                     |
| `footnote-style`     | `"numeric" \| "roman" \| "alpha" \| "symbol"` | Footnote marker style.                                     |
| `footnote-placement` | `"page" \| "endnotes"`                        | Where footnotes are rendered.                              |

### **8.5. Collection Metadata**

| Field        | Type            | Description                                             |
| ------------ | --------------- | ------------------------------------------------------- |
| `collection` | `string`        | Name of the document collection (e.g., `"user-guide"`). |
| `order`      | `number`        | Sort order within the collection.                       |
| `next`       | `string` (path) | Path to the next document in the collection.            |
| `prev`       | `string` (path) | Path to the previous document in the collection.        |
| `version`    | `string`        | Document or API version (e.g., `"2.0"`).                |

### **8.6. Abbreviations**

| Field           | Type                     | Description                               |
| --------------- | ------------------------ | ----------------------------------------- |
| `abbreviations` | `Record<string, string>` | Map of abbreviations to their expansions. |

```yaml
abbreviations:
  HTML: HyperText Markup Language
  CSS: Cascading Style Sheets
  API: Application Programming Interface
```

The Host Environment **SHOULD** wrap first occurrences of defined abbreviations in `<abbr>` tags on web output and expand them on first use in print output.
