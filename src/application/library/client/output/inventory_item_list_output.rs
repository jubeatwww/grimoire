use chrono::{DateTime, NaiveDate, Utc};
use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Serialize)]
pub(crate) struct ListInventoryItemsOutput {
    pub(crate) items: Vec<InventoryItemListItemOutput>,
}

/// One file with its work, as the library list shows it.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct InventoryItemListItemOutput {
    pub(crate) id: Uuid,
    pub(crate) source_id: String,
    pub(crate) path: String,
    pub(crate) file_name: String,
    pub(crate) legacy_location: Option<String>,
    pub(crate) game_work_id: Option<Uuid>,
    /// Work-level category id (see `categories`); files inherit it.
    pub(crate) primary_category: Option<String>,
    pub(crate) category_confirmed: bool,
    /// Work-level genre codes.
    pub(crate) genre_facets: Vec<String>,
    pub(crate) organization_status: String,
    pub(crate) play_status: String,
    pub(crate) rating: Option<i16>,
    pub(crate) version: Option<String>,
    pub(crate) language: Option<String>,
    pub(crate) notes: Option<String>,
    pub(crate) missing: bool,
    pub(crate) display_title: Option<String>,
    pub(crate) cover_image_url: Option<String>,
    pub(crate) circle: Option<String>,
    pub(crate) description: Option<String>,
    pub(crate) release_date: Option<NaiveDate>,
    pub(crate) series: Option<String>,
    pub(crate) source_tags: Vec<SourceTagOutput>,
    pub(crate) preview_image_urls: Vec<String>,
    pub(crate) file_type: Option<String>,
    pub(crate) file_size_bytes: Option<i64>,
    pub(crate) dl_count: Option<i32>,
    pub(crate) rate_average: Option<f32>,
    pub(crate) rate_count: Option<i32>,
    pub(crate) price_jpy: Option<i32>,
    pub(crate) work_type: Option<String>,
    pub(crate) work_type_label: Option<String>,
    pub(crate) dlsite_work_id: Option<String>,
    pub(crate) vndb_id: Option<String>,
    pub(crate) steam_app_id: Option<String>,
    pub(crate) enriched_at: Option<DateTime<Utc>>,
}

/// A source's tag with the site it came from.
#[derive(Debug, Serialize)]
pub(crate) struct SourceTagOutput {
    pub(crate) label: String,
    pub(crate) source: String,
}
