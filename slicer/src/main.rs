use std::cell::RefCell;
use std::collections::HashSet;
use std::hash::{Hash, Hasher};
use std::path::Path;
use tree_sitter;
use tree_sitter_c;

mod guess_language;

/// SlicerConfig is the main configuration for the slicer.
/// This includes all language-specific tree-sitter type names which various stages of the slicing
/// need.
struct SlicerConfig {
    /// The tree_sitter language the slicer should use to parse with
    language: tree_sitter::Language,

    /// Type names representing "atomic" name fragments (e.g. `self`, `foo`, `bar`)
    identifier_types: Vec<&'static str>,

    /// Type names representing any possible "complete" name (e.g. `self.foo.bar`)
    name_types: Vec<&'static str>,

    /// Type names and the type names for the descendant target and source representing ways a
    /// variable can flow into a new variable (e.g. assignment).
    /// e.g. ("assignment_expression", ("left", "right"))
    propagating_types: Vec<(&'static str, (&'static str, &'static str))>,

    /// Type names representing statements. Can use "inheritance" information from node-types.
    statement_types: Vec<&'static str>,

    /// Type names representing scopes in which we can slice (just functions?)
    slice_scope_types: Vec<&'static str>,

    /// Type names representing variable accessibility "boundaries" in the language, where
    /// variables defined within are not accessible outside of.
    /// For Python, this would be function level, but for C-like languages, this would be
    /// block-level.
    var_definition_scope_types: Vec<&'static str>,
}

fn from_guessed_language(language: guess_language::Language) -> Option<SlicerConfig> {
    use guess_language::Language::*;

    match language {
        C => {
            Some(SlicerConfig{
                language: tree_sitter_c::language(),
                identifier_types: vec!["identifier", "field_identifier"],
                name_types: vec!["identifier", "field_expression"],
                propagating_types: vec![
                    ("assignment_expression", ("left", "right")),
                ],
                // TODO: from node-types.json
                statement_types: vec!["expression_statement", "assignment_expression", "init_declarator"],
                slice_scope_types: vec!["function_definition"],
                var_definition_scope_types: vec!["compound_statement"],
            })
        }
        _ => None
    }
}

/// Represents a symbol name, represented as the list of components which make up the symbol
/// e.g. ["self", "foo", "bar"] in the case of `self.foo.bar` in Python.
/// This lets us easily check if a variable affects/is affected by another (in name).
#[derive(Clone, Debug)]
struct NameRef<'a> {
    node: tree_sitter::Node<'a>,
    components: Vec<String>,
}

impl<'a> PartialEq for NameRef<'a> {
    fn eq(&self, other: &Self) -> bool {
        self.components == other.components
    }
}

impl<'a> Eq for NameRef<'a> {}

impl<'a> Hash for NameRef<'a> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // we don't care about the node itself, just the name
        self.components.hash(state);
    }
}

impl<'a> NameRef<'a> {
    fn affects(&self, other: &NameRef) -> bool {
        let len = self.components.len().min(other.components.len());
        return self.components[..len].iter().zip(other.components[..len].iter()).all(|(a, b)| a == b);
    }
}

/// DepthFirstWalk is a small helper to do simple iterations over a tree-sitter node/tree,
/// implementing Iterator for simple for-in uses, as well as a callback-based traversal function,
/// useful if you want to/need to not traverse deeper when a specific condition is met.
struct DepthFirstWalk<'a> {
    root: tree_sitter::Node<'a>,
    cursor: tree_sitter::TreeCursor<'a>,
    done: bool,
}

fn depth_first<'a>(node: tree_sitter::Node<'a>) -> DepthFirstWalk<'a> {
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
    fn traverse_with_depth<F, D, A>(&mut self, mut cb: F, mut on_descent: D, mut on_ascent: A)
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
    fn traverse<F>(&mut self, cb: F) where F: FnMut(tree_sitter::Node<'a>) -> bool {
        self.traverse_with_depth(cb, |_, _|{}, |_, _|{})
    }
}

struct Slicer {
    config: SlicerConfig,
    src: String,
}

