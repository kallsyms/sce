use serde::Deserialize;
use std::cell::RefCell;
use std::collections::{HashSet, HashMap};
use std::hash::{Hash, Hasher};
use thiserror::Error;

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

struct InlineTempVar {
    name: String,
    value: String,
    typ: String,
}

impl InlineTempVar {
    fn format(&self, fmt: &str) -> String {
        fmt.clone()
        .replace("{name}", &self.name)
        .replace("{value}", &self.value)
        .replace("{type}", &self.typ)
    }
}

#[derive(Deserialize)]
pub enum SliceDirection {
    Backward,
    Forward,
}

#[derive(Error, Debug)]
pub enum SliceError {
    #[error("tree-sitter version mismatch: {0}")]
    TreeSitterVersionError(tree_sitter::LanguageError),
    // #[error("Parse error")]
    // ParseError,
    #[error("No identifier at point {0}")]
    NoNameAtPointError(tree_sitter::Point),
    #[error("No call at point {0}")]
    NoCallAtPointError(tree_sitter::Point),
}

pub struct Slicer {
    pub config: SlicerConfig,
    pub src: String,
}

#[derive(Debug)]
enum RewriteValue<'a> {
    None,
    String(String),
    Node(tree_sitter::Node<'a>),
}

impl Slicer {
    fn contains_subtype(&self, types: &Vec<&'static str>, node: &tree_sitter::Node) -> bool {
        types.iter().any(|t| self.config.subtypes[&t.to_string()].contains(&node.kind().to_string()))
    }

    /// Return a Vec of all "name components", e.g. ["self", "foo", "bar"]
    fn name_components(&self, node: &tree_sitter::Node) -> Vec<String> {
        depth_first(*node)
            .filter(|&descendant| self.config.identifier_types.contains(&descendant.kind()))
            .map(|descendant| String::from(&self.src[descendant.byte_range()]))
            .into_iter().collect()
    }

    fn node_of_kind_for_point<'a>(&self, root: &'a tree_sitter::Node, kinds: &Vec<&'static str>, point: tree_sitter::Point) -> Option<tree_sitter::Node<'a>> {
        let mut cur = root.walk();

        loop {
            let node = cur.node();

            if kinds.contains(&node.kind()) {
                return Some(node);
            }

            // Either we progress down to a child node which contains the point, or we bail out.
            if cur.goto_first_child_for_point(point) == None {
                return None;
            }
        }
    }

