use crate::{
    error::{ApiError, ApiResult},
    repo,
    state::AppState,
};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{delete, get, post},
    Json, Router,
};
use grimoire_app::vocabulary::{canonical_form, CategoryDef, CodeDef};
use serde::{Deserialize, Serialize};
use sqlx::Row;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(get_vocabulary))
        .route("/codes", post(upsert_code))
        .route("/codes/{code}", delete(delete_code))
}

#[derive(Serialize)]
struct VocabularyResponse {
    categories: Vec<CategoryDef>,
    codes: Vec<CodeDef>,
}

async fn get_vocabulary(State(state): State<AppState>) -> ApiResult<Json<VocabularyResponse>> {
    let vocab = repo::load_vocabulary(&state.db).await?;
    Ok(Json(VocabularyResponse {
        categories: vocab.categories().to_vec(),
        codes: vocab.codes().to_vec(),
    }))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct UpsertCodeRequest {
    code: String,
    label: String,
    category_id: String,
    family: Option<String>,
    #[serde(default)]
    accepts_prefix: bool,
    #[serde(default)]
    aliases: Vec<String>,
}

/// Add a new code or edit an existing one (label, category, family, aliases).
async fn upsert_code(
    State(state): State<AppState>,
    Json(body): Json<UpsertCodeRequest>,
) -> ApiResult<Json<CodeDef>> {
    let code = canonical_form(&body.code);
    let label = body.label.trim().to_string();
    if code.is_empty() || label.is_empty() {
        return Err(ApiError::bad_request("code and label are required"));
    }
    let vocab = repo::load_vocabulary(&state.db).await?;
    let existing = vocab.resolve(&code);
    if existing.is_some_and(|r| r.prefix.is_some()) {
        return Err(ApiError::conflict(format!(
            "{code} already means a T-/X- variant of an existing code"
        )));
    }
    let category_exists: bool =
        sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM categories WHERE id = $1)")
            .bind(&body.category_id)
            .fetch_one(&state.db)
            .await?;
    if !category_exists {
        return Err(ApiError::bad_request(format!(
            "unknown category: {}",
            body.category_id
        )));
    }
    // A named level (SSG³) joins the family of the code it levels.
    let default_family = existing
        .filter(|r| r.level.is_some())
        .map_or_else(|| code.clone(), |r| r.base.family.clone());
    let family = body
        .family
        .map(|f| canonical_form(&f))
        .filter(|f| !f.is_empty())
        .unwrap_or(default_family);
    let aliases: Vec<String> = body
        .aliases
        .iter()
        .map(|a| canonical_form(a))
        .filter(|a| !a.is_empty() && *a != code)
        .collect();

    let row = sqlx::query(
        "INSERT INTO genre_codes (code, label, category_id, family, accepts_prefix, aliases, sort_order)
         VALUES ($1, $2, $3, $4, $5, $6,
                 COALESCE((SELECT max(sort_order) + 1 FROM genre_codes WHERE category_id = $3), 0))
         ON CONFLICT (code) DO UPDATE SET
            label          = EXCLUDED.label,
            category_id    = EXCLUDED.category_id,
            family         = EXCLUDED.family,
            accepts_prefix = EXCLUDED.accepts_prefix,
            aliases        = EXCLUDED.aliases
         RETURNING code, label, category_id, family, accepts_prefix, aliases, sort_order",
    )
    .bind(&code)
    .bind(&label)
    .bind(&body.category_id)
    .bind(&family)
    .bind(body.accepts_prefix)
    .bind(serde_json::json!(aliases))
    .fetch_one(&state.db)
    .await?;

    let aliases: serde_json::Value = row.get("aliases");
    Ok(Json(CodeDef {
        code: row.get("code"),
        label: row.get("label"),
        category_id: row.get("category_id"),
        family: row.get("family"),
        accepts_prefix: row.get("accepts_prefix"),
        aliases: serde_json::from_value(aliases).unwrap_or_default(),
        sort_order: row.get("sort_order"),
    }))
}

/// Remove a code nobody uses. Category ids are protected because confirming a
/// category falls back to its id as the work's own code.
async fn delete_code(
    State(state): State<AppState>,
    Path(code): Path<String>,
) -> ApiResult<StatusCode> {
    let is_category: bool =
        sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM categories WHERE id = $1)")
            .bind(&code)
            .fetch_one(&state.db)
            .await?;
    if is_category {
        return Err(ApiError::conflict(format!(
            "{code} is a category's own code and cannot be removed"
        )));
    }
    // Stored codes may carry a prefix or a level, so match them through the
    // vocabulary rather than by spelling.
    let vocab = repo::load_vocabulary(&state.db).await?;
    let uses_code = |stored: &[String]| {
        stored
            .iter()
            .any(|s| vocab.resolve(s).is_some_and(|r| r.base.code == code))
    };
    let facets: Vec<String> = sqlx::query_scalar(
        "SELECT DISTINCT jsonb_array_elements_text(genre_facets) FROM game_works",
    )
    .fetch_all(&state.db)
    .await?;
    if uses_code(&facets) {
        return Err(ApiError::conflict(format!(
            "{code} is still used by at least one game"
        )));
    }
    let rule_codes: Vec<String> = sqlx::query_scalar(
        "SELECT code FROM work_type_rules WHERE code IS NOT NULL
         UNION SELECT code FROM keyword_rules",
    )
    .fetch_all(&state.db)
    .await?;
    if uses_code(&rule_codes) {
        return Err(ApiError::conflict(format!(
            "{code} is still used by a classification rule"
        )));
    }
    let result = sqlx::query("DELETE FROM genre_codes WHERE code = $1")
        .bind(&code)
        .execute(&state.db)
        .await?;
    if result.rows_affected() == 0 {
        return Err(ApiError::not_found(format!("no such code: {code}")));
    }
    Ok(StatusCode::NO_CONTENT)
}