impl Slicer {
    /// Return a Vec of all "name components", e.g. ["self", "foo", "bar"]
    fn name_components<'a>(&self, node: tree_sitter::Node) -> Vec<String> {
        depth_first(node)
            .filter(|&descendant| self.config.identifier_types.contains(&descendant.kind()))
            .map(|descendant| String::from(&self.src[descendant.start_byte()..descendant.end_byte()]))
            .into_iter().collect()
    }

    /// Find the name reference at the specified point, if an identifier is referenced at that
    /// point.
    fn name_at_point<'a>(&self, root: &'a tree_sitter::Node, point: tree_sitter::Point) -> Option<NameRef<'a>> {
        let mut cur = root.walk();

        loop {
            let node = cur.node();

            if self.config.name_types.contains(&node.kind()) {
                // Walk down and gather all specific identifiers
                return Some(NameRef{node, components: self.name_components(node)});
            }

            if cur.goto_first_child_for_point(point) == None {
                return None;
            }
        }
    }

    fn referenced_names<'a>(&self, node: tree_sitter::Node<'a>) -> Vec<NameRef<'a>> {
        let mut names = vec![];
        depth_first(node).traverse(|descendant| {
            if self.config.name_types.contains(&descendant.kind()) {
                names.push(NameRef{node: descendant.clone(), components: self.name_components(descendant)});
                return false;
            }
            return true;
        });
        names
    }

    /// Propagate the set of target names out through all assignments
    fn propagate_targets<'a>(&self, outer_scope: tree_sitter::Node<'a>, initial_target_names: &HashSet<NameRef<'a>>) -> HashSet<NameRef<'a>> {
        let mut target_names = initial_target_names.clone();

        // TODO: use depth_first.traverse_with_depth to push and pop scopes based on
        // var_definition_scope_types
        loop {
            let mut changed = false;

            for descendant in depth_first(outer_scope) {
                if let Some((_, (target_child_name, source_child_name))) = self.config.propagating_types.iter().find(|&&(expr_kind, (_, _))| expr_kind == descendant.kind()) {
                    let target_node = descendant.child_by_field_name(target_child_name);
                    let source_node = descendant.child_by_field_name(source_child_name);

                    // Guard against things like python's `with` which may or may not have a target
                    if target_node.is_none() || source_node.is_none() {
                        continue;
                    }

                    let node_target_names = self.referenced_names(target_node.unwrap());
                    let node_source_names = self.referenced_names(source_node.unwrap());

                    // If any known targets "affects" a var in the source, all vars in the dest are now targets
                    if target_names.iter().any(|tname| node_source_names.iter().any(|sname| tname.affects(&sname))) {
                        println!("Propagating node {:?} adds {:?} to targets", descendant, node_target_names);
                        changed = target_names.intersection(&HashSet::from_iter(node_target_names.iter().cloned())).count() > 0;
                        target_names.extend(node_target_names);
                    }
                }
            }
            
            if !changed {
                break;
            }
        }

        target_names
    }

    /// Returns an in-order Vec of the highest-level statement-type nodes which do not reference
    /// any target name.
    fn flatten_unreferenced<'a>(&self, target_func: tree_sitter::Node<'a>, target_names: &HashSet<NameRef<'a>>) -> Vec<tree_sitter::Node<'a>> {
        let mut delete_nodes = vec![];

        // compute set of each node which references any target.
        // this is an equivalent to computing referenced names for each node in the walk,
        // but avoids having to walk each subtree for every single node, instead doing the
        // computation once and calling back in on_ascent.
        let references = RefCell::new(HashSet::new());

        depth_first(target_func).traverse_with_depth(
            |descendant| {
                if self.config.name_types.contains(&descendant.kind()) {
                    let name = NameRef{node: descendant, components: self.name_components(descendant)};
                    if target_names.contains(&name) {
                        references.borrow_mut().insert(descendant);
                    }
                    return false;
                }
                return true;
            },
            |_, _|{},
            |_, to| {
                // "bubble-up" reference data when exiting out of a node
                let mut cur = to.walk();
                for child in to.children(&mut cur) {
                    if references.borrow().get(&child).is_some() {
                        references.borrow_mut().insert(to);
                        break;
                    }
                }
            }
        );

        depth_first(target_func).traverse(|statement| {
            if !self.config.statement_types.contains(&statement.kind()) {
                return true;
            }

            // if this node is not recorded as having a reference to a target, we can delete it
            if !references.borrow().contains(&statement) {
                // check if a parent has already been marked for deletion
                let mut parent = statement;
                let parent_deleted = loop {
                    match parent.parent() {
                        Some(n) => {
                            parent = n;
                        }
                        None => {
                            break false;
                        }
                    }

                    if delete_nodes.contains(&parent) {
                        break true;
                    }
                };

                if !parent_deleted {
                    delete_nodes.push(statement);
                }
            }
            
            // Have to continue descent because statement can mean both compound statements, single
            // expression statements, etc.
            return true;
        });

        delete_nodes
    }

    fn coalesce_nodes<'a>(&self, nodes: &Vec<tree_sitter::Node<'a>>) -> Vec<tree_sitter::Range> {
        let ranges = vec![];
        ranges
    }

    pub fn slice(&mut self, source_code: &str, target_point: tree_sitter::Point) -> String {
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(self.config.language).unwrap();

        let tree = parser.parse(source_code, None).unwrap();
        let root_node = tree.root_node();

        let target_name = self.name_at_point(&root_node, target_point).unwrap();

        // walk up to the containing function
        let mut target_func = target_name.node;
        // Cursors don't do what you'd expect here?
        // cur = target.walk();
        // assert_eq!(cur.goto_parent(), true); fails
        // i'm guessing cursors arent supposed to be able to walk "out" of their initial
        // node, but nothing in tree-sitter source seems to say that...
        loop {
            if self.config.slice_scope_types.contains(&target_func.kind()) {
                break;
            }
            target_func = target_func.parent().unwrap();
        };

        let mut target_names: HashSet<NameRef> = HashSet::new();
        target_names.insert(target_name.clone());

        target_names = self.propagate_targets(target_func, &target_names);
        let delete_nodes = self.flatten_unreferenced(target_func, &target_names);
        let delete_ranges = self.coalesce_nodes(&delete_nodes);

        return format!("{:?}", delete_ranges);
    }
}

fn main() {
    let source_code = "#include <stdio.h>
int main() {
    int x = 0;
    int y = 0;
    s.z = x;
    foo = s;
    foo.y = bar;
    return x;
}";

    let lang = guess_language::guess(Path::new("test.c"), source_code).unwrap();
    let slicer_config = from_guessed_language(lang).unwrap();
    let mut slicer = Slicer{
        config: slicer_config,
        src: String::from(source_code),
    };
    let reduced = slicer.slice(source_code, tree_sitter::Point::new(2, 8));

    println!("{}", reduced);
}