    /// Find the name reference at the specified point, if an identifier is referenced at that
    /// point.
    fn name_at_point<'a>(&self, root: &'a tree_sitter::Node, point: tree_sitter::Point) -> Option<NameRef<'a>> {
        let node = self.node_of_kind_for_point(root, &self.config.name_types, point)?;
        Some(NameRef{node, components: self.name_components(&node)})
    }

    /// List all names referenced by this node or any descendant.
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

    /// Propagate the set of target names out through all assignments until we hit a fixed point.
    fn propagate_targets<'a>(&self, outer_scope: &'a tree_sitter::Node, initial_target_names: &HashSet<NameRef<'a>>, direction: SliceDirection) -> HashSet<NameRef<'a>> {
        let mut target_names = initial_target_names.clone();

        // TODO: use depth_first.traverse_with_depth to push and pop scopes based on
        // var_definition_scope_types
        loop {
            let len_before = target_names.len();

            for descendant in depth_first(*outer_scope) {
                if let Some((_, (defs_child_name, refs_child_name))) = self.config.propagating_types.iter().find(|&&(expr_kind, (_, _))| expr_kind == descendant.kind()) {
                    let defs_node = descendant.child_by_field_name(defs_child_name);
                    let refs_node = descendant.child_by_field_name(refs_child_name);

                    // Guard against things like python's `with` which may or may not define
                    // variable(s)
                    if defs_node.is_none() || refs_node.is_none() {
                        continue;
                    }

                    let node_defs_names = self.referenced_names(defs_node.unwrap());
                    let node_refs_names = self.referenced_names(refs_node.unwrap());
                    log::debug!("defs {:?} refs {:?}", node_defs_names, node_refs_names);

                    match direction {
                        SliceDirection::Backward => {
                            // if any known target is used in a defs, all refss in the
                            // assign should now be targets
                            if target_names.iter().any(|tname| node_defs_names.iter().any(|dname| tname.affects(&dname))) {
                                log::info!("Propagating node {:?} adds {:?} to targets", descendant, node_refs_names);
                                target_names.extend(node_refs_names.clone());
                            }
                        },
                        SliceDirection::Forward => {
                            // opposite: if any known target is used in a refs, all defss
                            // should be targets.
                            if target_names.iter().any(|tname| node_refs_names.iter().any(|sname| tname.affects(&sname))) {
                                log::info!("Propagating node {:?} adds {:?} to targets", descendant, node_defs_names);
                                target_names.extend(node_defs_names.clone());
                            }
                        },
                    }
                }
            }
            
            if target_names.len() == len_before {
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
                    if target_names.iter().any(|tname| tname.affects(&name)) {
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

    pub fn slice(&mut self, target_point: tree_sitter::Point, direction: SliceDirection) -> Result<Vec<tree_sitter::Range>, SliceError> {
        let mut parser = tree_sitter::Parser::new();
        if let Err(lang_err) = parser.set_language(self.config.language) {
            return Err(SliceError::TreeSitterVersionError(lang_err));
        }

        // unchecked unwrap since this only fails if:
        // 1. parser lang is unset (it is)
        // 2. timeout expired (we don't set any)
        // 3. cancellation flag set (we don't)
        let tree = parser.parse(&self.src, None).unwrap();
        let root_node = tree.root_node();
        log::debug!("sexp: {}", root_node.to_sexp());
        // TODO: check root_node.has_error()?

        let target_name = self.name_at_point(&root_node, target_point).ok_or(SliceError::NoNameAtPointError(target_point))?;
        log::debug!("targeting {:?}", target_name);

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

        target_names = self.propagate_targets(&target_func, &target_names, direction);
        log::info!("Final set of target names: {:?}", target_names);
        let delete_nodes = self.flatten_unreferenced(target_func, &target_names);
        let delete_ranges = self.coalesce_ranges(&delete_nodes);

        Ok(delete_ranges)
    }

    fn get_capture<'a>(&self, query: &tree_sitter::Query, capture_name: &str, node: tree_sitter::Node<'a>, content: &[u8]) -> Vec<tree_sitter::Node<'a>> {
        let capture_idx = query.capture_index_for_name(capture_name).unwrap();
        let mut cursor = tree_sitter::QueryCursor::new();

        cursor
        .captures(query, node, content)
        .map(|(m, _)| m.captures.iter().filter(|c| c.index == capture_idx).map(|c| c.node)).into_iter().flatten().collect()
    }

    fn get_captures<'a, const COUNT: usize>(&self, query: &tree_sitter::Query, capture_names: [&str; COUNT], node: tree_sitter::Node<'a>, content: &[u8]) -> Vec<[tree_sitter::Node<'a>; COUNT]> {
        let capture_idxs: Vec<u32> = capture_names.iter().map(|name| query.capture_index_for_name(name).unwrap()).collect();
        let mut cursor = tree_sitter::QueryCursor::new();

        cursor
        .matches(query, node, content)
        .map(|m| {
            let capture_map: HashMap<u32, tree_sitter::Node> = m.captures.iter().map(|c| (c.index, c.node)).collect();
            capture_idxs.iter().map(|idx| capture_map[idx]).collect::<Vec<tree_sitter::Node>>().try_into().unwrap()
        }).collect()
    }

    fn rewrite_names(&self, node: &tree_sitter::Node, rename_map: &HashMap<NameRef, String>, src: &str) -> String {
        let mut rewritten_src: String = String::new();

        let mut prev_byte = node.start_byte();
        depth_first(*node).traverse(|n| {
            if self.config.name_types.contains(&n.kind()) {
                let name = NameRef{node: n, components: self.name_components(&n)};
                if let Some(new_name) = rename_map.get(&name) {
                    rewritten_src += &src[prev_byte..n.start_byte()];
                    rewritten_src += new_name;
                    prev_byte = n.end_byte();
                }

                return false;
            }

            return true;
        });
        rewritten_src += &src[prev_byte..node.end_byte()];

        rewritten_src
    }

    pub fn inline(&mut self, point: tree_sitter::Point, target_content: &str, target_point: tree_sitter::Point) -> Result<String, SliceError> {
        let mut parser = tree_sitter::Parser::new();
        if let Err(lang_err) = parser.set_language(self.config.language) {
            return Err(SliceError::TreeSitterVersionError(lang_err));
        }

        // The tree of the file in which the call is made
        let tree = parser.parse(&self.src, None).unwrap();
        let root_node = tree.root_node();

        // The tree of the file in which the target function is defined
        let function_definition_file_tree = parser.parse(target_content, None).unwrap();
        let function_definition_file_root_node = function_definition_file_tree.root_node();

        let callsite = self.node_of_kind_for_point(&root_node, &self.config.function_call_types, point).ok_or(SliceError::NoCallAtPointError(point))?;
        log::debug!("callsite: {}", callsite.to_sexp());
        // The node in the function_definition_tree representing the target function's definition
        let function_definition = self.node_of_kind_for_point(&function_definition_file_root_node, &self.config.slice_scope_types, target_point).ok_or(SliceError::NoNameAtPointError(target_point))?;
        log::debug!("function_definition: {}", function_definition.to_sexp());

        // Create rename map of function parameter name to name/expression passed at callsite
        let call_args = self.get_capture(&self.config.call_args_query, "value", callsite, self.src.as_bytes());
        log::debug!("call_args: {:?}", call_args);

        let function_params = self.get_captures(&self.config.function_query, ["param_name", "param_type"], function_definition, target_content.as_bytes());
        log::debug!("function_params: {:?}", function_params);

        let function = self.get_captures(&self.config.function_query, ["function_type", "function_body"], function_definition, target_content.as_bytes());
        let [function_type, function_body] = function[0];
        let returns = self.get_captures(&self.config.returns_query, ["return_statement", "return_value"], function_definition, target_content.as_bytes());
        
        // TODO: check len of call_args and function_params are equal
        // TODO: this/self implicit first arg

        // First stage, basic variable substitution

        // Passed args which are more than a simple constant or name are put into temporary variables
        // e.g. inlining `foo(x=bar(baz))` would result in `let x = bar(baz); {contents of foo}`
        // to avoid giving the impression that `bar(baz)` is evaluated twice
        let mut temps: Vec<InlineTempVar> = vec![];
        
        let mut rename_map: HashMap<NameRef, String> = HashMap::new();

        for (arg, [param_name_node, param_type_node]) in call_args.iter().zip(function_params.iter()) {
            let param_name = self.name_at_point(&function_definition_file_root_node, param_name_node.start_position()).ok_or(SliceError::NoNameAtPointError(param_name_node.start_position()))?;

            if self.config.constant_types.contains(&arg.kind()) || self.config.name_types.contains(&arg.kind()) {
                rename_map.insert(param_name, self.src[arg.byte_range()].to_string());
            } else {
                let inline_name = format!("inline_{}", &target_content[param_name.node.byte_range()]);
                temps.push(InlineTempVar{
                    name: inline_name.clone(),
                    value: self.src[arg.byte_range()].to_string(),
                    // TODO: check if type is set before trying to pull content
                    // wont be applicable in e.g. python
                    typ: target_content[param_type_node.byte_range()].to_string(),
                });
                rename_map.insert(param_name, inline_name);
            }
        }
        log::debug!("rename_map: {:?}", rename_map);

        // Second stage, inlining
        // Replace returns in the function body
        // Replace the call site with the return expression

        // The value that a node should be rewritten to, if applicable
        let mut rewrite_map: HashMap<tree_sitter::Node, RewriteValue> = HashMap::new();

        // TODO: ensure that the return value is the last statement in the function body
        // could have an early return in a conditional in a void function for example
        // For single returns, just inline the return value.
        // For multiple returns, hoist the statement the call site is in to where the return is
        // and have a comment below saying "continue on line #x".
        let callsite_rewrite = match &returns[..] {
            [ret] => {
                let [return_stmt, retval] = ret;
                rewrite_map.insert(return_stmt.clone(), RewriteValue::None);
                // TODO: run retval through renamemap
                RewriteValue::Node(retval.clone())
            },
            _ => {
                RewriteValue::None
            }
        };

        let src_lines: Vec<&str> = self.src.split("\n").collect();
        let callsite_whitespace: String = src_lines[callsite.start_position().row].chars().take_while(|c| c.is_whitespace()).collect();

        let mut new_src = src_lines[0..callsite.start_position().row].join("\n") + "\n";

        for temp in temps {
            new_src += &callsite_whitespace;
            new_src += &temp.format(self.config.temp_var_format);
            new_src += "\n";
        }

        let mut start_byte = 0;
        let mut end_byte = 0;

        // Find the first byte of the first statement in the function body
        let mut cur = function_body.walk();
        for child in function_body.children(&mut cur) {
            if child.is_named() {
                if start_byte == 0 {
                    start_byte = child.start_byte();
                }
                end_byte = child.end_byte();
            }
        }

        let definition_whitespace: String = target_content[..end_byte].chars().rev().take_while(|c| c.is_whitespace()).collect();

        let mut prev_byte = start_byte;

        // Do the actual rewriting into a new inline_src string, just to make indentation fixups easier
        let mut inline_src: String = String::new();
        depth_first(function_body.clone()).traverse(|n| {
            if let Some(rewrite) = rewrite_map.get(&n) {
                inline_src += &target_content[prev_byte..n.start_byte()];
                match rewrite {
                    RewriteValue::String(s) => inline_src += s,
                    // TODO: do nameref rewriting for n
                    RewriteValue::Node(n) => inline_src += &self.rewrite_names(n, &rename_map, &target_content),
                    RewriteValue::None => (),
                }
                prev_byte = n.end_byte();

                return false;
            } else if self.config.name_types.contains(&n.kind()) {
                // TODO: use self.rewrite_names
                let name = NameRef{node: n, components: self.name_components(&n)};
                if let Some(new_name) = rename_map.get(&name) {
                    inline_src += &target_content[prev_byte..n.start_byte()];
                    inline_src += new_name;
                    prev_byte = n.end_byte();
                }

                return false;
            }
            
            return true;
        });
        inline_src += &target_content[prev_byte..end_byte];

        for line in inline_src.split("\n") {
            new_src += &callsite_whitespace;
            new_src += &(line.strip_prefix(&definition_whitespace).unwrap_or(&line));
            new_src += "\n";
        }

        new_src = new_src[..new_src.len()-1].to_string();

        new_src += &src_lines[callsite.start_position().row][0..callsite.start_position().column];
        match callsite_rewrite {
            RewriteValue::String(s) => new_src += &s,
            RewriteValue::Node(n) => new_src += &self.rewrite_names(&n, &rename_map, &target_content),
            RewriteValue::None => (),
        }

        new_src += &self.src[callsite.end_byte()..];

        Ok(new_src)
    }
}

/// Delete the given ranges from the src, returning both the source with lines removed as well as
/// the target_point adjusted to be pointing to the same location.
/// Ranges is assumed to be pre-sorted.
pub fn delete_ranges(src: &str, ranges: &Vec<tree_sitter::Range>, target_point: tree_sitter::Point) -> (String, tree_sitter::Point) {
    let src_lines: Vec<&str> = src.split("\n").collect();

    let mut target_point = target_point.clone();
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

        // the target point must be included in the final slice response (i.e. not deleted)
        // so no need to check for any weird cases.
        if range.end_point.row < target_point.row {
            let mut deleted_lines = range.end_point.row - range.start_point.row;
            if prefix.trim().is_empty() || suffix.trim().is_empty() {
                deleted_lines += 1;
            }
            target_point.row -= deleted_lines;
        }
    }
    new.extend(src_lines[i..].iter());

    (new.join("\n"), target_point)
}