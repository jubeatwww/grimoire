use crate::state::AppState;
use axum::{extract::State, http::StatusCode, routing::post, Json, Router};
use grimoire_app::dlsite::DlsiteSource;
use grimoire_app::metadata_source::MetadataSource;
use serde::{Deserialize, Serialize};
use sqlx::Row;
use uuid::Uuid;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/search", post(search))
        .route("/confirm", post(confirm))
        .route("/skip", post(skip))
        .route("/refresh", post(refresh))
        .route("/link", post(link))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ItemIdRequest {
    inventory_item_id: Uuid,
}

async fn skip(
    State(state): State<AppState>,
    Json(body): Json<ItemIdRequest>,
) -> Result<StatusCode, StatusCode> {
    sqlx::query(
        "UPDATE inventory_items
            SET organization_status = 'no_match', updated_at = now()
          WHERE id = $1",
    )
    .bind(body.inventory_item_id)
    .execute(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct LinkRequest {
    inventory_item_id: Uuid,
    workno_or_url: String,
}

async fn link(
    State(state): State<AppState>,
    Json(body): Json<LinkRequest>,
) -> Result<Json<ConfirmResponse>, StatusCode> {
    let workno = DlsiteSource::extract_work_id(&body.workno_or_url)
        .ok_or(StatusCode::BAD_REQUEST)?;

    let detail = match DlsiteSource::new().fetch_product_detail(&workno).await {
        Ok(Some(d)) => d,
        Ok(None) => return Err(StatusCode::NOT_FOUND),
        Err(_) => return Err(StatusCode::BAD_GATEWAY),
    };

    let source_url = format!(
        "https://www.dlsite.com/maniax/work/=/product_id/{}.html",
        workno
    );
    let title = detail
        .work_name
        .clone()
        .unwrap_or_else(|| workno.clone());

    // Find or create game_work
    let existing = sqlx::query("SELECT id FROM game_works WHERE dlsite_work_id = $1")
        .bind(&workno)
        .fetch_optional(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let game_work_id = if let Some(row) = existing {
        row.get::<Uuid, _>("id")
    } else {
        let id = Uuid::new_v4();
        let source_urls = serde_json::json!([source_url]);
        let genre_facets = serde_json::json!([]);
        sqlx::query(
            "INSERT INTO game_works (
                id, canonical_title, display_title, circle, source_urls,
                dlsite_work_id, genre_facets
            ) VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(id)
        .bind(&title)
        .bind(&title)
        .bind(&detail.maker_name)
        .bind(&source_urls)
        .bind(&workno)
        .bind(&genre_facets)
        .execute(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        id
    };

    // Enrich with full detail (overwrite — manual link is the authoritative action).
    let tags_json = serde_json::json!(detail.tags);
    let previews_json = serde_json::json!(detail.preview_image_urls);
    sqlx::query(
        "UPDATE game_works SET
            description        = $1,
            release_date       = COALESCE($2, release_date),
            series             = $3,
            source_tags        = $4,
            cover_image_url    = COALESCE($5, cover_image_url),
            preview_image_urls = $6,
            file_type          = $7,
            file_size_bytes    = $8,
            dl_count           = $9,
            rate_average       = $10,
            rate_count         = $11,
            price_jpy          = $12,
            work_type          = $13,
            work_type_label    = $14,
            updated_at         = now()
         WHERE id = $15",
    )
    .bind(detail.description)
    .bind(detail.release_date)
    .bind(detail.series)
    .bind(tags_json)
    .bind(detail.cover_image_url)
    .bind(previews_json)
    .bind(detail.file_type)
    .bind(detail.file_size_bytes)
    .bind(detail.dl_count)
    .bind(detail.rate_average)
    .bind(detail.rate_count)
    .bind(detail.price_jpy)
    .bind(detail.work_type)
    .bind(detail.work_type_label)
    .bind(game_work_id)
    .execute(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Link inventory item
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

async fn refresh(
    State(state): State<AppState>,
    Json(body): Json<ItemIdRequest>,
) -> Result<StatusCode, StatusCode> {
    let row = sqlx::query(
        "SELECT g.id AS game_work_id, g.dlsite_work_id
           FROM inventory_items i
           JOIN game_works g ON g.id = i.game_work_id
          WHERE i.id = $1",
    )
    .bind(body.inventory_item_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let Some(row) = row else {
        return Err(StatusCode::NOT_FOUND);
    };
    let game_work_id: Uuid = row.get("game_work_id");
    let workno: Option<String> = row.get("dlsite_work_id");
    let Some(workno) = workno else {
        return Err(StatusCode::BAD_REQUEST);
    };

    let detail = match DlsiteSource::new().fetch_product_detail(&workno).await {
        Ok(Some(d)) => d,
        Ok(None) => return Err(StatusCode::NOT_FOUND),
        Err(_) => return Err(StatusCode::BAD_GATEWAY),
    };

    let tags_json = serde_json::json!(detail.tags);
    let previews_json = serde_json::json!(detail.preview_image_urls);
    sqlx::query(
        "UPDATE game_works SET
            description        = $1,
            release_date       = COALESCE($2, release_date),
            series             = $3,
            source_tags        = $4,
            cover_image_url    = COALESCE($5, cover_image_url),
            preview_image_urls = $6,
            file_type          = $7,
            file_size_bytes    = $8,
            dl_count           = $9,
            rate_average       = $10,
            rate_count         = $11,
            price_jpy          = $12,
            work_type          = $13,
            work_type_label    = $14,
            updated_at         = now()
         WHERE id = $15",
    )
    .bind(detail.description)
    .bind(detail.release_date)
    .bind(detail.series)
    .bind(tags_json)
    .bind(detail.cover_image_url)
    .bind(previews_json)
    .bind(detail.file_type)
    .bind(detail.file_size_bytes)
    .bind(detail.dl_count)
    .bind(detail.rate_average)
    .bind(detail.rate_count)
    .bind(detail.price_jpy)
    .bind(detail.work_type)
    .bind(detail.work_type_label)
    .bind(game_work_id)
    .execute(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(StatusCode::NO_CONTENT)
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
    work_type: Option<String>,
    intro_short: Option<String>,
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
        .map(|c| {
            let work_type = c
                .normalized_payload
                .get("work_type")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let intro_short = c
                .normalized_payload
                .get("intro_s")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            CandidateResponse {
                id: c.id.to_string(),
                source_name: c.source_name,
                source_work_id: c.source_work_id,
                source_url: c.source_url,
                rank: c.rank,
                title: c.title,
                circle: c.circle,
                cover_url: c.cover_url,
                work_type,
                intro_short,
            }
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
    let cover_url: Option<String> = row.get("cover_url");
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
        let id = existing_row.get::<Uuid, _>("id");
        if cover_url.is_some() {
            sqlx::query(
                "UPDATE game_works
                    SET cover_image_url = COALESCE(cover_image_url, $1),
                        updated_at = now()
                  WHERE id = $2",
            )
            .bind(&cover_url)
            .bind(id)
            .execute(&state.db)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        }
        id
    } else {
        let id = Uuid::new_v4();
        let source_urls = serde_json::json!([source_url]);
        let source_tags = serde_json::json!(genres);
        let genre_facets = serde_json::json!([]);

        sqlx::query(
            "INSERT INTO game_works (
                id, canonical_title, display_title, circle, source_urls,
                dlsite_work_id, release_date, source_tags, genre_facets, cover_image_url
            ) VALUES ($1, $2, $3, $4, $5, $6, $7::date, $8, $9, $10)",
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
        .bind(&cover_url)
        .execute(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        id
    };

    // Best-effort enrichment from DLsite product.json. Failures don't block confirmation —
    // user-edited fields (primary_category, genre_facets, display_title) are preserved via COALESCE.
    if let Ok(Some(detail)) = DlsiteSource::new().fetch_product_detail(&source_work_id).await {
        let tags_json = serde_json::json!(detail.tags);
        let previews_json = serde_json::json!(detail.preview_image_urls);
        sqlx::query(
            "UPDATE game_works SET
                description       = COALESCE($1, description),
                release_date      = COALESCE($2, release_date),
                series            = COALESCE($3, series),
                source_tags       = $4,
                cover_image_url   = COALESCE($5, cover_image_url),
                preview_image_urls = $6,
                file_type         = COALESCE($7, file_type),
                file_size_bytes   = COALESCE($8, file_size_bytes),
                dl_count          = COALESCE($9, dl_count),
                rate_average      = COALESCE($10, rate_average),
                rate_count        = COALESCE($11, rate_count),
                price_jpy         = COALESCE($12, price_jpy),
                work_type         = COALESCE($13, work_type),
                work_type_label   = COALESCE($14, work_type_label),
                updated_at        = now()
             WHERE id = $15",
        )
        .bind(detail.description)
        .bind(detail.release_date)
        .bind(detail.series)
        .bind(tags_json)
        .bind(detail.cover_image_url)
        .bind(previews_json)
        .bind(detail.file_type)
        .bind(detail.file_size_bytes)
        .bind(detail.dl_count)
        .bind(detail.rate_average)
        .bind(detail.rate_count)
        .bind(detail.price_jpy)
        .bind(detail.work_type)
        .bind(detail.work_type_label)
        .bind(game_work_id)
        .execute(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }

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
