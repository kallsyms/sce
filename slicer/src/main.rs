use std::path::Path;
use std::str::FromStr;
use serde::{Deserialize, Serialize};

use slicer::guess_language::{Language, guess as guess_language};
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
    /// The direction of the slice (forward or backward)
    direction: SliceDirection,
}

#[derive(Deserialize)]
/// Request to inline the given target function at the given call site.
/// N.B. Some LSPs support inlining already (see https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#:~:text=constant%0A%09%20*%20%2D%20...%0A%09%20*/%0A%09export%20const-,RefactorInline,-%3A%20CodeActionKind%20%3D)
/// however many don't (notably clangd/any C(++) LSP I can find),
/// so this still provides one (albeit a "simpler" AST-based approach).
struct InlineRequest {
    /// The content (source) of the file which holds the definition of the target function
    target_content: String,
    /// The point of the target function definition
    target_point: SerializablePoint,
}

#[derive(Deserialize)]
enum RequestOperation{
    Slice,
    Inline,
}

#[derive(Deserialize)]
struct Request {
    /// The filename of the file to operate on. Mainly used for guessing the language.
    filename: String,
    /// The language of the file, if known.
    language: Option<String>,
    /// The content (source) of the file to operate on.
    content: String,
    /// The point of the cursor in the file.
    point: SerializablePoint,

    /// The desired operation, slice or inline.
    operation: RequestOperation,
    slice: Option<SliceRequest>,
    inline: Option<InlineRequest>,
}


#[derive(Serialize)]
struct SliceResponse {
    /// The list of ranges which should be removed/hidden to show the slice.
    ranges_to_remove: Vec<SerializableRange>,
}

#[derive(Serialize)]
struct InlineResponse {
    /// The full content of the file with the target function definition inlined.
    content: String,
}

fn main() {
    env_logger::init();

    let req: Request = serde_json::from_reader(std::io::stdin()).unwrap();

    let lang = match req.language {
        Some(lang) => Language::from_str(&lang).unwrap(),
        _ => guess_language(Path::new(&req.filename), &req.content).unwrap(),
    };
    let slicer_config = from_guessed_language(lang).unwrap();

    let mut slicer = Slicer{
        config: slicer_config,
        src: req.content,
    };

    match req.operation {
        RequestOperation::Slice => {
            let ranges_to_remove = slicer.slice(req.point.into(), req.slice.unwrap().direction).unwrap();

            serde_json::to_writer(std::io::stdout(), &SliceResponse{
                ranges_to_remove: ranges_to_remove.into_iter().map(|r| SerializableRange::from(r)).collect(),
            }).unwrap();
        }
        RequestOperation::Inline => {
            let inline = req.inline.unwrap();
            let content = slicer.inline(req.point.into(), &inline.target_content, inline.target_point.into()).unwrap();

            serde_json::to_writer(std::io::stdout(), &InlineResponse{
                content: content,
            }).unwrap();
        }
    }
}
