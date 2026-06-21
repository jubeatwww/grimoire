use crate::state::AppState;
use axum::{extract::State, http::StatusCode, routing::post, Json, Router};
use grimoire_app::{
    dlsite::DlsiteSource,
    metadata_source::{MetadataSource, ProductDetail},
    steam::SteamSource,
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
        .route("/exclude", post(exclude))
        .route("/refresh", post(refresh))
        .route("/link", post(link))
        .route("/reset", post(reset))
        .route("/edit-work", post(edit_work))
        .route("/edit-item", post(edit_item))
        .route("/manual", post(manual))
        .route("/delete-item", post(delete_item))
        .route("/delete-missing", post(delete_missing))
        .route("/delete-item-and-file", post(delete_item_and_file))
}

// ---- source dispatch ------------------------------------------------------

#[derive(Debug, Clone, Copy)]
enum Src {
    Dlsite,
    Vndb,
    Steam,
}

impl Src {
    fn from_name(s: &str) -> Option<Self> {
        match s {
            "dlsite" => Some(Self::Dlsite),
            "vndb" => Some(Self::Vndb),
            "steam" => Some(Self::Steam),
            _ => None,
        }
    }

    async fn fetch_detail(&self, id: &str) -> anyhow::Result<Option<ProductDetail>> {
        match self {
            Self::Dlsite => DlsiteSource::new().fetch_product_detail(id).await,
            Self::Vndb => VndbSource::new().fetch_product_detail(id).await,
            Self::Steam => SteamSource::new().fetch_product_detail(id).await,
        }
    }
}

fn source_url_for(src: Src, id: &str) -> String {
    match src {
        Src::Dlsite => format!("https://www.dlsite.com/maniax/work/=/product_id/{id}.html"),
        Src::Vndb => format!("https://vndb.org/{id}"),
        Src::Steam => format!("https://store.steampowered.com/app/{id}/"),
    }
}

