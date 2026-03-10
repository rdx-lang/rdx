# rdx-github

Optional [RDX](https://github.com/rdx-lang/rdx) transform that converts GitHub-style references in text to links.

## Usage

```rust
use rdx_transform::Pipeline;
use rdx_github::GithubReferences;

let root = Pipeline::new()
    .add(GithubReferences::new("rdx-lang/rdx"))
    .run("Fixed in #42 by @alice (commit abc1234f).\n");
```

## What it converts

| Source | Output |
|---|---|
| `#123` | Link to `https://github.com/{repo}/issues/123` |
| `@username` | Link to `https://github.com/username` |
| `abc1234f` (7+ hex chars with a letter) | Link to `https://github.com/{repo}/commit/abc1234f` |

## Configuration

The repository can be set explicitly or read from frontmatter:

```rust
// Explicit
GithubReferences::new("rdx-lang/rdx")

// From frontmatter (reads `github: owner/repo` field)
GithubReferences::from_frontmatter()
```

A custom `base_url` can be set for GitHub Enterprise:

```rust
GithubReferences::new("org/repo").base_url("https://github.example.com")
```

## Behavior

- Skips references inside existing links and images (no nested links)
- Issue references require a non-alphanumeric preceding character to avoid false positives
- User mentions must follow whitespace
- Commit SHAs must be 7-40 hex characters and contain at least one letter

## License

Licensed under either of [Apache License, Version 2.0](../LICENSE-APACHE) or [MIT License](../LICENSE-MIT) at your option.
