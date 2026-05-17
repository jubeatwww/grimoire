use crate::state::AppState;
use axum::{extract::State, http::StatusCode, Json};
use grimoire_app::scanner::{ScanOptions, ScanResult};
use serde::Serialize;

#[derive(Serialize)]
pub(crate) struct ScanResponse {
    scanned: usize,
    warnings: Vec<String>,
}

pub(crate) async fn scan(State(state): State<AppState>) -> Result<Json<ScanResponse>, StatusCode> {
    let options = ScanOptions {
        source_id: "local".to_string(),
        root: state.library_root.root().to_path_buf(),
    };

    let ScanResult { items, warnings } =
        grimoire_app::scanner::scan_library(options)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    for item in &items {
        let path_str = item.path.to_string_lossy();
        let kind = serde_json::to_value(&item.kind)
            .unwrap_or_default()
            .as_str()
            .unwrap_or("file")
            .to_string();
        let org_status = serde_json::to_value(&item.organization_status)
            .unwrap_or_default()
            .as_str()
            .unwrap_or("pending")
            .to_string();
        let play_status = serde_json::to_value(&item.play_status)
            .unwrap_or_default()
            .as_str()
            .unwrap_or("not_played")
            .to_string();
        let genre_facets = serde_json::to_value(&item.genre_facets).unwrap_or_default();
        let personal_tags = serde_json::to_value(&item.personal_tags).unwrap_or_default();

        sqlx::query(
            "INSERT INTO inventory_items (
                id, source_id, path, file_name, extension, kind, file_size,
                modified_at, content_hash, legacy_location, primary_category,
                genre_facets, version, language, extracted, downloaded,
                organization_status, play_status, rating, personal_tags, notes
            ) VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14,
                $15, $16, $17, $18, $19, $20, $21
            )
            ON CONFLICT (source_id, path) DO UPDATE SET
                file_name = EXCLUDED.file_name,
                extension = EXCLUDED.extension,
                kind = EXCLUDED.kind,
                file_size = EXCLUDED.file_size,
                modified_at = EXCLUDED.modified_at,
                updated_at = now()",
        )
        .bind(item.id)
        .bind(&item.source_id)
        .bind(path_str.as_ref())
        .bind(&item.file_name)
        .bind(&item.extension)
        .bind(&kind)
        .bind(item.file_size as i64)
        .bind(item.modified_at)
        .bind(&item.content_hash)
        .bind(&item.legacy_location)
        .bind(item.primary_category.as_ref().map(|c| c.as_str()))
        .bind(&genre_facets)
        .bind(&item.version)
        .bind(&item.language)
        .bind(item.extracted)
        .bind(item.downloaded)
        .bind(&org_status)
        .bind(&play_status)
        .bind(item.rating.map(|r| r as i16))
        .bind(&personal_tags)
        .bind(&item.notes)
        .execute(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }

    let scanned = items.len();
    Ok(Json(ScanResponse { scanned, warnings }))
}
