use crate::state::AppState;
use axum::{extract::State, http::StatusCode, routing::post, Json, Router};
use grimoire_app::{
    dlsite::DlsiteSource,
    metadata_source::{MetadataSource, ProductDetail},
    vndb::VndbSource,
};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row};
use uuid::Uuid;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/search", post(search))
        .route("/confirm", post(confirm))
        .route("/skip", post(skip))
        .route("/refresh", post(refresh))
        .route("/link", post(link))
        .route("/reset", post(reset))
        .route("/edit-work", post(edit_work))
        .route("/edit-item", post(edit_item))
}

// ---- source dispatch ------------------------------------------------------

#[derive(Debug, Clone, Copy)]
enum Src {
    Dlsite,
    Vndb,
}

impl Src {
    fn from_name(s: &str) -> Option<Self> {
        match s {
            "dlsite" => Some(Self::Dlsite),
            "vndb" => Some(Self::Vndb),
            _ => None,
        }
    }

    async fn fetch_detail(&self, id: &str) -> anyhow::Result<Option<ProductDetail>> {
        match self {
            Self::Dlsite => DlsiteSource::new().fetch_product_detail(id).await,
            Self::Vndb => VndbSource::new().fetch_product_detail(id).await,
        }
    }
}

fn source_url_for(src: Src, id: &str) -> String {
    match src {
        Src::Dlsite => format!("https://www.dlsite.com/maniax/work/=/product_id/{id}.html"),
        Src::Vndb => format!("https://vndb.org/{id}"),
    }
}

/// Auto-detect which source a raw user-pasted input belongs to. Prefer VNDB
/// (more specific) when the input looks like a vN code or vndb.org URL so a
/// stray "v123" doesn't accidentally route to DLsite's id parser.
fn detect_link_source(input: &str) -> Option<(Src, String)> {
    if let Some(id) = VndbSource::extract_id(input) {
        return Some((Src::Vndb, id));
    }
    if let Some(id) = DlsiteSource::extract_work_id(input) {
        return Some((Src::Dlsite, id));
    }
    None
}

// ---- DB helpers -----------------------------------------------------------

async fn find_game_work_by_external_id(
    db: &PgPool,
    src: Src,
    id: &str,
) -> Result<Option<Uuid>, StatusCode> {
    let sql = match src {
        Src::Dlsite => "SELECT id FROM game_works WHERE dlsite_work_id = $1",
        Src::Vndb => "SELECT id FROM game_works WHERE vndb_id = $1",
    };
    let row = sqlx::query(sql)
        .bind(id)
        .fetch_optional(db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(row.map(|r| r.get("id")))
}

async fn insert_game_work(
    db: &PgPool,
    src: Src,
    external_id: &str,
    source_url: &str,
    title: &str,
    circle: &Option<String>,
) -> Result<Uuid, StatusCode> {
    let new_id = Uuid::new_v4();
    let source_urls = serde_json::json!([source_url]);
    let genre_facets = serde_json::json!([]);
    let sql = match src {
        Src::Dlsite => {
            "INSERT INTO game_works (
                id, canonical_title, display_title, circle, source_urls,
                dlsite_work_id, genre_facets
            ) VALUES ($1, $2, $3, $4, $5, $6, $7)"
        }
        Src::Vndb => {
            "INSERT INTO game_works (
                id, canonical_title, display_title, circle, source_urls,
                vndb_id, genre_facets
            ) VALUES ($1, $2, $3, $4, $5, $6, $7)"
        }
    };
    sqlx::query(sql)
        .bind(new_id)
        .bind(title)
        .bind(title)
        .bind(circle)
        .bind(&source_urls)
        .bind(external_id)
        .bind(&genre_facets)
        .execute(db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(new_id)
}

