use std::path::Path;
use serde::{Deserialize, Serialize};

use slicer::guess_language::guess as guess_language;
use slicer::slicer_config::from_guessed_language;
use slicer::slicer::{Slicer, SliceDirection};

#[derive(Serialize, Deserialize)]
struct SerializablePoint((usize, usize));
impl From<tree_sitter::Point> for SerializablePoint {
    fn from(point: tree_sitter::Point) -> Self {
        SerializablePoint((point.row, point.column))
    }
}
impl Into<tree_sitter::Point> for SerializablePoint {
    fn into(self) -> tree_sitter::Point {
        tree_sitter::Point {
            row: self.0.0,
            column: self.0.1,
        }
    }
}

#[derive(Serialize, Deserialize)]
struct SerializableRange((SerializablePoint, SerializablePoint));
impl From<tree_sitter::Range> for SerializableRange {
    fn from(range: tree_sitter::Range) -> Self {
        SerializableRange((SerializablePoint::from(range.start_point), SerializablePoint::from(range.end_point)))
    }
}

#[derive(Deserialize)]
struct SliceRequest {
    filename: String,
    content: String,
    point: SerializablePoint,
    direction: SliceDirection,
}

#[derive(Serialize)]
struct SliceResponse {
    ranges_to_remove: Vec<SerializableRange>,
}

fn main() {
    env_logger::init();

    let req: SliceRequest = serde_json::from_reader(std::io::stdin()).unwrap();

    let lang = guess_language(Path::new(&req.filename), &req.content).unwrap();
    let slicer_config = from_guessed_language(lang).unwrap();

    let mut slicer = Slicer{
        config: slicer_config,
        src: req.content,
    };
    let ranges_to_remove = slicer.slice(req.point.into(), req.direction).unwrap();

    serde_json::to_writer(std::io::stdout(), &SliceResponse{
        ranges_to_remove: ranges_to_remove.into_iter().map(|r| SerializableRange::from(r)).collect(),
    }).unwrap();
}
