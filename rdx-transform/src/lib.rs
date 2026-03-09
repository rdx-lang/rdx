pub use rdx_ast::*;
pub use rdx_parser::parse;

mod transforms;
pub use transforms::slug::AutoSlug;
pub use transforms::toc::TableOfContents;

/// A transform that operates on an RDX AST in place.
///
/// Implement this trait to create custom RDX plugins. Transforms receive
/// a mutable reference to the full document root and the original source text.
///
/// # Example
///
/// ```rust
/// use rdx_transform::{Transform, Root};
///
/// struct MyPlugin;
///
/// impl Transform for MyPlugin {
///     fn name(&self) -> &str { "my-plugin" }
///     fn transform(&self, root: &mut Root, _source: &str) {
///         // modify the AST
///     }
/// }
/// ```
pub trait Transform {
    /// A short identifier for this transform (used in error messages / debugging).
    fn name(&self) -> &str;

    /// Apply the transform to the AST. `source` is the original document text,
    /// available for transforms that need to reference raw content.
    fn transform(&self, root: &mut Root, source: &str);
}

/// A composable pipeline that parses an RDX document and runs a chain of transforms.
///
/// # Example
///
/// ```rust
/// use rdx_transform::{Pipeline, AutoSlug, TableOfContents};
///
/// let root = Pipeline::new()
///     .add(AutoSlug::new())
///     .add(TableOfContents::default())
///     .run("# Hello\n\n## World\n");
/// ```
pub struct Pipeline {
    transforms: Vec<Box<dyn Transform>>,
}

impl Pipeline {
    pub fn new() -> Self {
        Pipeline {
            transforms: Vec::new(),
        }
    }

    /// Append a transform to the pipeline. Transforms run in insertion order.
    #[allow(clippy::should_implement_trait)]
    pub fn add(mut self, transform: impl Transform + 'static) -> Self {
        self.transforms.push(Box::new(transform));
        self
    }

    /// Parse the input and run all transforms in order.
    pub fn run(&self, input: &str) -> Root {
        let mut root = parse(input);
        for t in &self.transforms {
            t.transform(&mut root, input);
        }
        root
    }

    /// Run transforms on an already-parsed AST.
    pub fn apply(&self, root: &mut Root, source: &str) {
        for t in &self.transforms {
            t.transform(root, source);
        }
    }
}

impl Default for Pipeline {
    fn default() -> Self {
        Self::new()
    }
}

/// Convenience: parse + apply built-in transforms (slug + toc).
pub fn parse_with_defaults(input: &str) -> Root {
    Pipeline::new()
        .add(AutoSlug::new())
        .add(TableOfContents::default())
        .run(input)
}

/// Walk all nodes in the AST, calling `f` on each with a mutable reference.
/// Useful for implementing transforms.
#[allow(clippy::ptr_arg)]
pub fn walk_mut(nodes: &mut Vec<Node>, f: &mut dyn FnMut(&mut Node)) {
    for node in nodes.iter_mut() {
        f(node);
        match node {
            Node::Paragraph(b)
            | Node::Heading(b)
            | Node::List(b)
            | Node::ListItem(b)
            | Node::Blockquote(b)
            | Node::Html(b)
            | Node::Table(b)
            | Node::TableRow(b)
            | Node::TableCell(b)
            | Node::Emphasis(b)
            | Node::Strong(b)
            | Node::Strikethrough(b)
            | Node::ThematicBreak(b) => {
                walk_mut(&mut b.children, f);
            }
            Node::Link(l) => walk_mut(&mut l.children, f),
            Node::Image(i) => walk_mut(&mut i.children, f),
            Node::Component(c) => walk_mut(&mut c.children, f),
            Node::FootnoteDefinition(n) => walk_mut(&mut n.children, f),
            _ => {}
        }
    }
}

/// Walk all nodes immutably.
pub fn walk<'a>(nodes: &'a [Node], f: &mut dyn FnMut(&'a Node)) {
    for node in nodes {
        f(node);
        match node {
            Node::Paragraph(b)
            | Node::Heading(b)
            | Node::List(b)
            | Node::ListItem(b)
            | Node::Blockquote(b)
            | Node::Html(b)
            | Node::Table(b)
            | Node::TableRow(b)
            | Node::TableCell(b)
            | Node::Emphasis(b)
            | Node::Strong(b)
            | Node::Strikethrough(b)
            | Node::ThematicBreak(b) => {
                walk(&b.children, f);
            }
            Node::Link(l) => walk(&l.children, f),
            Node::Image(i) => walk(&i.children, f),
            Node::Component(c) => walk(&c.children, f),
            Node::FootnoteDefinition(n) => walk(&n.children, f),
            _ => {}
        }
    }
}

/// Extract plain text from a list of nodes (for generating slugs, alt text, etc).
pub fn collect_text(nodes: &[Node]) -> String {
    let mut out = String::new();
    walk(nodes, &mut |node| {
        if let Node::Text(t) = node {
            out.push_str(&t.value);
        }
    });
    out
}
