use std::path::Path;
use std::str::FromStr;
use tonic::{transport::Server, Request, Response, Status};

use sce::guess_language::{Language, guess as guess_language};
use sce::engine_config::from_guessed_language;
use sce::engine::Engine;
use sce::rpc::{Source, SliceRequest, SliceResponse, InlineRequest, InlineResponse};
use sce::rpc::sce_server::{Sce, SceServer};

fn to_ts(point: &sce::rpc::Point) -> tree_sitter::Point {
    tree_sitter::Point {
        row: point.line as usize,
        column: point.col as usize,
    }
}

fn to_rpc(range: tree_sitter::Range) -> sce::rpc::Range {
    sce::rpc::Range {
        start: Some(sce::rpc::Point {
            line: range.start_point.row as u32,
            col: range.start_point.column as u32,
        }),
        end: Some(sce::rpc::Point {
            line: range.end_point.row as u32,
            col: range.end_point.column as u32,
        }),
    }
}

#[derive(Debug, Default)]
pub struct SCEService {}
impl SCEService {
    fn make_engine(source: &Source) -> Engine {
        let lang = match source.language.as_str() {
            "" => guess_language(Path::new(&source.filename), &source.content).unwrap(),
            _ => Language::from_str(&source.language).unwrap(),
        };
        let config = from_guessed_language(lang).unwrap();

        Engine{
            config,
            src: source.content.clone(),
        }
    }
}

#[tonic::async_trait]
impl Sce for SCEService {
    async fn slice(&self, request: Request<SliceRequest>) -> Result<Response<SliceResponse>, Status> {
        let req = request.into_inner();
        let direction = req.direction();
        let source = req.source.unwrap();

        let mut engine = Self::make_engine(&source);
        let ranges_to_remove = engine.slice(to_ts(&source.point.unwrap()), direction).unwrap();

        Ok(Response::new(SliceResponse{to_remove: ranges_to_remove.into_iter().map(|r| to_rpc(r)).collect()}))
    }

    async fn inline(&self, request: Request<InlineRequest>) -> Result<Response<InlineResponse>, Status> {
        let req = request.into_inner();
        let source = req.source.unwrap();

        let mut engine = Self::make_engine(&source);

        let content = engine.inline(to_ts(&source.point.unwrap()), &req.target_content, to_ts(&req.target_point.unwrap())).unwrap();

        Ok(Response::new(InlineResponse{content}))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let addr = "[::1]:1486".parse()?;
    let svc = SCEService::default();

    Server::builder()
        .add_service(SceServer::new(svc))
        .serve(addr)
        .await?;

    Ok(())
}