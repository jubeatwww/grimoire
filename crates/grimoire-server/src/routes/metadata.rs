use crate::state::AppState;
use axum::{
    extract::State,
    http::StatusCode,
    routing::post,
    Json, Router,
};
use grimoire_app::dlsite::DlsiteSource;
use grimoire_app::metadata_source::MetadataSource;
use serde::{Deserialize, Serialize};
use sqlx::Row;
use uuid::Uuid;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/search", post(search))
        .route("/confirm", post(confirm))
}

#[derive(Deserialize)]
struct SearchRequest {
    query: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CandidateResponse {
    id: String,
    source_name: String,
    source_work_id: String,
    source_url: String,
    rank: i32,
    title: String,
    circle: Option<String>,
    cover_url: Option<String>,
}

#[derive(Serialize)]
struct SearchResponse {
    candidates: Vec<CandidateResponse>,
}

async fn search(
    State(state): State<AppState>,
    Json(body): Json<SearchRequest>,
) -> Result<Json<SearchResponse>, StatusCode> {
    let source = DlsiteSource::new();
    let candidates = source
        .search(&body.query)
        .await
        .map_err(|_| StatusCode::BAD_GATEWAY)?;

    // Persist candidates to DB
    for c in &candidates {
        sqlx::query(
            "INSERT INTO metadata_candidates (
                id, source_name, source_work_id, source_url, query_used,
                rank, title, circle, cover_url, normalized_payload
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            ON CONFLICT DO NOTHING",
        )
        .bind(c.id)
        .bind(&c.source_name)
        .bind(&c.source_work_id)
        .bind(&c.source_url)
        .bind(&c.query_used)
        .bind(c.rank)
        .bind(&c.title)
        .bind(&c.circle)
        .bind(&c.cover_url)
        .bind(&c.normalized_payload)
        .execute(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }

    let response = candidates
        .into_iter()
        .map(|c| CandidateResponse {
            id: c.id.to_string(),
            source_name: c.source_name,
            source_work_id: c.source_work_id,
            source_url: c.source_url,
            rank: c.rank,
            title: c.title,
            circle: c.circle,
            cover_url: c.cover_url,
        })
        .collect();

    Ok(Json(SearchResponse {
        candidates: response,
    }))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ConfirmRequest {
    candidate_id: Uuid,
    inventory_item_id: Uuid,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ConfirmResponse {
    game_work_id: String,
}

async fn confirm(
    State(state): State<AppState>,
    Json(body): Json<ConfirmRequest>,
) -> Result<Json<ConfirmResponse>, StatusCode> {
    // Fetch the candidate
    let row = sqlx::query(
        "SELECT source_name, source_work_id, source_url, title, circle, cover_url, normalized_payload
         FROM metadata_candidates WHERE id = $1",
    )
    .bind(body.candidate_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let Some(row) = row else {
        return Err(StatusCode::NOT_FOUND);
    };

    let title: String = row.get("title");
    let circle: Option<String> = row.get("circle");
    let source_work_id: String = row.get("source_work_id");
    let source_url: String = row.get("source_url");
    let payload: serde_json::Value = row.get("normalized_payload");

    let genres: Vec<String> = payload
        .get("genres")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default();
    let release_date: Option<String> = payload
        .get("release_date")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    // Create or find existing game_work by dlsite_work_id
    let existing = sqlx::query("SELECT id FROM game_works WHERE dlsite_work_id = $1")
        .bind(&source_work_id)
        .fetch_optional(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let game_work_id = if let Some(existing_row) = existing {
        existing_row.get::<Uuid, _>("id")
    } else {
        let id = Uuid::new_v4();
        let source_urls = serde_json::json!([source_url]);
        let source_tags = serde_json::json!(genres);
        let genre_facets = serde_json::json!([]);

        sqlx::query(
            "INSERT INTO game_works (
                id, canonical_title, display_title, circle, source_urls,
                dlsite_work_id, release_date, source_tags, genre_facets
            ) VALUES ($1, $2, $3, $4, $5, $6, $7::date, $8, $9)",
        )
        .bind(id)
        .bind(&title)
        .bind(&title)
        .bind(&circle)
        .bind(&source_urls)
        .bind(&source_work_id)
        .bind(&release_date)
        .bind(&source_tags)
        .bind(&genre_facets)
        .execute(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        id
    };

    // Link inventory item to game_work
    sqlx::query(
        "UPDATE inventory_items SET game_work_id = $1, organization_status = 'confirmed', updated_at = now()
         WHERE id = $2",
    )
    .bind(game_work_id)
    .bind(body.inventory_item_id)
    .execute(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(ConfirmResponse {
        game_work_id: game_work_id.to_string(),
    }))
}
