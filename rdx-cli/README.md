# rdx-cli

Command-line interface for [RDX](https://github.com/rdx-lang/rdx) documents — parse, validate, convert, and format `.rdx` files.

## Install

```sh
cargo install rdx-cli
```

## Commands

### `rdx parse`

Parse an `.rdx` file and output the AST as JSON.

```sh
rdx parse document.rdx
rdx parse document.rdx --pretty
```

### `rdx validate`

Validate a document against a component schema.

```sh
rdx validate document.rdx --schema schema.json
```

Prints diagnostics to stderr and exits with code 1 if errors are found.

### `rdx convert`

Convert `.mdx` files to `.rdx`.

```sh
rdx convert page.mdx                    # output to stdout
rdx convert page.mdx --output page.rdx  # output to file
rdx convert page.mdx --in-place         # writes page.rdx
```

Handles common MDX patterns:

- Removes `import`/`export` statements
- Converts JSX comments `{/* ... */}` to HTML comments `<!-- ... -->`
- Strips JS expression attributes (with warnings)
- Converts `className` to `class`
- Preserves valid RDX attributes (`{true}`, `{42}`, `{$var}`, `{{json}}`)

### `rdx fmt`

Format an `.rdx` file.

```sh
rdx fmt document.rdx              # output to stdout
rdx fmt document.rdx --write      # overwrite the file
rdx fmt document.rdx --check      # exit 1 if not formatted
```

## License

MIT OR Apache-2.0
