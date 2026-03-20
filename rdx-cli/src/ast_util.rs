use rdx_ast::Node;

/// Recursively walk all nodes in the AST, calling the visitor for each.
///
/// The visitor is called on each node before descending into its children
/// (pre-order traversal).
pub fn walk_nodes<F: FnMut(&Node)>(nodes: &[Node], visitor: &mut F) {
    for node in nodes {
        visitor(node);
        if let Some(children) = node.children() {
            walk_nodes(children, visitor);
        }
    }
}