/// Write detail fields onto an existing game_work. `overwrite=false` (used by
/// confirm) preserves any prior non-null values via COALESCE so a refresh
/// from one source doesn't blank fields the other source filled in.
/// `overwrite=true` (refresh / link) is the explicit "use this source's data"
/// path — the latest writer wins.
async fn apply_detail(
    db: &PgPool,
    game_work_id: Uuid,
    detail: ProductDetail,
    overwrite: bool,
) -> Result<(), StatusCode> {
    let tags_json = serde_json::json!(detail.tags);
    let previews_json = serde_json::json!(detail.preview_image_urls);
    let sql = if overwrite {
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
         WHERE id = $15"
    } else {
        "UPDATE game_works SET
            description        = COALESCE($1, description),
            release_date       = COALESCE($2, release_date),
            series             = COALESCE($3, series),
            source_tags        = $4,
            cover_image_url    = COALESCE($5, cover_image_url),
            preview_image_urls = $6,
            file_type          = COALESCE($7, file_type),
            file_size_bytes    = COALESCE($8, file_size_bytes),
            dl_count           = COALESCE($9, dl_count),
            rate_average       = COALESCE($10, rate_average),
            rate_count         = COALESCE($11, rate_count),
            price_jpy          = COALESCE($12, price_jpy),
            work_type          = COALESCE($13, work_type),
            work_type_label    = COALESCE($14, work_type_label),
            updated_at         = now()
         WHERE id = $15"
    };
    sqlx::query(sql)
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
        .execute(db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(())
}

async fn link_inventory_to_work(
    db: &PgPool,
    item_id: Uuid,
    game_work_id: Uuid,
) -> Result<(), StatusCode> {
    sqlx::query(
        "UPDATE inventory_items
            SET game_work_id = $1, organization_status = 'confirmed', updated_at = now()
          WHERE id = $2",
    )
    .bind(game_work_id)
    .bind(item_id)
    .execute(db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(())
}

// ---- shared request/response types ---------------------------------------

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ItemIdRequest {
    inventory_item_id: Uuid,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ConfirmResponse {
    game_work_id: String,
}

// ---- skip -----------------------------------------------------------------

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

async fn reset(
    State(state): State<AppState>,
    Json(body): Json<ItemIdRequest>,
) -> Result<StatusCode, StatusCode> {
    sqlx::query(
        "UPDATE inventory_items
            SET game_work_id = NULL,
                organization_status = 'pending',
                updated_at = now()
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
struct EditWorkRequest {
    inventory_item_id: Uuid,
    display_title: Option<String>,
    work_type: Option<String>,
    work_type_label: Option<String>,
}

async fn edit_work(
    State(state): State<AppState>,
    Json(body): Json<EditWorkRequest>,
) -> Result<StatusCode, StatusCode> {
    let row = sqlx::query(
        "SELECT game_work_id FROM inventory_items WHERE id = $1",
    )
    .bind(body.inventory_item_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let Some(row) = row else {
        return Err(StatusCode::NOT_FOUND);
    };
    let game_work_id: Option<Uuid> = row.get("game_work_id");
    let Some(game_work_id) = game_work_id else {
        return Err(StatusCode::BAD_REQUEST);
    };

    // display_title is NOT NULL — keep prior value when blank/missing.
    // work_type / work_type_label: empty string explicitly clears (lets the user
    // strip an over-eager source-derived value); missing leaves it alone.
    sqlx::query(
        "UPDATE game_works SET
            display_title    = COALESCE(NULLIF($1, ''), display_title),
            work_type        = CASE WHEN $2::text IS NULL THEN work_type ELSE NULLIF($2, '') END,
            work_type_label  = CASE WHEN $3::text IS NULL THEN work_type_label ELSE NULLIF($3, '') END,
            updated_at       = now()
         WHERE id = $4",
    )
    .bind(body.display_title)
    .bind(body.work_type)
    .bind(body.work_type_label)
    .bind(game_work_id)
    .execute(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct EditItemRequest {
    inventory_item_id: Uuid,
    primary_category: Option<String>,
}

async fn edit_item(
    State(state): State<AppState>,
    Json(body): Json<EditItemRequest>,
) -> Result<StatusCode, StatusCode> {
    sqlx::query(
        "UPDATE inventory_items SET
            primary_category = CASE WHEN $1::text IS NULL THEN primary_category ELSE NULLIF($1, '') END,
            updated_at       = now()
          WHERE id = $2",
    )
    .bind(body.primary_category)
    .bind(body.inventory_item_id)
    .execute(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(StatusCode::NO_CONTENT)
}

// ---- search ---------------------------------------------------------------

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
    let dl = DlsiteSource::new();
    let vn = VndbSource::new();
    let (dl_res, vn_res) = tokio::join!(dl.search(&body.query), vn.search(&body.query));

    let mut candidates: Vec<grimoire_domain::metadata::MetadataCandidate> = Vec::new();
    if let Ok(v) = dl_res {
        candidates.extend(v);
    }
    if let Ok(v) = vn_res {
        candidates.extend(v);
    }

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

// ---- confirm --------------------------------------------------------------

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ConfirmRequest {
    candidate_id: Uuid,
    inventory_item_id: Uuid,
}

async fn confirm(
    State(state): State<AppState>,
    Json(body): Json<ConfirmRequest>,
) -> Result<Json<ConfirmResponse>, StatusCode> {
    let row = sqlx::query(
        "SELECT source_name, source_work_id, source_url, title, circle, cover_url
           FROM metadata_candidates
          WHERE id = $1",
    )
    .bind(body.candidate_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let Some(row) = row else {
        return Err(StatusCode::NOT_FOUND);
    };

    let source_name: String = row.get("source_name");
    let src = Src::from_name(&source_name).ok_or(StatusCode::BAD_REQUEST)?;
    let title: String = row.get("title");
    let circle: Option<String> = row.get("circle");
    let source_work_id: String = row.get("source_work_id");
    let source_url: String = row.get("source_url");
    let cover_url: Option<String> = row.get("cover_url");

    let game_work_id = match find_game_work_by_external_id(&state.db, src, &source_work_id).await? {
        Some(id) => {
            // Pre-fill cover if we have one and the existing row doesn't.
            if cover_url.is_some() {
                sqlx::query(
                    "UPDATE game_works
                        SET cover_image_url = COALESCE(cover_image_url, $1), updated_at = now()
                      WHERE id = $2",
                )
                .bind(&cover_url)
                .bind(id)
                .execute(&state.db)
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            }
            id
        }
        None => insert_game_work(&state.db, src, &source_work_id, &source_url, &title, &circle).await?,
    };

    // Best-effort enrichment; failure doesn't block confirm.
    if let Ok(Some(detail)) = src.fetch_detail(&source_work_id).await {
        apply_detail(&state.db, game_work_id, detail, false).await?;
    }

    link_inventory_to_work(&state.db, body.inventory_item_id, game_work_id).await?;

    Ok(Json(ConfirmResponse {
        game_work_id: game_work_id.to_string(),
    }))
}

// ---- refresh --------------------------------------------------------------

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RefreshRequest {
    inventory_item_id: Uuid,
    source: String,
}

async fn refresh(
    State(state): State<AppState>,
    Json(body): Json<RefreshRequest>,
) -> Result<StatusCode, StatusCode> {
    let src = Src::from_name(&body.source).ok_or(StatusCode::BAD_REQUEST)?;

    let row = sqlx::query(
        "SELECT g.id AS game_work_id, g.dlsite_work_id, g.vndb_id
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
    let external_id: Option<String> = match src {
        Src::Dlsite => row.get("dlsite_work_id"),
        Src::Vndb => row.get("vndb_id"),
    };
    let Some(external_id) = external_id else {
        return Err(StatusCode::BAD_REQUEST);
    };

    let detail = match src.fetch_detail(&external_id).await {
        Ok(Some(d)) => d,
        Ok(None) => return Err(StatusCode::NOT_FOUND),
        Err(_) => return Err(StatusCode::BAD_GATEWAY),
    };

    apply_detail(&state.db, game_work_id, detail, true).await?;
    Ok(StatusCode::NO_CONTENT)
}

// ---- link -----------------------------------------------------------------

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
    let (src, external_id) =
        detect_link_source(&body.workno_or_url).ok_or(StatusCode::BAD_REQUEST)?;

    let detail = match src.fetch_detail(&external_id).await {
        Ok(Some(d)) => d,
        Ok(None) => return Err(StatusCode::NOT_FOUND),
        Err(_) => return Err(StatusCode::BAD_GATEWAY),
    };

    let source_url = source_url_for(src, &external_id);
    let title = detail
        .work_name
        .clone()
        .unwrap_or_else(|| external_id.clone());
    let circle = detail.maker_name.clone();

    let game_work_id = match find_game_work_by_external_id(&state.db, src, &external_id).await? {
        Some(id) => id,
        None => {
            insert_game_work(&state.db, src, &external_id, &source_url, &title, &circle).await?
        }
    };

    apply_detail(&state.db, game_work_id, detail, true).await?;
    link_inventory_to_work(&state.db, body.inventory_item_id, game_work_id).await?;

    Ok(Json(ConfirmResponse {
        game_work_id: game_work_id.to_string(),
    }))
}