/// Auto-detect which source a raw user-pasted input belongs to. Order matters
/// from most-specific to most-permissive: Steam (full store URL) → VNDB
/// (vN token or vndb.org URL) → DLsite (RJ/VJ/BJ code or dlsite.com URL).
fn detect_link_source(input: &str) -> Option<(Src, String)> {
    if let Some(id) = SteamSource::extract_app_id(input) {
        return Some((Src::Steam, id));
    }
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
        Src::Steam => "SELECT id FROM game_works WHERE steam_app_id = $1",
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
        Src::Steam => {
            "INSERT INTO game_works (
                id, canonical_title, display_title, circle, source_urls,
                steam_app_id, genre_facets
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
    let result = if overwrite {
        // refresh / link path: the user explicitly chose this source, so it
        // wins on the source-derived fields — including title and circle, so
        // a locale-switched refresh actually updates the display string.
        sqlx::query(
            "UPDATE game_works SET
                display_title      = COALESCE($1, display_title),
                canonical_title    = COALESCE($1, canonical_title),
                circle             = COALESCE($2, circle),
                description        = $3,
                release_date       = COALESCE($4, release_date),
                series             = $5,
                source_tags        = $6,
                cover_image_url    = COALESCE($7, cover_image_url),
                preview_image_urls = $8,
                file_type          = $9,
                file_size_bytes    = $10,
                dl_count           = $11,
                rate_average       = $12,
                rate_count         = $13,
                price_jpy          = $14,
                work_type          = $15,
                work_type_label    = $16,
                enriched_at        = now(),
                updated_at         = now()
             WHERE id = $17",
        )
        .bind(detail.work_name)
        .bind(detail.maker_name)
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
    } else {
        // confirm path: preserve any prior non-null values so a different
        // source's earlier confirm doesn't get clobbered.
        sqlx::query(
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
                enriched_at        = now(),
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
        .execute(db)
        .await
    };
    result.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
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

/// Exclude is the "this isn't a game / compilation / junk — never process it"
/// marker. Distinct from Skip (`no_match`) which means "source didn't have it,
/// try again later (e.g. via VNDB)".
async fn exclude(
    State(state): State<AppState>,
    Json(body): Json<ItemIdRequest>,
) -> Result<StatusCode, StatusCode> {
    sqlx::query(
        "UPDATE inventory_items
            SET organization_status = 'ignored', updated_at = now()
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
    cover_image_url: Option<String>,
    preview_image_urls: Option<Vec<String>>,
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
    // work_type / work_type_label / cover_image_url: empty string explicitly
    // clears (lets the user strip an over-eager source-derived value); missing
    // leaves it alone.
    // preview_image_urls: null means no change, an array (possibly empty)
    // replaces the whole set.
    let preview_json = body.preview_image_urls.as_ref().map(serde_json::to_value);
    let preview_value: Option<serde_json::Value> = match preview_json {
        Some(Ok(v)) => Some(v),
        Some(Err(_)) => return Err(StatusCode::BAD_REQUEST),
        None => None,
    };
    sqlx::query(
        "UPDATE game_works SET
            display_title      = COALESCE(NULLIF($1, ''), display_title),
            work_type          = CASE WHEN $2::text IS NULL THEN work_type ELSE NULLIF($2, '') END,
            work_type_label    = CASE WHEN $3::text IS NULL THEN work_type_label ELSE NULLIF($3, '') END,
            cover_image_url    = CASE WHEN $4::text IS NULL THEN cover_image_url ELSE NULLIF($4, '') END,
            preview_image_urls = COALESCE($5, preview_image_urls),
            updated_at         = now()
         WHERE id = $6",
    )
    .bind(body.display_title)
    .bind(body.work_type)
    .bind(body.work_type_label)
    .bind(body.cover_image_url)
    .bind(preview_value)
    .bind(game_work_id)
    .execute(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(StatusCode::NO_CONTENT)
}

/// One-shot cleanup: drop every inventory_item flagged missing. Returned
/// count tells the UI how many rows actually went away.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct DeleteMissingResponse {
    deleted: u64,
}

async fn delete_missing(
    State(state): State<AppState>,
) -> Result<Json<DeleteMissingResponse>, StatusCode> {
    let result = sqlx::query("DELETE FROM inventory_items WHERE missing = true")
        .execute(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(DeleteMissingResponse {
        deleted: result.rows_affected(),
    }))
}

/// Delete the file under library_root AND the inventory_item row. Intended
/// for cleaning up duplicates — destructive on the filesystem, so the UI
/// gates this behind an extra-click menu + confirmation dialog. The linked
/// game_work is left in place.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct DeleteWithFileResponse {
    file_deleted: bool,
    file_missing: bool,
}

async fn delete_item_and_file(
    State(state): State<AppState>,
    Json(body): Json<ItemIdRequest>,
) -> Result<Json<DeleteWithFileResponse>, StatusCode> {
    let row = sqlx::query("SELECT path FROM inventory_items WHERE id = $1")
        .bind(body.inventory_item_id)
        .fetch_optional(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let Some(row) = row else {
        return Err(StatusCode::NOT_FOUND);
    };
    let rel_path: String = row.get("path");

    let full_path = state
        .library_root
        .resolve_relative(&rel_path)
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    let mut file_deleted = false;
    let mut file_missing = false;
    match tokio::fs::remove_file(&full_path).await {
        Ok(()) => file_deleted = true,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => file_missing = true,
        Err(_) => return Err(StatusCode::INTERNAL_SERVER_ERROR),
    }

    sqlx::query("DELETE FROM inventory_items WHERE id = $1")
        .bind(body.inventory_item_id)
        .execute(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(DeleteWithFileResponse {
        file_deleted,
        file_missing,
    }))
}

/// Hard-delete an inventory_item row. Use this to drop orphan records once
/// the underlying file is gone for good (e.g. Synology #recycle exits scan
/// scope after the user emptied the recycle bin). The linked game_work is
/// left untouched in case other inventory items reference it.
async fn delete_item(
    State(state): State<AppState>,
    Json(body): Json<ItemIdRequest>,
) -> Result<StatusCode, StatusCode> {
    let result = sqlx::query("DELETE FROM inventory_items WHERE id = $1")
        .bind(body.inventory_item_id)
        .execute(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    if result.rows_affected() == 0 {
        return Err(StatusCode::NOT_FOUND);
    }
    Ok(StatusCode::NO_CONTENT)
}

/// Create an empty game_work for a pending inventory item so the user can fill
/// it in manually via InlineText / image edits. Title defaults to the filename.
async fn manual(
    State(state): State<AppState>,
    Json(body): Json<ItemIdRequest>,
) -> Result<Json<ConfirmResponse>, StatusCode> {
    let row = sqlx::query(
        "SELECT file_name, game_work_id FROM inventory_items WHERE id = $1",
    )
    .bind(body.inventory_item_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let Some(row) = row else {
        return Err(StatusCode::NOT_FOUND);
    };

    // If the item already links somewhere, treat this as a no-op rather than
    // creating an orphan game_work the user can't get back to.
    if let Some(existing) = row.get::<Option<Uuid>, _>("game_work_id") {
        return Ok(Json(ConfirmResponse {
            game_work_id: existing.to_string(),
        }));
    }

    let file_name: String = row.get("file_name");
    let new_id = Uuid::new_v4();
    let source_urls = serde_json::json!([]);
    let genre_facets = serde_json::json!([]);
    // enriched_at = now() so the item doesn't immediately bounce into the
    // "missing detail" filter that targets confirmed-but-never-enriched rows.
    sqlx::query(
        "INSERT INTO game_works (
            id, canonical_title, display_title, source_urls, genre_facets, enriched_at
        ) VALUES ($1, $2, $3, $4, $5, now())",
    )
    .bind(new_id)
    .bind(&file_name)
    .bind(&file_name)
    .bind(&source_urls)
    .bind(&genre_facets)
    .execute(&state.db)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    link_inventory_to_work(&state.db, body.inventory_item_id, new_id).await?;

    Ok(Json(ConfirmResponse {
        game_work_id: new_id.to_string(),
    }))
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
    let st = SteamSource::new();
    let (dl_res, vn_res, st_res) = tokio::join!(
        dl.search(&body.query),
        vn.search(&body.query),
        st.search(&body.query),
    );

    let mut candidates: Vec<grimoire_domain::metadata::MetadataCandidate> = Vec::new();
    if let Ok(v) = dl_res {
        candidates.extend(v);
    }
    if let Ok(v) = vn_res {
        candidates.extend(v);
    }
    if let Ok(v) = st_res {
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
        "SELECT g.id AS game_work_id, g.dlsite_work_id, g.vndb_id, g.steam_app_id
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
        Src::Steam => row.get("steam_app_id"),
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
