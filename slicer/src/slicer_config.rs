use std::collections::HashMap;
use serde::Deserialize;

use crate::guess_language;

extern "C" {
    pub fn tree_sitter_c() -> tree_sitter::Language;
    pub fn tree_sitter_cpp() -> tree_sitter::Language;
    pub fn tree_sitter_c_sharp() -> tree_sitter::Language;
    pub fn tree_sitter_go() -> tree_sitter::Language;
    pub fn tree_sitter_java() -> tree_sitter::Language;
    pub fn tree_sitter_javascript() -> tree_sitter::Language;
    pub fn tree_sitter_python() -> tree_sitter::Language;
    pub fn tree_sitter_ruby() -> tree_sitter::Language;
    pub fn tree_sitter_rust() -> tree_sitter::Language;
    pub fn tree_sitter_typescript() -> tree_sitter::Language;
}

/// SlicerConfig is the main configuration for the slicer.
/// This includes all language-specific tree-sitter type names which various stages of the slicing
/// need.
pub struct SlicerConfig {
    /// The tree_sitter language the slicer should use to parse with
    pub language: tree_sitter::Language,

    /// Subtype information from NODE_TYPES
    pub subtypes: HashMap<String, Vec<String>>,

    /// Type names representing "atomic" name fragments (e.g. `self`, `foo`, `bar`)
    pub identifier_types: Vec<&'static str>,

    /// Type names representing any possible "complete" name (e.g. `self.foo.bar`)
    pub name_types: Vec<&'static str>,

