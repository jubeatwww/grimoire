use crate::{
    error::{ApiError, ApiResult},
    repo,
    sources::Src,
    state::AppState,
};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{delete, get, post},
    Json, Router,
};
use grimoire_app::{
    classification::{self, KeywordRule, Suggestion, WorkTypeRule},
    vocabulary::Vocabulary,
};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use uuid::Uuid;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/confirm", post(confirm))
        .route("/suggestions", get(suggestions))
        .route("/rules", get(list_rules))
        .route("/rules/work-type", post(upsert_work_type_rule))
        .route("/rules/work-type/{source}/{work_type}", delete(delete_work_type_rule))
        .route("/rules/keyword", post(upsert_keyword_rule))
        .route("/rules/keyword/{id}", delete(delete_keyword_rule))
        .route("/common-tags", get(common_tags))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ConfirmItem {
    game_work_id: Uuid,
    /// `None` clears the category and its confirmation.
    primary_category: Option<String>,
    #[serde(default)]
    genre_facets: Vec<String>,
}

#[derive(Deserialize)]
struct ConfirmRequest {
    items: Vec<ConfirmItem>,
}

#[derive(Serialize)]
struct ConfirmResponse {
    updated: usize,
}

/// Set the primary category and genre codes on one or more works. Any manual
/// change counts as confirmation. All items are validated before anything is
/// written, and the writes happen in one transaction.
async fn confirm(
    State(state): State<AppState>,
    Json(body): Json<ConfirmRequest>,
) -> ApiResult<Json<ConfirmResponse>> {
    let vocab = repo::load_vocabulary(&state.db).await?;
    let mut prepared = Vec::with_capacity(body.items.len());
    for item in &body.items {
        let category = item
            .primary_category
            .as_deref()
            .map(str::trim)
            .filter(|c| !c.is_empty());
        let facets = vocab
            .prepare_classification(category, item.genre_facets.iter().map(String::as_str))
            .map_err(|e| ApiError::bad_request(e.to_string()))?;
        prepared.push((item.game_work_id, category.map(str::to_string), facets));
    }

    let mut tx = state.db.begin().await?;
    for (game_work_id, category, facets) in &prepared {
        let result = sqlx::query(
            "UPDATE game_works SET
                primary_category      = $1,
                genre_facets          = $2,
                category_confirmed_at = CASE WHEN $1::text IS NULL THEN NULL ELSE now() END,
                updated_at            = now()
              WHERE id = $3",
        )
        .bind(category)
        .bind(serde_json::json!(facets))
        .bind(game_work_id)
        .execute(&mut *tx)
        .await?;
        if result.rows_affected() == 0 {
            return Err(ApiError::not_found(format!("no such game work: {game_work_id}")));
        }
    }
    tx.commit().await?;

    Ok(Json(ConfirmResponse {
        updated: prepared.len(),
    }))
}

// ---- suggestions ----------------------------------------------------------

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct WorkSuggestion {
    game_work_id: Uuid,
    #[serde(flatten)]
    suggestion: Suggestion,
}

#[derive(Serialize)]
struct SuggestionsResponse {
    suggestions: Vec<WorkSuggestion>,
}

async fn suggestions(State(state): State<AppState>) -> ApiResult<Json<SuggestionsResponse>> {
    let vocab = repo::load_vocabulary(&state.db).await?;
    let rules = repo::load_rules(&state.db).await?;
    let works = repo::load_unclassified_facts(&state.db).await?;
    let suggestions = works
        .into_iter()
        .map(|(game_work_id, facts)| WorkSuggestion {
            game_work_id,
            suggestion: classification::suggest(&vocab, &rules, &facts),
        })
        .collect();
    Ok(Json(SuggestionsResponse { suggestions }))
}

// ---- rules ----------------------------------------------------------------

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RulesResponse {
    work_type_rules: Vec<WorkTypeRule>,
    keyword_rules: Vec<KeywordRule>,
}

async fn list_rules(State(state): State<AppState>) -> ApiResult<Json<RulesResponse>> {
    let rules = repo::load_rules(&state.db).await?;
    Ok(Json(RulesResponse {
        work_type_rules: rules.work_types,
        keyword_rules: rules.keywords,
    }))
}

fn parse_source(source: &str) -> ApiResult<Src> {
    Src::from_name(source).ok_or_else(|| ApiError::bad_request(format!("unknown source: {source}")))
}

