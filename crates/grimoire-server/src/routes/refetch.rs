//! One-off background re-fetch of every linked work's source data.
//!
//! Existing tags were stored without their source, and a later link to another
//! source could overwrite DLsite's work type. This walks every external id,
//! one request per second, and rewrites only `work_source_data`. Titles,
//! circles, descriptions and images on `game_works` are never touched, so
//! manual edits survive.

use crate::{
    error::{ApiError, ApiResult},
    repo,
    sources::Src,
    state::AppState,
};
use axum::{extract::State, routing::get, Json, Router};
use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::{PgPool, Row};
use std::{sync::Arc, time::Duration};
use tokio::sync::Mutex;
use uuid::Uuid;

const REQUEST_INTERVAL: Duration = Duration::from_secs(1);

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RefetchStatus {
    running: bool,
    total: usize,
    done: usize,
    failed: usize,
    started_at: Option<DateTime<Utc>>,
    finished_at: Option<DateTime<Utc>>,
    last_error: Option<String>,
}

pub type SharedRefetchStatus = Arc<Mutex<RefetchStatus>>;

pub fn router() -> Router<AppState> {
    Router::new().route("/refetch", get(status).post(start))
}

async fn status(State(state): State<AppState>) -> Json<RefetchStatus> {
    Json(state.refetch.lock().await.clone())
}

async fn start(State(state): State<AppState>) -> ApiResult<Json<RefetchStatus>> {
    let tasks = load_tasks(&state.db).await?;
    let snapshot = {
        let mut status = state.refetch.lock().await;
        if status.running {
            return Err(ApiError::conflict("a re-fetch is already running"));
        }
        *status = RefetchStatus {
            running: true,
            total: tasks.len(),
            started_at: Some(Utc::now()),
            ..RefetchStatus::default()
        };
        status.clone()
    };
    tokio::spawn(run(state.db.clone(), state.refetch.clone(), tasks));
    Ok(Json(snapshot))
}

struct Task {
    game_work_id: Uuid,
    src: Src,
    external_id: String,
}

async fn load_tasks(db: &PgPool) -> Result<Vec<Task>, sqlx::Error> {
    let rows = sqlx::query(
        "SELECT id, dlsite_work_id, vndb_id, steam_app_id
           FROM game_works
          WHERE dlsite_work_id IS NOT NULL OR vndb_id IS NOT NULL OR steam_app_id IS NOT NULL
          ORDER BY created_at",
    )
    .fetch_all(db)
    .await?;
    let mut tasks = Vec::new();
    for row in rows {
        let game_work_id: Uuid = row.get("id");
        for src in Src::ALL {
            if let Some(external_id) = row.get::<Option<String>, _>(src.id_column()) {
                tasks.push(Task {
                    game_work_id,
                    src,
                    external_id,
                });
            }
        }
    }
    Ok(tasks)
}

async fn run(db: PgPool, status: SharedRefetchStatus, tasks: Vec<Task>) {
    for (index, task) in tasks.iter().enumerate() {
        if index > 0 {
            tokio::time::sleep(REQUEST_INTERVAL).await;
        }
        let outcome = refetch_one(&db, task).await;
        let mut s = status.lock().await;
        s.done += 1;
        if let Err(message) = outcome {
            tracing::warn!(
                work = %task.game_work_id,
                source = task.src.name(),
                id = %task.external_id,
                error = %message,
                "re-fetch failed"
            );
            s.failed += 1;
            s.last_error = Some(format!(
                "{} {}: {}",
                task.src.name(),
                task.external_id,
                message
            ));
        }
    }
    let mut s = status.lock().await;
    s.running = false;
    s.finished_at = Some(Utc::now());
}

async fn refetch_one(db: &PgPool, task: &Task) -> Result<(), String> {
    let detail = match task.src.fetch_detail(&task.external_id).await {
        Ok(Some(detail)) => detail,
        Ok(None) => return Err("source has no record".to_string()),
        Err(e) => return Err(e.to_string()),
    };
    repo::upsert_source_data(db, task.game_work_id, task.src, &detail)
        .await
        .map_err(|e| e.to_string())
}
