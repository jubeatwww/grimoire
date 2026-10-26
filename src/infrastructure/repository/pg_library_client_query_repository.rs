use async_trait::async_trait;
use chrono::{DateTime, NaiveDate, Utc};
use sqlx::PgPool;
use sqlx::types::Json;
use uuid::Uuid;

use crate::application::library::client::{
    InventoryItemListItemOutput, LibraryClientQueryRepository, LibraryClientQueryRepositoryError,
    SourceTagOutput,
};

pub(crate) struct PgLibraryClientQueryRepository {
    pool: PgPool,
}

impl PgLibraryClientQueryRepository {
    pub(crate) fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

/// One element of the aggregated `source_tags` JSON.
#[derive(serde::Deserialize)]
struct SourceTagRow {
    label: String,
    source: String,
}

impl From<SourceTagRow> for SourceTagOutput {
    fn from(row: SourceTagRow) -> Self {
        Self {
            label: row.label,
            source: row.source,
        }
    }
}

#[derive(sqlx::FromRow)]
struct LibraryItemRow {
    id: Uuid,
    source_id: String,
    path: String,
    file_name: String,
    legacy_location: Option<String>,
    game_work_id: Option<Uuid>,
    primary_category: Option<String>,
    category_confirmed: Option<bool>,
    genre_facets: Option<Json<Vec<String>>>,
    organization_status: String,
    play_status: String,
    rating: Option<i16>,
    version: Option<String>,
    language: Option<String>,
    notes: Option<String>,
    missing: bool,
    display_title: Option<String>,
    cover_image_url: Option<String>,
    circle: Option<String>,
    description: Option<String>,
    release_date: Option<NaiveDate>,
    series: Option<String>,
    preview_image_urls: Option<Json<Vec<String>>>,
    source_tags: Option<Json<Vec<SourceTagRow>>>,
    file_type: Option<String>,
    file_size_bytes: Option<i64>,
    dl_count: Option<i32>,
    rate_average: Option<f32>,
    rate_count: Option<i32>,
    price_jpy: Option<i32>,
    work_type: Option<String>,
    work_type_label: Option<String>,
    dlsite_work_id: Option<String>,
    vndb_id: Option<String>,
    steam_app_id: Option<String>,
    enriched_at: Option<DateTime<Utc>>,
}

impl From<LibraryItemRow> for InventoryItemListItemOutput {
    fn from(row: LibraryItemRow) -> Self {
        Self {
            id: row.id,
            source_id: row.source_id,
            path: row.path,
            file_name: row.file_name,
            legacy_location: row.legacy_location,
            game_work_id: row.game_work_id,
            primary_category: row.primary_category,
            category_confirmed: row.category_confirmed.unwrap_or(false),
            genre_facets: row.genre_facets.map(|j| j.0).unwrap_or_default(),
            organization_status: row.organization_status,
            play_status: row.play_status,
            rating: row.rating,
            version: row.version,
            language: row.language,
            notes: row.notes,
            missing: row.missing,
            display_title: row.display_title,
            cover_image_url: row.cover_image_url,
            circle: row.circle,
            description: row.description,
            release_date: row.release_date,
            series: row.series,
            source_tags: row
                .source_tags
                .map(|j| j.0.into_iter().map(SourceTagOutput::from).collect())
                .unwrap_or_default(),
            preview_image_urls: row.preview_image_urls.map(|j| j.0).unwrap_or_default(),
            file_type: row.file_type,
            file_size_bytes: row.file_size_bytes,
            dl_count: row.dl_count,
            rate_average: row.rate_average,
            rate_count: row.rate_count,
            price_jpy: row.price_jpy,
            work_type: row.work_type,
            work_type_label: row.work_type_label,
            dlsite_work_id: row.dlsite_work_id,
            vndb_id: row.vndb_id,
            steam_app_id: row.steam_app_id,
            enriched_at: row.enriched_at,
        }
    }
}

#[async_trait]
impl LibraryClientQueryRepository for PgLibraryClientQueryRepository {
    async fn list_items(
        &self,
        limit: i64,
    ) -> Result<Vec<InventoryItemListItemOutput>, LibraryClientQueryRepositoryError> {
        let rows: Vec<LibraryItemRow> = sqlx::query_as(
            "SELECT i.id, i.source_id, i.path, i.file_name, i.legacy_location, i.game_work_id,
                    g.primary_category, g.category_confirmed_at IS NOT NULL AS category_confirmed,
                    g.genre_facets, i.organization_status, i.play_status, i.rating,
                    i.version, i.language, i.notes, i.missing,
                    g.display_title, g.cover_image_url, g.circle, g.description,
                    g.release_date, g.series, g.preview_image_urls,
                    (SELECT jsonb_agg(jsonb_build_object('label', t.label, 'source', s.source)
                                      ORDER BY s.source, t.ord)
                       FROM work_source_data s,
                            jsonb_array_elements_text(s.tags) WITH ORDINALITY AS t(label, ord)
                      WHERE s.game_work_id = g.id) AS source_tags,
                    g.file_type, g.file_size_bytes, g.dl_count, g.rate_average,
                    g.rate_count, g.price_jpy, g.work_type, g.work_type_label,
                    g.dlsite_work_id, g.vndb_id, g.steam_app_id, g.enriched_at
               FROM inventory_items i
               LEFT JOIN game_works g ON g.id = i.game_work_id
              ORDER BY i.file_name
              LIMIT $1",
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| LibraryClientQueryRepositoryError::Backend(Box::new(e)))?;
        Ok(rows
            .into_iter()
            .map(InventoryItemListItemOutput::from)
            .collect())
    }
}
