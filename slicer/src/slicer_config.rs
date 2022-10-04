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

    /// Type names representing constants (constant integers, true/false, null, etc.)
    pub constant_types: Vec<&'static str>,

    /// Type names and the field names for the descendant destination and source representing ways a
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

    // In general, the "accuracy" with detecting names and constructs is lower for slicing than it
    // is for inlining, hence the change to using actual queries below for inlining related things.
    // https://tree-sitter.github.io/tree-sitter/using-parsers#query-syntax

    /// Type names representing function calls.
    pub function_call_types: Vec<&'static str>,

    /// The tree-sitter query used to list function definition parameters.
    /// This should capture the name of parameters as @param_name, and the type of the parameters as @param_type.
    /// It should also capture the type of the function as @function_type, and the body of the function as @function_body.
    pub function_query: tree_sitter::Query,

    /// The tree-sitter query used to list function call arguments.
    /// This should capture the argument's value expression as @value.
    /// TODO: can this be simplified to a list of types (e.g. _expression) and we just find the outermost list?
    pub call_args_query: tree_sitter::Query,

    /// The tree-sitter query used to list return expressions.
    /// This should capture the return statement as @return_statement and the returned value expression as @return_value.
    pub returns_query: tree_sitter::Query,

    /// The format string used to generate temporary variables.
    /// e.g. `{type} {name} = {value};`
    pub temp_var_format: &'static str,
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
            // https://github.com/tree-sitter/tree-sitter-c/blob/master/src/grammar.json
            Some(SlicerConfig{
                language: unsafe {tree_sitter_c()},
                subtypes: expand_node_types(include_str!("../vendor/tree-sitter-c/src/node-types.json")),
                identifier_types: vec!["identifier", "field_identifier"],
                name_types: vec!["identifier", "field_expression"],
                constant_types: vec!["null", "true", "false", "number_literal", "string_literal", "character_literal"],
                propagating_types: vec![
                    ("assignment_expression", ("left", "right")),
                    ("init_declarator", ("declarator", "value")),
                ],
                statement_types: vec!["_statement", "declaration"],
                slice_scope_types: vec!["function_definition"],
                var_definition_scope_types: vec!["compound_statement"],
                function_call_types: vec!["call_expression"],
                function_query: tree_sitter::Query::new(unsafe {tree_sitter_c()}, "
                    (function_definition
                        type: (_type_specifier) @function_type
                        declarator: (function_declarator
                            parameters: (parameter_list
                                (parameter_declaration
                                    type: (_type_specifier) @param_type
                                    declarator: (_declarator) @param_name
                                )
                            )
                        )
                        body: (compound_statement) @function_body
                    )").unwrap(),
                call_args_query: tree_sitter::Query::new(unsafe {tree_sitter_c()}, "
                    (call_expression
                        arguments: (argument_list
                            \"(\"
                            (_expression) @value
                            \")\"
                        )
                    )").unwrap(),
                returns_query: tree_sitter::Query::new(unsafe {tree_sitter_c()}, "
                    (return_statement
                        (_expression) @return_value
                    ) @return_statement").unwrap(),
                temp_var_format: "{type} {name} = {value};",
            })
        }
        // CPlusPlus => {
        //     Some(SlicerConfig{
        //         language: unsafe {tree_sitter_cpp()},
        //         subtypes: expand_node_types(include_str!("../vendor/tree-sitter-cpp/src/node-types.json")),
        //         identifier_types: vec!["identifier", "field_identifier"],
        //         name_types: vec!["identifier", "field_expression"],
        //         constant_types: vec![],  // TODO
        //         propagating_types: vec![
        //             ("assignment_expression", ("left", "right")),
        //             ("init_declarator", ("declarator", "value")),
        //             // TODO: for in
        //         ],
        //         statement_types: vec!["_statement", "declaration"],
        //         slice_scope_types: vec!["function_definition"],
        //         var_definition_scope_types: vec!["compound_statement"],
        //         function_call_types: vec![""],
        //         function_query: tree_sitter::Query::new(unsafe {tree_sitter_cpp()}, "").unwrap(),
        //         call_args_query: tree_sitter::Query::new(unsafe {tree_sitter_cpp()}, "").unwrap(),
        //     })
        // }
        // CSharp => {
        //     Some(SlicerConfig{
        //         language: unsafe {tree_sitter_c_sharp()},
        //         subtypes: expand_node_types(include_str!("../vendor/tree-sitter-c-sharp/src/node-types.json")),
        //         identifier_types: vec!["identifier"],
        //         name_types: vec!["identifier", "member_access_expression"],
        //         constant_types: vec![],  // TODO
        //         propagating_types: vec![
        //             ("assignment_expression", ("left", "right")),
        //             // TODO: for in
        //         ],
        //         statement_types: vec!["_statement"],
        //         slice_scope_types: vec!["_function_body", "method_declaration"],
        //         var_definition_scope_types: vec!["block"],
        //         function_call_types: vec![""],
        //         function_query: tree_sitter::Query::new(unsafe {tree_sitter_c_sharp()}, "").unwrap(),
        //         call_args_query: tree_sitter::Query::new(unsafe {tree_sitter_c_sharp()}, "").unwrap(),
        //     })
        // }
        // Go => {
        //     Some(SlicerConfig{
        //         language: unsafe {tree_sitter_go()},
        //         subtypes: expand_node_types(include_str!("../vendor/tree-sitter-go/src/node-types.json")),
        //         identifier_types: vec!["identifier", "field_identifier"],
        //         name_types: vec!["identifier", "selector_expression"],
        //         constant_types: vec![],  // TODO
        //         propagating_types: vec![
        //             ("assignment_statement", ("left", "right")),
        //             ("short_var_declaration", ("left", "right")),
        //         ],
        //         statement_types: vec!["_statement"],
        //         slice_scope_types: vec!["function_declaration"],
        //         var_definition_scope_types: vec!["block"],
        //         function_call_types: vec![""],
        //         function_query: tree_sitter::Query::new(unsafe {tree_sitter_go()}, "").unwrap(),
        //         call_args_query: tree_sitter::Query::new(unsafe {tree_sitter_go()}, "").unwrap(),
        //     })
        // }
        // Java => {
        //     Some(SlicerConfig{
        //         language: unsafe {tree_sitter_java()},
        //         subtypes: expand_node_types(include_str!("../vendor/tree-sitter-java/src/node-types.json")),
        //         identifier_types: vec!["identifier"],
        //         name_types: vec!["identifier", "field_access"],
        //         constant_types: vec![],  // TODO
        //         propagating_types: vec![
        //             ("assignment_expression", ("left", "right")),
        //             // TODO: for in
        //         ],
        //         statement_types: vec!["statement"],
        //         slice_scope_types: vec!["method_declaration"],
        //         var_definition_scope_types: vec!["block"],
        //         function_call_types: vec![""],
        //         function_query: tree_sitter::Query::new(unsafe {tree_sitter_java()}, "").unwrap(),
        //         call_args_query: tree_sitter::Query::new(unsafe {tree_sitter_java()}, "").unwrap(),
        //     })
        // }
        // JavaScript => {
        //     Some(SlicerConfig{
        //         language: unsafe {tree_sitter_javascript()},
        //         subtypes: expand_node_types(include_str!("../vendor/tree-sitter-javascript/src/node-types.json")),
        //         identifier_types: vec!["identifier", "property_identifier"],
        //         name_types: vec!["identifier", "member_expression"],
        //         constant_types: vec![],  // TODO
        //         propagating_types: vec![
        //             ("assignment_expression", ("left", "right")),
        //             ("variable_declarator", ("name", "value")),
        //         ],
        //         statement_types: vec!["statement"],
        //         slice_scope_types: vec!["function_declaration", "generator_function_declaration", "arrow_function", "method_definition"],
        //         var_definition_scope_types: vec!["statement_block"],
        //         function_call_types: vec![""],
        //         function_query: tree_sitter::Query::new(unsafe {tree_sitter_javascript()}, "").unwrap(),
        //         call_args_query: tree_sitter::Query::new(unsafe {tree_sitter_javascript()}, "").unwrap(),
        //     })
        // }
        // Python => {
        //     Some(SlicerConfig{
        //         language: unsafe {tree_sitter_python()},
        //         subtypes: expand_node_types(include_str!("../vendor/tree-sitter-python/src/node-types.json")),
        //         identifier_types: vec!["identifier"],
        //         name_types: vec!["identifier", "attribute"],
        //         constant_types: vec![],  // TODO
        //         propagating_types: vec![
        //             ("assignment", ("left", "right")),
        //             ("with_item", ("alias", "value")),
        //         ],
        //         statement_types: vec!["_compound_statement", "_simple_statement"],
        //         slice_scope_types: vec!["function_definition"],
        //         var_definition_scope_types: vec!["function_definition"],
        //         function_call_types: vec![""],
        //         function_query: tree_sitter::Query::new(unsafe {tree_sitter_python()}, "").unwrap(),
        //         call_args_query: tree_sitter::Query::new(unsafe {tree_sitter_python()}, "").unwrap(),
        //     })
        // }
        // Ruby => {
        //     Some(SlicerConfig{
        //         language: unsafe {tree_sitter_ruby()},
        //         subtypes: expand_node_types(include_str!("../vendor/tree-sitter-ruby/src/node-types.json")),
        //         identifier_types: vec!["identifier"],
        //         name_types: vec!["identifier", "call"],
        //         constant_types: vec![],  // TODO
        //         propagating_types: vec![
        //             ("assignment", ("left", "right")),
        //         ],
        //         // Can't use _primary since that includes like `integer`
        //         statement_types: vec!["_statement", "begin", "while", "until", "if", "unless", "for", "case"],
        //         slice_scope_types: vec!["method", "singleton_method"],
        //         var_definition_scope_types: vec!["method", "singleton_method"],
        //         function_call_types: vec![""],
        //         function_query: tree_sitter::Query::new(unsafe {tree_sitter_ruby()}, "").unwrap(),
        //         call_args_query: tree_sitter::Query::new(unsafe {tree_sitter_ruby()}, "").unwrap(),
        //     })
        // }
        // Rust => {
        //     Some(SlicerConfig{
        //         language: unsafe {tree_sitter_rust()},
        //         subtypes: expand_node_types(include_str!("../vendor/tree-sitter-rust/src/node-types.json")),
        //         identifier_types: vec!["identifier"],
        //         name_types: vec!["identifier", "token_tree"],
        //         constant_types: vec![],  // TODO
        //         propagating_types: vec![
        //             ("assignment_expression", ("left", "right")),
        //             ("let_declaration", ("pattern", "value")),
        //             // TODO: for in, if let, while let if those don't already work
        //         ],
        //         // # treesitter (and maybe rust's spec?) doesn't have a normal "statement"
        //         // so we have to do our best and enumerate what is normally used as a statement
        //         statement_types: vec![
        //             "let_declaration",
        //             "macro_invocation",
        //             "assignment_expression",
        //             "await_expression",
        //             "call_expression",
        //             "compound_assignment_expr",
        //             "for_expression",
        //             "if_expression",
        //             "if_let_expression",
        //             "loop_expression",
        //             "match_expression",
        //             "return_expression",
        //             "struct_expression",
        //             "try_expression",
        //             "while_expression",
        //             "while_let_expression",
        //         ],
        //         slice_scope_types: vec!["function_item"],
        //         var_definition_scope_types: vec!["block"],
        //         function_call_types: vec![""],
        //         function_query: tree_sitter::Query::new(unsafe {tree_sitter_rust()}, "").unwrap(),
        //         call_args_query: tree_sitter::Query::new(unsafe {tree_sitter_rust()}, "").unwrap(),
        //     })
        // }
        // TypeScript => {
        //     Some(SlicerConfig{
        //         language: unsafe {tree_sitter_typescript()},
        //         subtypes: expand_node_types(include_str!("../vendor/tree-sitter-typescript/typescript/src/node-types.json")),
        //         identifier_types: vec!["identifier", "property_identifier"],
        //         name_types: vec!["identifier", "member_expression"],
        //         constant_types: vec![],  // TODO
        //         propagating_types: vec![
        //             ("assignment_expression", ("left", "right")),
        //             ("variable_declarator", ("name", "value")),
        //         ],
        //         statement_types: vec!["statement"],
        //         slice_scope_types: vec!["function_declaration", "generator_function_declaration", "arrow_function", "method_definition"],
        //         var_definition_scope_types: vec!["statement_block"],
        //         function_call_types: vec![""],
        //         function_query: tree_sitter::Query::new(unsafe {tree_sitter_typescript()}, "").unwrap(),
        //         call_args_query: tree_sitter::Query::new(unsafe {tree_sitter_typescript()}, "").unwrap(),
        //     })
        // }
        _ => None
    }
}