fn normalize_rule_code(vocab: &Vocabulary, code: &str) -> ApiResult<String> {
    vocab
        .normalize(code)
        .ok_or_else(|| ApiError::bad_request(format!("unknown genre code: {code}")))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct WorkTypeRuleRequest {
    source: String,
    work_type: String,
    /// `None` means "decide from the tags".
    code: Option<String>,
    confident: bool,
}

async fn upsert_work_type_rule(
    State(state): State<AppState>,
    Json(body): Json<WorkTypeRuleRequest>,
) -> ApiResult<Json<WorkTypeRule>> {
    let src = parse_source(&body.source)?;
    let work_type = body.work_type.trim().to_uppercase();
    if work_type.is_empty() {
        return Err(ApiError::bad_request("work type is required"));
    }
    let vocab = repo::load_vocabulary(&state.db).await?;
    let code = match body.code.as_deref().map(str::trim).filter(|c| !c.is_empty()) {
        Some(c) => Some(normalize_rule_code(&vocab, c)?),
        None => None,
    };
    // A rule without a code can never be confident: the tags decide.
    let confident = body.confident && code.is_some();
    sqlx::query(
        "INSERT INTO work_type_rules (source, work_type, code, confident)
         VALUES ($1, $2, $3, $4)
         ON CONFLICT (source, work_type) DO UPDATE SET
            code = EXCLUDED.code, confident = EXCLUDED.confident",
    )
    .bind(src.name())
    .bind(&work_type)
    .bind(&code)
    .bind(confident)
    .execute(&state.db)
    .await?;
    Ok(Json(WorkTypeRule {
        source: src.name().to_string(),
        work_type,
        code,
        confident,
    }))
}

async fn delete_work_type_rule(
    State(state): State<AppState>,
    Path((source, work_type)): Path<(String, String)>,
) -> ApiResult<StatusCode> {
    let result = sqlx::query("DELETE FROM work_type_rules WHERE source = $1 AND work_type = $2")
        .bind(&source)
        .bind(&work_type)
        .execute(&state.db)
        .await?;
    if result.rows_affected() == 0 {
        return Err(ApiError::not_found("no such rule"));
    }
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct KeywordRuleRequest {
    /// Present to edit an existing rule.
    id: Option<Uuid>,
    /// `None` applies the keyword to every source.
    source: Option<String>,
    keyword: String,
    code: String,
    #[serde(default)]
    contests_type: bool,
}

async fn upsert_keyword_rule(
    State(state): State<AppState>,
    Json(body): Json<KeywordRuleRequest>,
) -> ApiResult<Json<KeywordRule>> {
    let source = match body.source.as_deref().filter(|s| !s.is_empty()) {
        Some(s) => Some(parse_source(s)?.name().to_string()),
        None => None,
    };
    let keyword = body.keyword.trim().to_string();
    if keyword.is_empty() {
        return Err(ApiError::bad_request("keyword is required"));
    }
    let vocab = repo::load_vocabulary(&state.db).await?;
    let code = normalize_rule_code(&vocab, &body.code)?;

    let result = match body.id {
        Some(id) => {
            sqlx::query(
                "UPDATE keyword_rules
                    SET source = $1, keyword = $2, code = $3, contests_type = $4
                  WHERE id = $5
              RETURNING id",
            )
            .bind(&source)
            .bind(&keyword)
            .bind(&code)
            .bind(body.contests_type)
            .bind(id)
            .fetch_optional(&state.db)
            .await
        }
        None => {
            sqlx::query(
                "INSERT INTO keyword_rules (source, keyword, code, contests_type)
                 VALUES ($1, $2, $3, $4)
              RETURNING id",
            )
            .bind(&source)
            .bind(&keyword)
            .bind(&code)
            .bind(body.contests_type)
            .fetch_optional(&state.db)
            .await
        }
    };
    let row = match result {
        Ok(Some(row)) => row,
        Ok(None) => return Err(ApiError::not_found("no such rule")),
        Err(sqlx::Error::Database(db)) if db.is_unique_violation() => {
            return Err(ApiError::conflict(format!(
                "a rule for keyword {keyword} already exists for this source"
            )))
        }
        Err(e) => return Err(e.into()),
    };
    Ok(Json(KeywordRule {
        id: row.get::<Uuid, _>("id").to_string(),
        source,
        keyword,
        code,
        contests_type: body.contests_type,
    }))
}

async fn delete_keyword_rule(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> ApiResult<StatusCode> {
    let result = sqlx::query("DELETE FROM keyword_rules WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;
    if result.rows_affected() == 0 {
        return Err(ApiError::not_found("no such rule"));
    }
    Ok(StatusCode::NO_CONTENT)
}

// ---- common tags ----------------------------------------------------------

const COMMON_TAG_LIMIT: usize = 150;

#[derive(Serialize)]
struct CommonTag {
    label: String,
    source: String,
    count: i64,
}

#[derive(Serialize)]
struct CommonTagsResponse {
    tags: Vec<CommonTag>,
}

/// The most frequent source tags on unclassified works that no keyword rule
/// matches yet: the raw material for new keyword rules.
async fn common_tags(State(state): State<AppState>) -> ApiResult<Json<CommonTagsResponse>> {
    let rules = repo::load_rules(&state.db).await?;
    let rows = sqlx::query(
        "SELECT t.label, s.source, count(*) AS n
           FROM work_source_data s
           JOIN game_works g ON g.id = s.game_work_id
     CROSS JOIN jsonb_array_elements_text(s.tags) AS t(label)
          WHERE g.category_confirmed_at IS NULL
       GROUP BY t.label, s.source
       ORDER BY n DESC, t.label
          LIMIT 1000",
    )
    .fetch_all(&state.db)
    .await?;
    let tags = rows
        .into_iter()
        .map(|row| CommonTag {
            label: row.get("label"),
            source: row.get("source"),
            count: row.get("n"),
        })
        .filter(|tag| {
            let lower = tag.label.to_lowercase();
            !rules.keywords.iter().any(|r| {
                r.source.as_deref().is_none_or(|s| s == tag.source)
                    && lower.contains(&r.keyword.to_lowercase())
            })
        })
        .take(COMMON_TAG_LIMIT)
        .collect();
    Ok(Json(CommonTagsResponse { tags }))
}
