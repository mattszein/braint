use crate::EntryId;
use serde::{Deserialize, Serialize};

pub const METHOD_INGEST: &str = "ingest";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestRequest {
    pub text: String,
    pub source: crate::Source,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestResponse {
    pub entry_id: EntryId,
}
