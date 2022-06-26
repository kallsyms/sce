use std::path::Path;
use std::collections::HashSet;
use std::hash::{Hash, Hasher};
use tree_sitter;
use tree_sitter_c;
mod guess_language;

struct SlicerConfig {
    language: tree_sitter::Language,
    // Type names representing "atomic" name fragments (e.g. `self`, `foo`, `bar`)
    identifier_types: Vec<&'static str>,
    // Type names representing any possible "complete" name (e.g. `self.foo.bar`)
    name_types: Vec<&'static str>,
    // Type names and the type names for the descendant target and source
    // representing ways a variable can flow into a new variable (e.g. `assignment`)
    propagating_types: Vec<(&'static str, (&'static str, &'static str))>,
    // Type names representing statements. Can use inheritance information from node-types.
    statement_types: Vec<&'static str>,
    // Type names representing scopes in which we can slice (usually just functions)
    scope_types: Vec<&'static str>,
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
                statement_types: vec!["_statement"],
                scope_types: vec!["function_definition"],
            })
        }
        _ => None
    }
}

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
        self.components.hash(state);
    }
}

impl<'a> NameRef<'a> {
    fn affects(&self, other: NameRef) -> bool {
        let len = self.components.len().min(other.components.len());
        return self.components[..len].iter().zip(other.components[..len].iter()).all(|(a, b)| a == b);
    }
}

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
    // Call the given cb for each node, skipping any descendants of a given node
    // if the cb returns false.
    fn traverse<F>(&mut self, mut cb: F) where F: FnMut(tree_sitter::Node<'a>) -> bool {
        loop {
            if cb(self.cursor.node()) {
                if self.cursor.goto_first_child() {
                    continue;
                }
            }

            if self.cursor.goto_next_sibling() {
                continue;
            }

            loop {
                self.cursor.goto_parent();

                if self.cursor.node() == self.root {
                    return;
                }

                if self.cursor.goto_next_sibling() {
                    continue;
                }
            }
        }
    }
}

struct Slicer {
    config: SlicerConfig,
}

impl Slicer {
    fn name_components<'a>(&self, src: &'a str, node: tree_sitter::Node) -> Vec<String> {
        depth_first(node)
            .filter(|&descendant| self.config.identifier_types.contains(&descendant.kind()))
            .map(|descendant| String::from(&src[descendant.start_byte()..descendant.end_byte()]))
            .into_iter().collect()
    }

    fn name_at_point<'a>(&self, src: &'a str, root: &'a tree_sitter::Node, point: tree_sitter::Point) -> Option<NameRef<'a>> {
        let mut cur = root.walk();

        loop {
            let node = cur.node();

            if self.config.name_types.contains(&node.kind()) {
                // Walk down and gather all specific identifiers
                return Some(NameRef{node, components: self.name_components(src, node)});
            }

            if cur.goto_first_child_for_point(point) == None {
                return None;
            }
        }
    }

    fn referenced_names<'a>(&self, src: &'a str, node: tree_sitter::Node<'a>) -> Vec<NameRef<'a>> {
        let mut names = vec![];
        depth_first(node).traverse(|descendant| {
            if self.config.name_types.contains(&descendant.kind()) {
                names.push(NameRef{node: descendant.clone(), components: self.name_components(src, descendant)});
                return false;
            }
            return true;
        });
        names
    }

    pub fn slice(&mut self, source_code: &str, target_point: tree_sitter::Point) -> String {
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(self.config.language).unwrap();

        let tree = parser.parse(source_code, None).unwrap();
        let root_node = tree.root_node();
        println!("{}", root_node.to_sexp());

        let target = self.name_at_point(source_code, &root_node, target_point).unwrap();

        let mut target_func = target.node;
        // Cursors don't do what you'd expect here?
        // cur = target.walk();
        // assert_eq!(cur.goto_parent(), true); fails
        // i'm guessing cursors arent supposed to be able to walk "out" of their initial
        // node, but nothing in tree-sitter source seems to say that...
        loop {
            if self.config.scope_types.contains(&target_func.kind()) {
                break;
            }
            target_func = target_func.parent().unwrap();
        };

        let mut target_names: HashSet<NameRef> = HashSet::new();
        target_names.insert(target.clone());

        return format!("{:?}", target);
    }
}

fn main() {
    let source_code = "#include <stdio.h>
int main() {
    int x = 0;
    s.y = 0;
    return x;
}";

    let lang = guess_language::guess(Path::new("test.c"), source_code).unwrap();
    let slicer_config = from_guessed_language(lang).unwrap();
    let mut slicer = Slicer{
        config: slicer_config,
    };
    let reduced = slicer.slice(source_code, tree_sitter::Point::new(3, 6));

    println!("{}", reduced);
}
