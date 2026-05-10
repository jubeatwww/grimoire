use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AssetFetchStatus {
    Pending,
    Cached,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Asset {
    pub id: Uuid,
    pub source_url: String,
    pub cache_path: PathBuf,
    pub media_type: Option<String>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub source_attribution: Option<String>,
    pub fetch_status: AssetFetchStatus,
    pub last_fetched_at: Option<DateTime<Utc>>,
}
