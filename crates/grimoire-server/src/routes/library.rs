use crate::state::AppState;
use axum::{
    extract::State,
    http::StatusCode,
    routing::{get, MethodRouter},
    Json,
};
use serde::Serialize;
use sqlx::Row;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct LibraryItem {
    id: String,
    source_id: String,
    file_name: String,
    legacy_location: Option<String>,
    primary_category: Option<String>,
    genre_facets: Vec<String>,
    organization_status: String,
    play_status: String,
    rating: Option<i16>,
    version: Option<String>,
    language: Option<String>,
    notes: Option<String>,
    display_title: Option<String>,
    cover_image_url: Option<String>,
    circle: Option<String>,
    description: Option<String>,
    release_date: Option<chrono::NaiveDate>,
    series: Option<String>,
    source_tags: Vec<String>,
    preview_image_urls: Vec<String>,
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
    enriched_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Serialize)]
struct LibraryResponse {
    items: Vec<LibraryItem>,
}

pub(crate) fn list_route() -> MethodRouter<AppState> {
    get(list)
}

async fn list(State(state): State<AppState>) -> Result<Json<LibraryResponse>, StatusCode> {
    let rows = sqlx::query(
        "SELECT i.id, i.source_id, i.file_name, i.legacy_location, i.primary_category,
                i.genre_facets, i.organization_status, i.play_status, i.rating,
                i.version, i.language, i.notes,
                g.display_title, g.cover_image_url, g.circle, g.description,
                g.release_date, g.series, g.source_tags, g.preview_image_urls,
                g.file_type, g.file_size_bytes, g.dl_count, g.rate_average,
                g.rate_count, g.price_jpy, g.work_type, g.work_type_label,
                g.dlsite_work_id, g.vndb_id, g.enriched_at
         FROM inventory_items i
         LEFT JOIN game_works g ON g.id = i.game_work_id
         ORDER BY i.file_name",
    )
    .fetch_all(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let items = rows
        .iter()
        .map(|row| {
            let genre_facets: serde_json::Value = row.get("genre_facets");
            let source_tags: Option<serde_json::Value> = row.get("source_tags");
            let preview_urls: Option<serde_json::Value> = row.get("preview_image_urls");
            LibraryItem {
                id: row.get::<uuid::Uuid, _>("id").to_string(),
                source_id: row.get("source_id"),
                file_name: row.get("file_name"),
                legacy_location: row.get("legacy_location"),
                primary_category: row.get("primary_category"),
                genre_facets: serde_json::from_value(genre_facets).unwrap_or_default(),
                organization_status: row.get("organization_status"),
                play_status: row.get("play_status"),
                rating: row.get("rating"),
                version: row.get("version"),
                language: row.get("language"),
                notes: row.get("notes"),
                display_title: row.get("display_title"),
                cover_image_url: row.get("cover_image_url"),
                circle: row.get("circle"),
                description: row.get("description"),
                release_date: row.get("release_date"),
                series: row.get("series"),
                source_tags: source_tags
                    .and_then(|v| serde_json::from_value(v).ok())
                    .unwrap_or_default(),
                preview_image_urls: preview_urls
                    .and_then(|v| serde_json::from_value(v).ok())
                    .unwrap_or_default(),
                file_type: row.get("file_type"),
                file_size_bytes: row.get("file_size_bytes"),
                dl_count: row.get("dl_count"),
                rate_average: row.get("rate_average"),
                rate_count: row.get("rate_count"),
                price_jpy: row.get("price_jpy"),
                work_type: row.get("work_type"),
                work_type_label: row.get("work_type_label"),
                dlsite_work_id: row.get("dlsite_work_id"),
                vndb_id: row.get("vndb_id"),
                enriched_at: row.get("enriched_at"),
            }
        })
        .collect();

    Ok(Json(LibraryResponse { items }))
}
