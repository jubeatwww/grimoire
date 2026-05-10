use crate::category::{GenreFacet, PrimaryCategory};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameWork {
    pub id: Uuid,
    pub canonical_title: String,
    pub original_title: Option<String>,
    pub display_title: String,
    pub circle: Option<String>,
    pub developer: Option<String>,
    pub publisher: Option<String>,
    pub source_urls: Vec<String>,
    pub dlsite_work_id: Option<String>,
    pub description: Option<String>,
    pub release_date: Option<NaiveDate>,
    pub source_tags: Vec<String>,
    pub primary_category: Option<PrimaryCategory>,
    pub genre_facets: Vec<GenreFacet>,
    pub cover_asset_id: Option<Uuid>,
    pub preview_asset_ids: Vec<Uuid>,
    pub series: Option<String>,
}
