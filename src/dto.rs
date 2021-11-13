
use serde::{Deserialize, Serialize};
use tantivy::schema::NamedFieldDocument;

pub struct AddDocReq {
    pub doc: String,
    pub commit: bool,
}

#[derive(Deserialize)]
pub struct DeleteByTermReq {
    pub field: String,
    pub term: String,
    pub commit: bool,
}

#[derive(Deserialize)]
pub struct SearchReq {
    pub query: String,
    pub limit: usize,
    pub offset: usize,
}

pub type Score = f32;

#[derive(Serialize)]
pub struct ScoredDocument<D = NamedFieldDocument> {
    pub score: Score,
    pub doc: D,
}

#[derive(Serialize)]
pub struct SearchResp {
    pub docs: Vec<ScoredDocument>
}