use std::path::Path;
use serde::Deserialize;

mod guess_language;
mod slicer_config;
mod slicer;
mod traverse;

use guess_language::guess as guess_language;
use slicer_config::from_guessed_language;
use slicer::Slicer;

#[derive(Deserialize)]
struct SliceRequest {
    filename: String,
    content: String,
    point: (usize, usize),
}

fn main() {
    let req: SliceRequest = serde_json::from_reader(std::io::stdin()).unwrap();
    let lang = guess_language(Path::new(&req.filename), &req.content).unwrap();
    let slicer_config = from_guessed_language(lang).unwrap();
    let mut slicer = Slicer{
        config: slicer_config,
        src: req.content,
    };
    let reduced = slicer.slice(tree_sitter::Point::new(req.point.0, req.point.1));

    println!("{}", reduced);
}
