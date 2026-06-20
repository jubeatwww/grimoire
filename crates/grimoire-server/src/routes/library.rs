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
                g.display_title, g.cover_image_url
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
            }
        })
        .collect();

    Ok(Json(LibraryResponse { items }))
}