    /// Type names and the type names for the descendant target and source representing ways a
    /// variable can flow into a new variable (e.g. assignment).
    /// e.g. ("assignment_expression", ("left", "right"))
    pub propagating_types: Vec<(&'static str, (&'static str, &'static str))>,

    /// Type names representing statements. Can use "inheritance" information from node-types.
    pub statement_types: Vec<&'static str>,

    /// Type names representing scopes in which we can slice (just functions?)
    pub slice_scope_types: Vec<&'static str>,

    /// Type names representing variable accessibility "boundaries" in the language, where
    /// variables defined within are not accessible outside of.
    /// For Python, this would be function level, but for C-like languages, this would be
    /// block-level.
    pub var_definition_scope_types: Vec<&'static str>,
}

#[derive(Deserialize)]
struct NodeType {
    r#type: String,
    #[serde(default)]
    subtypes: Vec<NodeType>,
}

fn expand_node_types(node_types_json: &str) -> HashMap<String, Vec<String>> {
    let mut subtypes = HashMap::new();

    for node_type in serde_json::from_str::<Vec<NodeType>>(node_types_json).unwrap() {
        let mut node_subtypes = vec![node_type.r#type.clone()];
        node_subtypes.extend(node_type.subtypes.iter().map(|t| t.r#type.clone()));
        subtypes.insert(node_type.r#type, node_subtypes);
    }

    subtypes
}

pub fn from_guessed_language(language: guess_language::Language) -> Option<SlicerConfig> {
    use guess_language::Language::*;

    match language {
        C => {
            Some(SlicerConfig{
                language: unsafe {tree_sitter_c()},
                subtypes: expand_node_types(include_str!("../vendor/tree-sitter-c/src/node-types.json")),
                identifier_types: vec!["identifier", "field_identifier"],
                name_types: vec!["identifier", "field_expression"],
                propagating_types: vec![
                    ("assignment_expression", ("left", "right")),
                ],
                statement_types: vec!["_statement", "declaration"],
                slice_scope_types: vec!["function_definition"],
                var_definition_scope_types: vec!["compound_statement"],
            })
        }
        CPlusPlus => {
            Some(SlicerConfig{
                language: unsafe {tree_sitter_cpp()},
                subtypes: expand_node_types(include_str!("../vendor/tree-sitter-cpp/src/node-types.json")),
                identifier_types: vec!["identifier", "field_identifier"],
                name_types: vec!["identifier", "field_expression"],
                propagating_types: vec![
                    ("assignment_expression", ("left", "right")),
                    // TODO: for in
                ],
                statement_types: vec!["_statement", "declaration"],
                slice_scope_types: vec!["function_definition"],
                var_definition_scope_types: vec!["compound_statement"],
            })
        }
        CSharp => {
            Some(SlicerConfig{
                language: unsafe {tree_sitter_c_sharp()},
                subtypes: expand_node_types(include_str!("../vendor/tree-sitter-c-sharp/src/node-types.json")),
                identifier_types: vec!["identifier"],
                name_types: vec!["identifier", "member_access_expression"],
                propagating_types: vec![
                    ("assignment_expression", ("left", "right")),
                    // TODO: for in
                ],
                statement_types: vec!["_statement"],
                slice_scope_types: vec!["_function_body", "method_declaration"],
                var_definition_scope_types: vec!["block"],
            })
        }
        Go => {
            Some(SlicerConfig{
                language: unsafe {tree_sitter_go()},
                subtypes: expand_node_types(include_str!("../vendor/tree-sitter-go/src/node-types.json")),
                identifier_types: vec!["identifier", "field_identifier"],
                name_types: vec!["identifier", "selector_expression"],
                propagating_types: vec![
                    ("assignment_statement", ("left", "right")),
                    ("short_var_declaration", ("left", "right")),
                ],
                statement_types: vec!["_statement"],
                slice_scope_types: vec!["function_declaration"],
                var_definition_scope_types: vec!["block"],
            })
        }
        Java => {
            Some(SlicerConfig{
                language: unsafe {tree_sitter_java()},
                subtypes: expand_node_types(include_str!("../vendor/tree-sitter-java/src/node-types.json")),
                identifier_types: vec!["identifier"],
                name_types: vec!["identifier", "field_access"],
                propagating_types: vec![
                    ("assignment_expression", ("left", "right")),
                    // TODO: for in
                ],
                statement_types: vec!["statement"],
                slice_scope_types: vec!["method_declaration"],
                var_definition_scope_types: vec!["block"],
            })
        }
        JavaScript => {
            Some(SlicerConfig{
                language: unsafe {tree_sitter_javascript()},
                subtypes: expand_node_types(include_str!("../vendor/tree-sitter-javascript/src/node-types.json")),
                identifier_types: vec!["identifier", "property_identifier"],
                name_types: vec!["identifier", "member_expression"],
                propagating_types: vec![
                    ("assignment_expression", ("left", "right")),
                    ("variable_declarator", ("name", "value")),
                ],
                statement_types: vec!["statement"],
                slice_scope_types: vec!["function_declaration", "generator_function_declaration", "arrow_function", "method_definition"],
                var_definition_scope_types: vec!["statement_block"],
            })
        }
        Python => {
            Some(SlicerConfig{
                language: unsafe {tree_sitter_python()},
                subtypes: expand_node_types(include_str!("../vendor/tree-sitter-python/src/node-types.json")),
                identifier_types: vec!["identifier"],
                name_types: vec!["identifier", "attribute"],
                propagating_types: vec![
                    ("assignment", ("left", "right")),
                    ("with_item", ("alias", "value")),
                ],
                statement_types: vec!["_compound_statement", "_simple_statement"],
                slice_scope_types: vec!["function_definition"],
                var_definition_scope_types: vec!["function_definition"],
            })
        }
        Ruby => {
            Some(SlicerConfig{
                language: unsafe {tree_sitter_ruby()},
                subtypes: expand_node_types(include_str!("../vendor/tree-sitter-ruby/src/node-types.json")),
                identifier_types: vec!["identifier"],
                name_types: vec!["identifier", "call"],
                propagating_types: vec![
                    ("assignment", ("left", "right")),
                ],
                // Can't use _primary since that includes like `integer`
                statement_types: vec!["_statement", "begin", "while", "until", "if", "unless", "for", "case"],
                slice_scope_types: vec!["method", "singleton_method"],
                var_definition_scope_types: vec!["method", "singleton_method"],
            })
        }
        Rust => {
            Some(SlicerConfig{
                language: unsafe {tree_sitter_rust()},
                subtypes: expand_node_types(include_str!("../vendor/tree-sitter-rust/src/node-types.json")),
                identifier_types: vec!["identifier"],
                name_types: vec!["identifier", "token_tree"],
                propagating_types: vec![
                    ("assignment_expression", ("left", "right")),
                    ("let_declaration", ("pattern", "value")),
                    // TODO: for in, if let, while let if those don't already work
                ],
                // # treesitter (and maybe rust's spec?) doesn't have a normal "statement"
                // so we have to do our best and enumerate what is normally used as a statement
                statement_types: vec![
                    "let_declaration",
                    "macro_invocation",
                    "assignment_expression",
                    "await_expression",
                    "call_expression",
                    "compound_assignment_expr",
                    "for_expression",
                    "if_expression",
                    "if_let_expression",
                    "loop_expression",
                    "match_expression",
                    "return_expression",
                    "struct_expression",
                    "try_expression",
                    "while_expression",
                    "while_let_expression",
                ],
                slice_scope_types: vec!["function_item"],
                var_definition_scope_types: vec!["block"],
            })
        }
        TypeScript => {
            Some(SlicerConfig{
                language: unsafe {tree_sitter_typescript()},
                subtypes: expand_node_types(include_str!("../vendor/tree-sitter-typescript/typescript/src/node-types.json")),
                identifier_types: vec!["identifier", "property_identifier"],
                name_types: vec!["identifier", "member_expression"],
                propagating_types: vec![
                    ("assignment_expression", ("left", "right")),
                    ("variable_declarator", ("name", "value")),
                ],
                statement_types: vec!["statement"],
                slice_scope_types: vec!["function_declaration", "generator_function_declaration", "arrow_function", "method_definition"],
                var_definition_scope_types: vec!["statement_block"],
            })
        }
        _ => None
    }
}

