use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetadataCandidate {
    pub id: Uuid,
    pub source_name: String,
    pub source_work_id: String,
    pub source_url: String,
    pub query_used: String,
    pub rank: i32,
    pub title: String,
    pub circle: Option<String>,
    pub cover_url: Option<String>,
    pub normalized_payload: serde_json::Value,
}
