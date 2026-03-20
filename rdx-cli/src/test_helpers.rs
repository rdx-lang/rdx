/// Shared test utilities for rdx-cli unit tests.
///
/// This module is only compiled when running tests.
use rdx_ast::{Point, Position, Root, RootType};
use rdx_ast::Node;

/// Construct a [`Point`] with explicit line, column, and byte offset.
pub fn pos(line: usize, col: usize, off: usize) -> Point {
    Point { line, column: col, offset: off }
}

/// Construct a [`Position`] from two (line, col, offset) triples.
pub fn span(sl: usize, sc: usize, so: usize, el: usize, ec: usize, eo: usize) -> Position {
    Position {
        start: pos(sl, sc, so),
        end: pos(el, ec, eo),
    }
}

/// Construct a minimal [`Root`] node containing `children`.
pub fn make_root(children: Vec<Node>) -> Root {
    Root {
        node_type: RootType::Root,
        frontmatter: None,
        children,
        position: span(1, 1, 0, 100, 1, 0),
    }
}
