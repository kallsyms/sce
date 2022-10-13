/// DepthFirstWalk is a small helper to do simple iterations over a tree-sitter node/tree,
/// implementing Iterator for simple for-in uses, as well as a callback-based traversal function,
/// useful if you want to/need to not traverse deeper when a specific condition is met.
pub struct DepthFirstWalk<'a> {
    root: tree_sitter::Node<'a>,
    cursor: tree_sitter::TreeCursor<'a>,
    done: bool,
}

pub fn depth_first<'a>(node: tree_sitter::Node<'a>) -> DepthFirstWalk<'a> {
    DepthFirstWalk{
        root: node,
        cursor: node.walk(),
        done: false,
    }
}

impl<'a> Iterator for DepthFirstWalk<'a> {
    type Item = tree_sitter::Node<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }

        let node = self.cursor.node();

        if self.cursor.goto_first_child() {
            return Some(node);
        }
        if self.cursor.goto_next_sibling() {
            return Some(node);
        }

        loop {
            self.cursor.goto_parent();

            if self.cursor.node() == self.root {
                self.done = true;
                return Some(node);
            }

            if self.cursor.goto_next_sibling() {
                return Some(node);
            }
        }
    }
}

impl<'a> DepthFirstWalk<'a> {
    /// Call the given cb for each node, skipping any descendants of a given node if the cb returns
    /// false. Additionally, call on_descent when descending down into a new "layer" and on_ascent
    /// when coming back up.
    pub fn traverse_with_depth<F, D, A>(&mut self, mut cb: F, mut on_descent: D, mut on_ascent: A)
        where F: FnMut(tree_sitter::Node<'a>) -> bool,
              D: FnMut(tree_sitter::Node<'a>, tree_sitter::Node<'a>),
              A: FnMut(tree_sitter::Node<'a>, tree_sitter::Node<'a>)
              {
        'outer: loop {
            let mut node = self.cursor.node();
            if cb(node) {
                if self.cursor.goto_first_child() {
                    on_descent(node, self.cursor.node());
                    continue;
                }
            }

            if self.cursor.goto_next_sibling() {
                continue;
            }

            loop {
                self.cursor.goto_parent();
                on_ascent(node, self.cursor.node());

                node = self.cursor.node();

                if node == self.root {
                    return;
                }

                if self.cursor.goto_next_sibling() {
                    continue 'outer;
                }
            }
        }
    }

    /// Call the given cb for each node, skipping any descendants of a given node if the cb returns
    /// false.
    pub fn traverse<F>(&mut self, cb: F) where F: FnMut(tree_sitter::Node<'a>) -> bool {
        self.traverse_with_depth(cb, |_, _|{}, |_, _|{})
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    fn sample_tree() -> tree_sitter::Tree {
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(unsafe {crate::engine_config::tree_sitter_python()}).unwrap();
        let tree = parser.parse("def foo(a, b, c): return a + b + c", None).unwrap();

        tree
    }

    #[test]
    /// Test the iterator form of depth_first
    fn test_depth_first() {
        let tree = sample_tree();

        let mut node_kinds = vec![];
        for node in depth_first(tree.root_node()) {
            node_kinds.push(node.kind());
        }

        assert_eq!(node_kinds, vec![
                   "module",
                   "function_definition",
                   "def",
                   "identifier",
                   "parameters",
                   "(",
                   "identifier",
                   ",",
                   "identifier",
                   ",",
                   "identifier",
                   ")",
                   ":",
                   "block",
                   "return_statement",
                   "return",
                   "binary_operator",
                   "binary_operator",
                   "identifier",
                   "+",
                   "identifier",
                   "+",
                   "identifier",
        ]);
    }

    #[test]
    /// Test the traverse(cb) form of depth_first, always requesting child nodes
    fn test_traverse_all() {
        let tree = sample_tree();

        let mut node_kinds = vec![];
        depth_first(tree.root_node()).traverse(|node| {
            node_kinds.push(node.kind());

            true
        });

        assert_eq!(node_kinds, vec![
                   "module",
                   "function_definition",
                   "def",
                   "identifier",
                   "parameters",
                   "(",
                   "identifier",
                   ",",
                   "identifier",
                   ",",
                   "identifier",
                   ")",
                   ":",
                   "block",
                   "return_statement",
                   "return",
                   "binary_operator",
                   "binary_operator",
                   "identifier",
                   "+",
                   "identifier",
                   "+",
                   "identifier",
        ]);
    }

    #[test]
    /// Test the traverse(cb) form of depth_first, requesting child nodes except in the case of the
    /// binary_operator
    fn test_traverse_exit() {
        let tree = sample_tree();

        let mut node_kinds = vec![];
        depth_first(tree.root_node()).traverse(|node| {
            node_kinds.push(node.kind());

            return node.kind() != "binary_operator";
        });

        assert_eq!(node_kinds, vec![
                   "module",
                   "function_definition",
                   "def",
                   "identifier",
                   "parameters",
                   "(",
                   "identifier",
                   ",",
                   "identifier",
                   ",",
                   "identifier",
                   ")",
                   ":",
                   "block",
                   "return_statement",
                   "return",
                   "binary_operator",
        ]);
    }

    #[test]
    /// Test traverse_with_depth(cb, on_descent, on_ascent), ensuring descend and ascend get called
    /// as appropriate.
    fn test_traverse_with_depth() {
        let tree = sample_tree();

        let transitions = std::cell::RefCell::new(vec![]);

        depth_first(tree.root_node()).traverse_with_depth(
            |_| { true },
            |from, to| {
                transitions.borrow_mut().push(("DESCEND", from.kind(), to.kind()));
            },
            |from, to| {
                transitions.borrow_mut().push(("ASCEND", from.kind(), to.kind()));
            },
        );

        assert_eq!(transitions.into_inner(), vec![
                   ("DESCEND", "module", "function_definition"),
                   ("DESCEND", "function_definition", "def"),
                   ("DESCEND", "parameters", "("),
                   ("ASCEND", ")", "parameters"),
                   // sibling to block
                   ("DESCEND", "block", "return_statement"),
                   ("DESCEND", "return_statement", "return"),
                   // sibling to binary_operator
                   ("DESCEND", "binary_operator", "binary_operator"),
                   ("DESCEND", "binary_operator", "identifier"),
                   ("ASCEND", "identifier", "binary_operator"),
                   ("ASCEND", "identifier", "binary_operator"),
                   ("ASCEND", "binary_operator", "return_statement"),
                   ("ASCEND", "return_statement", "block"),
                   ("ASCEND", "block", "function_definition"),
                   ("ASCEND", "function_definition", "module"),
        ]);
    }
}
