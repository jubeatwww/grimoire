use crate::category::{GenreFacet, PrimaryCategory};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ImportStatus {
    Staged,
    Reviewed,
    ReadyToCommit,
    Committed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StagingItem {
    pub id: Uuid,
    pub staging_path: PathBuf,
    pub original_filename: String,
    pub file_size: u64,
    pub modified_at: DateTime<Utc>,
    pub content_hash: Option<String>,
    pub suggested_primary_category: Option<PrimaryCategory>,
    pub suggested_genre_facets: Vec<GenreFacet>,
    pub suggested_filename: Option<String>,
    pub suggested_target_path: Option<PathBuf>,
    pub linked_candidate_id: Option<Uuid>,
    pub linked_work_id: Option<Uuid>,
    pub import_status: ImportStatus,
    pub conflict_warnings: Vec<String>,
}
