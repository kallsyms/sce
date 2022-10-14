#![feature(custom_test_frameworks)]
#![test_runner(datatest::runner)]

use pretty_assertions::assert_eq;
use std::fs;
use std::path::Path;
use serde::Deserialize;

use sce::guess_language::guess as guess_language;
use sce::engine::{Engine, delete_ranges};
use sce::engine_config::from_guessed_language;

#[derive(Deserialize)]
// https://serde.rs/remote-derive.html
#[serde(remote = "sce::rpc::SliceDirection")]
pub enum SliceDirectionDef {
    Backward = 0,
    Forward = 1,
}

#[derive(Deserialize)]
struct SliceTest {
    /// The source file name
    source: String,
    /// The point which contains the variable to be sliced on, with rows and cols starting at 1 for
    /// ease of writing.
    point: (usize, usize),
    /// The name of the variable, just as a check to ensure point is correct and make the test more
    /// obvious.
    var: String,
    /// The direction of the slice, Backward or Forward
    #[serde(with = "SliceDirectionDef")]
    direction: sce::rpc::SliceDirection,
}

#[datatest::files("tests/files/", {
  path in r"slice.*",
})]
fn test_slice(path: &Path) {
    let _ = env_logger::try_init();

    let output_contents = fs::read_to_string(&path).unwrap();

    let test_line = output_contents.lines().next().unwrap().split("TEST:").last().unwrap();
    let test: SliceTest = match serde_json::from_str(&test_line) {
        Ok(t) => t,
        Err(_) => return,
    };

    let input_contents = fs::read_to_string(Path::new("tests/files/").join(&test.source)).unwrap();

    // ensure var is what the test thinks it is
    let target_line = input_contents.lines().skip(test.point.0 - 1).next().unwrap();
    let target_var = &target_line[test.point.1 - 1..test.point.1 - 1 + test.var.len()];
    assert_eq!(target_var, test.var);

    let lang = guess_language(&path, &input_contents).unwrap();
    let engine_config = from_guessed_language(lang).unwrap();

    let mut engine = Engine{
        config: engine_config,
        src: input_contents,
    };
    let point = tree_sitter::Point::new(test.point.0 - 1, test.point.1);
    let to_remove = engine.slice(point, test.direction).unwrap();
    let (sliced, _) = delete_ranges(&engine.src, &to_remove, point);

    // this is "backwards" because pretty_assertions diffs from a to b, and it's more intuitive if
    // we show what the slicer output is missing.
    assert_eq!(output_contents.lines().skip(1).collect::<Vec<&str>>().join("\n"), sliced.trim());
}

#[derive(Deserialize)]
struct InlineTest {
    /// The source file name
    source: String,
    /// The point which contains the function to be inlined, with rows and cols starting at 1 for
    /// ease of writing.
    point: (usize, usize),
    /// The name of the function, just as a check to ensure point is correct and make the test more
    /// obvious.
    func: String,
    /// The point which defines the target of the inline.
    target: (usize, usize),
}

#[datatest::files("tests/files/", {
  path in r"inline.*",
})]
fn test_inline(path: &Path) {
    let _ = env_logger::try_init();

    let output_contents = fs::read_to_string(&path).unwrap();

    let test_line = output_contents.lines().next().unwrap().split("TEST:").last().unwrap();
    let test: InlineTest = match serde_json::from_str(&test_line) {
        Ok(t) => t,
        Err(_) => return,
    };

    let input_contents = fs::read_to_string(Path::new("tests/files/").join(&test.source)).unwrap();

    // ensure var is what the test thinks it is
    let target_line = input_contents.lines().skip(test.point.0 - 1).next().unwrap();
    let target_func = &target_line[test.point.1 - 1..test.point.1 - 1 + test.func.len()];
    assert_eq!(target_func, test.func);

    let lang = guess_language(&path, &input_contents).unwrap();
    let engine_config = from_guessed_language(lang).unwrap();

    let mut engine = Engine{
        config: engine_config,
        src: input_contents.clone(),
    };
    let point = tree_sitter::Point::new(test.point.0 - 1, test.point.1);
    let target_point = tree_sitter::Point::new(test.target.0 - 1, test.target.1);
    let inlined = engine.inline(point, &input_contents, target_point).unwrap();

    assert_eq!(output_contents.lines().skip(1).collect::<Vec<&str>>().join("\n"), inlined);
}