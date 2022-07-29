use std::fs;
use std::path::Path;
use serde::Deserialize;

use slicer::guess_language::guess as guess_language;
use slicer::slicer::Slicer;
use slicer::slicer_config::from_guessed_language;

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
}

const TEST_BASE_DIR: &str = "tests/files/";

#[test]
fn test_slice() {
    // TODO: https://github.com/commure/datatest seems almost perfect, but nightly only :(
    for file in fs::read_dir(TEST_BASE_DIR).unwrap() {
        let path = file.unwrap().path();
        let output_contents = fs::read_to_string(&path).unwrap();
        if !output_contents.contains("TEST:") {
            continue;
        }

        let test_line = output_contents.lines().next().unwrap().split("TEST:").last().unwrap();
        let test: SliceTest = serde_json::from_str(&test_line).unwrap();

        let input_contents = fs::read_to_string(Path::new(TEST_BASE_DIR).join(&test.source)).unwrap();

        // ensure var is what the test thinks it is
        let target_line = input_contents.lines().skip(test.point.0 - 1).next().unwrap();
        let target_var = target_line[test.point.1 - 1..test.point.1 - 1 + test.var.len()];
        assert_eq!(target_var, test.var);

        let lang = guess_language(&path, &input_contents).unwrap();
        let slicer_config = from_guessed_language(lang).unwrap();

        let mut slicer = Slicer{
            config: slicer_config,
            src: input_contents,
        };
        let (sliced, _) = slicer.slice(tree_sitter::Point::new(test.point.0 - 1, test.point.1)).unwrap();

        assert_eq!(sliced.trim(), output_contents.lines().skip(1).collect::<Vec<&str>>().join("\n"));
    }
}
