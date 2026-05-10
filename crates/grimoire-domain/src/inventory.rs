use crate::category::{GenreFacet, PrimaryCategory};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrganizationStatus {
    Pending,
    Matched,
    Confirmed,
    Ignored,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlayStatus {
    NotPlayed,
    WantToPlay,
    Playing,
    Completed,
    Dropped,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum InventoryKind {
    File,
    Directory,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InventoryItem {
    pub id: Uuid,
    pub source_id: String,
    pub path: PathBuf,
    pub file_name: String,
    pub extension: Option<String>,
    pub kind: InventoryKind,
    pub file_size: u64,
    pub modified_at: DateTime<Utc>,
    pub content_hash: Option<String>,
    pub legacy_location: Option<String>,
    pub primary_category: Option<PrimaryCategory>,
    pub genre_facets: Vec<GenreFacet>,
    pub game_work_id: Option<Uuid>,
    pub version: Option<String>,
    pub language: Option<String>,
    pub patch_location: Option<String>,
    pub save_location: Option<String>,
    pub extracted: bool,
    pub downloaded: bool,
    pub organization_status: OrganizationStatus,
    pub play_status: PlayStatus,
    pub rating: Option<u8>,
    pub personal_tags: Vec<String>,
    pub notes: Option<String>,
}
