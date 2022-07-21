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

