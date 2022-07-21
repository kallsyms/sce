use std::cell::RefCell;
use std::collections::HashSet;
use std::hash::{Hash, Hasher};

use crate::traverse::depth_first;
use crate::slicer_config::SlicerConfig;

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

pub struct Slicer {
    pub config: SlicerConfig,
    pub src: String,
}

impl Slicer {
    fn contains_subtype(&self, types: &Vec<&'static str>, node: &tree_sitter::Node) -> bool {
        types.iter().any(|t| self.config.subtypes[&t.to_string()].contains(&node.kind().to_string()))
    }

    /// Return a Vec of all "name components", e.g. ["self", "foo", "bar"]
    fn name_components(&self, node: &tree_sitter::Node) -> Vec<String> {
        depth_first(*node)
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
                return Some(NameRef{node, components: self.name_components(&node)});
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
                names.push(NameRef{node: descendant.clone(), components: self.name_components(&descendant)});
                return false;
            }
            return true;
        });
        names
    }

    /// Propagate the set of target names out through all assignments
    fn propagate_targets<'a>(&self, outer_scope: &'a tree_sitter::Node, initial_target_names: &HashSet<NameRef<'a>>) -> HashSet<NameRef<'a>> {
        let mut target_names = initial_target_names.clone();

        // TODO: use depth_first.traverse_with_depth to push and pop scopes based on
        // var_definition_scope_types
        loop {
            let mut changed = false;

            for descendant in depth_first(*outer_scope) {
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
                        //println!("Propagating node {:?} adds {:?} to targets", descendant, node_target_names);
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
                    let name = NameRef{node: descendant, components: self.name_components(&descendant)};
                    if target_names.contains(&name) {
                        references.borrow_mut().insert(descendant);
                    }
                    return false;
                }
                return true;
            },
            // nothing to do on descend
            |_, _|{},
            // on ascend,
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
            if !self.contains_subtype(&self.config.statement_types, &statement) {
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

    /// Coalesce adjacent deleted spans only if they are adjacent in the AST
    fn coalesce_ranges<'a>(&self, nodes: &Vec<tree_sitter::Node<'a>>) -> Vec<tree_sitter::Range> {
        let mut ranges = vec![];

        let mut i = 0;
        while i < nodes.len() {
            let start = (nodes[i].start_byte(), nodes[i].start_position());

            let mut end_node = nodes[i];

            while i + 1 < nodes.len() {
                let mut next = end_node.next_sibling();

                // traverse to the nearest sibling of an ancestor if we don't have a sibling
                // ourselves
                let mut cur = end_node;
                while next.is_none() {
                    match cur.parent() {
                        Some(parent) => {
                            cur = parent;
                            next = parent.next_sibling();
                        }
                        None => {
                            break;
                        }
                    }
                }

                match next {
                    // we have a next node to check for range coalescing
                    Some(next) => {
                        // if it's the next in the vector of nodes, set the new end node to this
                        if next == nodes[i + 1] {
                            end_node = next;
                            i += 1;
                        } else {
                            // otherwise we can't do anything else, so break out
                            break;
                        }
                    },
                    None => {
                        // no next node, nothing to coalesce
                        break;
                    }
                }
            }

            let end = (end_node.end_byte(), end_node.end_position());

            ranges.push(tree_sitter::Range{
                start_byte: start.0,
                start_point: start.1,
                end_byte: end.0,
                end_point: end.1,
            });

            i += 1;
        }

        ranges
    }

    // fn delete_ranges(&self, ranges: &Vec<tree_sitter::Range>) -> String {
    //     // this assumes that there's only one statement per line, so it's safe to completely remove
    //     // any lines which have a range to delete within it.
    //     if ranges.len() == 0 {
    //         return self.src.clone();
    //     }

    //     let src_lines: Vec<&str> = self.src.split("\n").collect();
    //     let mut new: Vec<&str> = vec![];

    //     new.extend(src_lines[0..ranges[0].start_point.row].iter());
    //     for (a, b) in ranges.iter().zip(ranges[1..].iter()) {
    //         new.extend(src_lines[a.end_point.row + 1..b.start_point.row].iter());
    //     }
    //     new.extend(src_lines[ranges[ranges.len() - 1].end_point.row + 1..].iter());

    //     new.join("\n")
    // }

    fn delete_ranges(&self, ranges: &Vec<tree_sitter::Range>) -> String {
        let src_lines: Vec<&str> = self.src.split("\n").collect();
        let mut new: Vec<&str> = vec![];

        let mut i = 0;
        for range in ranges {
            if i < range.start_point.row {
                new.extend(src_lines[i..range.start_point.row].iter());
            }
            let prefix = &src_lines[range.start_point.row][0..range.start_point.column];
            if !prefix.trim().is_empty() {
                new.push(prefix);
            }
            let suffix = &src_lines[range.end_point.row][range.end_point.column..];
            if !suffix.trim().is_empty() {
                new.push(suffix);
            }
            i = range.end_point.row + 1;
        }
        new.extend(src_lines[i..].iter());

        new.join("\n")
    }

    pub fn slice(&mut self, target_point: tree_sitter::Point) -> String {
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(self.config.language).unwrap();

        let tree = parser.parse(&self.src, None).unwrap();
        let root_node = tree.root_node();
        //println!("{}", root_node.to_sexp());

        let target_name = self.name_at_point(&root_node, target_point).unwrap();

        // walk up to the containing function
        // TODO: maybe make this just "outermost block"?
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

        target_names = self.propagate_targets(&target_func, &target_names);
        let delete_nodes = self.flatten_unreferenced(target_func, &target_names);
        let delete_ranges = self.coalesce_ranges(&delete_nodes);

        //println!("delete_ranges: {:?}", delete_ranges);

        let sliced_source = self.delete_ranges(&delete_ranges);
        sliced_source
    }
}
