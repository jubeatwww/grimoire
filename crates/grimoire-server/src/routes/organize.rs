//! Moving classified files into the new NAS layout: plan, preview, execute,
//! undo. All writes to the library go through `grimoire_app::library_writer`
//! while holding `AppState::library_lock`, which scans take too.

use crate::{
    error::{ApiError, ApiResult},
    repo,
    routes::scan::SOURCE_ID,
    state::AppState,
};
use axum::{
    extract::{Path, State},
    routing::{get, post},
    Json, Router,
};
use chrono::{DateTime, Utc};
use grimoire_app::{
    library_writer::{move_no_overwrite, same_file, MoveError, PruneGuard},
    organization::{plan_target, work_folder_target, FilePlacement, INBOX_FOLDER},
    scanner::{report_other_files, OtherEntry},
};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use std::{
    collections::{HashMap, HashSet},
    path::{Path as FsPath, PathBuf},
};
use uuid::Uuid;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/plan", get(plan))
        .route("/preview", post(preview))
        .route("/execute", post(execute))
        .route("/batches", get(list_batches))
        .route("/batches/{id}", get(get_batch))
        .route("/batches/{id}/undo", post(undo))
        .route("/inbox", get(inbox_status).post(create_inbox))
        .route("/other-files", get(other_files))
}

// ---- planning -------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlannedMove {
    inventory_item_id: Uuid,
    game_work_id: Uuid,
    title: String,
    category_id: String,
    from_path: String,
    to_path: String,
    /// Why this file cannot move right now, if it cannot.
    conflict: Option<String>,
}

/// Every file of a confirmed work that is not where it belongs, with
/// conflicts marked. `only` limits the result to those inventory items.
async fn compute_plan(state: &AppState, only: Option<&[Uuid]>) -> ApiResult<Vec<PlannedMove>> {
    let rows = sqlx::query(
        "SELECT i.id, i.path, i.file_name, g.id AS game_work_id, g.display_title, g.circle,
                c.id AS category_id, c.folder_name,
                (SELECT count(*) FROM inventory_items j
                  WHERE j.game_work_id = g.id AND NOT j.missing
                    AND j.organization_status <> 'ignored') AS work_files
           FROM inventory_items i
           JOIN game_works g ON g.id = i.game_work_id
           JOIN categories c ON c.id = g.primary_category
          WHERE g.category_confirmed_at IS NOT NULL
            AND i.source_id = $1
            AND NOT i.missing
            AND i.organization_status <> 'ignored'
          ORDER BY c.sort_order, g.circle, g.display_title, i.file_name",
    )
    .bind(SOURCE_ID)
    .fetch_all(&state.db)
    .await?;

    let mut moves: Vec<PlannedMove> = Vec::new();
    // Work folders and the works that end up inside each one, counting files
    // already in place. Two works sharing a folder name must not be mixed.
    let mut folder_owners: HashMap<String, HashSet<Uuid>> = HashMap::new();
    for row in rows {
        let path: String = row.get("path");
        let file_name: String = row.get("file_name");
        let title: String = row.get("display_title");
        let circle: Option<String> = row.get("circle");
        let folder: String = row.get("folder_name");
        let work_files: i64 = row.get("work_files");
        let placement = FilePlacement {
            category_folder: &folder,
            circle: circle.as_deref(),
            work_title: &title,
            file_name: &file_name,
            current_path: &path,
            work_file_count: work_files as usize,
        };
        let game_work_id: Uuid = row.get("game_work_id");
        let target = plan_target(&placement);
        let in_work = work_folder_target(&placement);
        if target.as_deref().unwrap_or(&path) == in_work {
            if let Some((dir, _)) = in_work.rsplit_once('/') {
                folder_owners
                    .entry(dir.to_string())
                    .or_default()
                    .insert(game_work_id);
            }
        }
        if let Some(to_path) = target {
            moves.push(PlannedMove {
                inventory_item_id: row.get("id"),
                game_work_id,
                title,
                category_id: row.get("category_id"),
                from_path: path,
                to_path,
                conflict: None,
            });
        }
    }

    mark_conflicts(state, &mut moves).await?;
    for m in moves.iter_mut().filter(|m| m.conflict.is_none()) {
        let shared = m
            .to_path
            .rsplit_once('/')
            .and_then(|(dir, _)| folder_owners.get(dir))
            .is_some_and(|owners| owners.len() > 1);
        if shared {
            m.conflict = Some(
                "another work's title maps to the same folder; change one of the titles".to_string(),
            );
        }
    }

    if let Some(only) = only {
        let wanted: HashSet<&Uuid> = only.iter().collect();
        moves.retain(|m| wanted.contains(&m.inventory_item_id));
    }
    Ok(moves)
}

async fn mark_conflicts(state: &AppState, moves: &mut [PlannedMove]) -> ApiResult<()> {
    // Two files heading for the same path.
    let mut target_count: HashMap<&str, usize> = HashMap::new();
    for m in moves.iter() {
        *target_count.entry(m.to_path.as_str()).or_default() += 1;
    }
    let duplicated: HashSet<String> = target_count
        .into_iter()
        .filter(|(_, n)| *n > 1)
        .map(|(p, _)| p.to_string())
        .collect();

    // Another inventory row already at the target (also blocks the unique key).
    let targets: Vec<String> = moves.iter().map(|m| m.to_path.clone()).collect();
    let occupied: HashMap<String, bool> = sqlx::query(
        "SELECT path, missing FROM inventory_items WHERE source_id = $1 AND path = ANY($2)",
    )
    .bind(SOURCE_ID)
    .bind(&targets)
    .fetch_all(&state.db)
    .await?
    .into_iter()
    .map(|r| (r.get("path"), r.get("missing")))
    .collect();

    let root = state.library_root.clone();
    // Filesystem checks can be slow over NFS; keep them off the async workers.
    let on_disk: Vec<bool> = tokio::task::spawn_blocking(move || {
        targets
            .iter()
            .map(|to| {
                root.resolve_relative(to)
                    .map(|p| std::fs::symlink_metadata(p).is_ok())
                    .unwrap_or(true)
            })
            .collect()
    })
    .await
    .map_err(ApiError::internal)?;

    for (m, exists) in moves.iter_mut().zip(on_disk) {
        m.conflict = if duplicated.contains(&m.to_path) {
            Some("another file is planned for the same path".to_string())
        } else if let Some(missing) = occupied.get(&m.to_path) {
            Some(if *missing {
                "a missing-file record still holds this path; delete it first".to_string()
            } else {
                "another inventory record is already at this path".to_string()
            })
        } else if exists {
            Some("a file already exists at the target".to_string())
        } else {
            None
        };
    }
    Ok(())
}

/// Folders pruning must leave alone: the root, every category folder, the
/// inbox, the asset cache and the Synology recycle bin.
async fn prune_guard(state: &AppState) -> ApiResult<PruneGuard> {
    let vocab = repo::load_vocabulary(&state.db).await?;
    let root = state.library_root.root().to_path_buf();
    let mut protected: Vec<PathBuf> = vocab
        .categories()
        .iter()
        .map(|c| root.join(&c.folder_name))
        .collect();
    protected.push(root.join(INBOX_FOLDER));
    protected.push(root.join("#recycle"));
    protected.push(state.asset_cache_root.clone());
    Ok(PruneGuard::new(root, protected))
}

fn absolute(state: &AppState, rel: &str) -> ApiResult<PathBuf> {
    state
        .library_root
        .resolve_relative(rel)
        .map_err(|e| ApiError::bad_request(format!("bad path {rel}: {e}")))
}

fn relative(state: &AppState, abs: &FsPath) -> String {
    abs.strip_prefix(state.library_root.root())
        .unwrap_or(abs)
        .to_string_lossy()
        .replace('\\', "/")
}

#[derive(Serialize)]
struct PlanResponse {
    moves: Vec<PlannedMove>,
}

async fn plan(State(state): State<AppState>) -> ApiResult<Json<PlanResponse>> {
    Ok(Json(PlanResponse {
        moves: compute_plan(&state, None).await?,
    }))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SelectionRequest {
    inventory_item_ids: Vec<Uuid>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PreviewResponse {
    moves: Vec<PlannedMove>,
    removable_dirs: Vec<String>,
}

async fn preview(
    State(state): State<AppState>,
    Json(body): Json<SelectionRequest>,
) -> ApiResult<Json<PreviewResponse>> {
    let moves = compute_plan(&state, Some(&body.inventory_item_ids)).await?;
    let guard = prune_guard(&state).await?;
    let mut out = Vec::new();
    let mut into = Vec::new();
    for m in moves.iter().filter(|m| m.conflict.is_none()) {
        out.push(absolute(&state, &m.from_path)?);
        into.push(absolute(&state, &m.to_path)?);
    }
    let dirs = tokio::task::spawn_blocking(move || guard.preview(&out, &into))
        .await
        .map_err(ApiError::internal)?;
    Ok(Json(PreviewResponse {
        removable_dirs: dirs.iter().map(|d| relative(&state, d)).collect(),
        moves,
    }))
}

// ---- batches --------------------------------------------------------------

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct MoveOp {
    seq: i32,
    inventory_item_id: Option<Uuid>,
    from_path: String,
    to_path: String,
    status: String,
    message: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Batch {
    id: Uuid,
    status: String,
    moved: i32,
    skipped: i32,
    failed: i32,
    removed_dirs: Vec<String>,
    notes: Vec<String>,
    created_at: DateTime<Utc>,
    finished_at: Option<DateTime<Utc>>,
    undone_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    ops: Option<Vec<MoveOp>>,
}

async fn load_batch(state: &AppState, id: Uuid, with_ops: bool) -> ApiResult<Batch> {
    let row = sqlx::query(
        "SELECT id, status, moved, skipped, failed, removed_dirs, notes,
                created_at, finished_at, undone_at
           FROM move_batches WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| ApiError::not_found("no such batch"))?;
    let mut batch = batch_from_row(&row);
    if with_ops {
        let ops = sqlx::query(
            "SELECT seq, inventory_item_id, from_path, to_path, status, message
               FROM move_ops WHERE batch_id = $1 ORDER BY seq",
        )
        .bind(id)
        .fetch_all(&state.db)
        .await?
        .into_iter()
        .map(|r| MoveOp {
            seq: r.get("seq"),
            inventory_item_id: r.get("inventory_item_id"),
            from_path: r.get("from_path"),
            to_path: r.get("to_path"),
            status: r.get("status"),
            message: r.get("message"),
        })
        .collect();
        batch.ops = Some(ops);
    }
    Ok(batch)
}

fn batch_from_row(row: &sqlx::postgres::PgRow) -> Batch {
    let removed: serde_json::Value = row.get("removed_dirs");
    let notes: serde_json::Value = row.get("notes");
    Batch {
        id: row.get("id"),
        status: row.get("status"),
        moved: row.get("moved"),
        skipped: row.get("skipped"),
        failed: row.get("failed"),
        removed_dirs: serde_json::from_value(removed).unwrap_or_default(),
        notes: serde_json::from_value(notes).unwrap_or_default(),
        created_at: row.get("created_at"),
        finished_at: row.get("finished_at"),
        undone_at: row.get("undone_at"),
        ops: None,
    }
}

#[derive(Serialize)]
struct BatchListResponse {
    batches: Vec<Batch>,
}

async fn list_batches(State(state): State<AppState>) -> ApiResult<Json<BatchListResponse>> {
    let batches = sqlx::query(
        "SELECT id, status, moved, skipped, failed, removed_dirs, notes,
                created_at, finished_at, undone_at
           FROM move_batches ORDER BY created_at DESC LIMIT 50",
    )
    .fetch_all(&state.db)
    .await?
    .iter()
    .map(batch_from_row)
    .collect();
    Ok(Json(BatchListResponse { batches }))
}

async fn get_batch(State(state): State<AppState>, Path(id): Path<Uuid>) -> ApiResult<Json<Batch>> {
    Ok(Json(load_batch(&state, id, true).await?))
}

/// Move one file on disk, off the async workers.
async fn fs_move(from: PathBuf, to: PathBuf) -> Result<(), MoveError> {
    tokio::task::spawn_blocking(move || move_no_overwrite(&from, &to))
        .await
        .unwrap_or_else(|e| Err(MoveError::Io(std::io::Error::other(e.to_string()))))
}

/// Prune each starting folder upwards; returns removed folders and errors.
async fn prune_all(guard: PruneGuard, starts: Vec<PathBuf>) -> (Vec<PathBuf>, Vec<String>) {
    tokio::task::spawn_blocking(move || {
        let mut removed = Vec::new();
        let mut errors = Vec::new();
        for start in starts {
            let outcome = guard.prune_upwards(&start);
            removed.extend(outcome.removed);
            if let Some(e) = outcome.error {
                errors.push(format!("could not prune {}: {e}", start.display()));
            }
        }
        (removed, errors)
    })
    .await
    .unwrap_or_else(|e| (Vec::new(), vec![format!("prune task failed: {e}")]))
}

async fn insert_op(
    state: &AppState,
    batch_id: Uuid,
    seq: i32,
    m: &PlannedMove,
    status: &str,
    message: Option<String>,
) -> ApiResult<Uuid> {
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO move_ops (id, batch_id, seq, inventory_item_id, from_path, to_path, status, message)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
    )
    .bind(id)
    .bind(batch_id)
    .bind(seq)
    .bind(m.inventory_item_id)
    .bind(&m.from_path)
    .bind(&m.to_path)
    .bind(status)
    .bind(message)
    .execute(&state.db)
    .await?;
    Ok(id)
}

async fn set_op(state: &AppState, op_id: Uuid, status: &str, message: Option<String>) -> ApiResult<()> {
    sqlx::query("UPDATE move_ops SET status = $1, message = $2, updated_at = now() WHERE id = $3")
        .bind(status)
        .bind(message)
        .bind(op_id)
        .execute(&state.db)
        .await?;
    Ok(())
}

/// Point the inventory row at its new path. Only succeeds while the row is
/// still where we found it.
async fn update_path(state: &AppState, item: Uuid, from: &str, to: &str) -> Result<bool, sqlx::Error> {
    let result = sqlx::query(
        "UPDATE inventory_items SET path = $1, updated_at = now() WHERE id = $2 AND path = $3",
    )
    .bind(to)
    .bind(item)
    .bind(from)
    .execute(&state.db)
    .await?;
    Ok(result.rows_affected() == 1)
}

enum RowPath {
    At(String),
    Gone,
    /// The database could not be reached.
    Unknown,
}

/// Where the database says an item is, retrying briefly. Used after an update
/// whose outcome is unclear (the commit may have landed before the error).
async fn row_path(state: &AppState, item: Uuid) -> RowPath {
    for attempt in 0..3 {
        if attempt > 0 {
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        }
        let result: Result<Option<String>, _> =
            sqlx::query_scalar("SELECT path FROM inventory_items WHERE id = $1")
                .bind(item)
                .fetch_optional(&state.db)
                .await;
        match result {
            Ok(Some(path)) => return RowPath::At(path),
            Ok(None) => return RowPath::Gone,
            Err(e) => tracing::warn!(error = %e, "re-reading inventory path failed"),
        }
    }
    RowPath::Unknown
}

const INTERRUPTED: &str =
    "the database became unreachable mid-batch; it will be reconciled before the next batch or on restart";

/// After a file moved on disk, record it in the database. If that fails,
/// find out what the database really holds before deciding to move back.
async fn settle_after_move(
    state: &AppState,
    m: &PlannedMove,
    from: &FsPath,
    to: &FsPath,
) -> ApiResult<(&'static str, Option<String>)> {
    let reason = match update_path(state, m.inventory_item_id, &m.from_path, &m.to_path).await {
        Ok(true) => return Ok(("moved", None)),
        Ok(false) => "the inventory record changed during the move".to_string(),
        Err(e) => format!("the database update failed ({e})"),
    };
    let reason = match row_path(state, m.inventory_item_id).await {
        // The update did land; only its reply was lost.
        RowPath::At(path) if path == m.to_path => return Ok(("moved", None)),
        RowPath::Unknown => return Err(ApiError::internal(INTERRUPTED)),
        RowPath::Gone => "the inventory record was deleted during the move".to_string(),
        RowPath::At(_) => reason,
    };
    let message = match fs_move(to.to_path_buf(), from.to_path_buf()).await {
        Ok(()) => format!("{reason}; file moved back"),
        Err(e) => format!("{reason}; moving back failed ({e}); file is at the target"),
    };
    Ok(("failed", Some(message)))
}

async fn execute(
    State(state): State<AppState>,
    Json(body): Json<SelectionRequest>,
) -> ApiResult<Json<Batch>> {
    if body.inventory_item_ids.is_empty() {
        return Err(ApiError::bad_request("select at least one file"));
    }
    // Detached: a dropped connection must not stop a batch between a move and
    // its database record. The task keeps running if this handler is dropped.
    let task = tokio::spawn(run_execute(state.clone(), body.inventory_item_ids));
    let batch_id = task.await.map_err(ApiError::internal)??;
    Ok(Json(load_batch(&state, batch_id, true).await?))
}

async fn run_execute(state: AppState, ids: Vec<Uuid>) -> ApiResult<Uuid> {
    let _lock = state.library_lock.lock().await;
    reconcile_interrupted(&state).await?;
    // Recompute from the database; never trust paths from the client.
    let moves = compute_plan(&state, Some(&ids)).await?;
    if moves.is_empty() {
        return Err(ApiError::bad_request("none of the selected files needs to move"));
    }
    let guard = prune_guard(&state).await?;

    let batch_id = Uuid::new_v4();
    sqlx::query("INSERT INTO move_batches (id, status) VALUES ($1, 'running')")
        .bind(batch_id)
        .execute(&state.db)
        .await?;

    let (mut moved, mut skipped, mut failed) = (0, 0, 0);
    let mut leftover_dirs: Vec<PathBuf> = Vec::new();
    for (seq, m) in moves.iter().enumerate() {
        let seq = seq as i32;
        if let Some(conflict) = &m.conflict {
            skipped += 1;
            insert_op(&state, batch_id, seq, m, "skipped", Some(conflict.clone())).await?;
            continue;
        }
        let from = absolute(&state, &m.from_path)?;
        let to = absolute(&state, &m.to_path)?;
        // Logged before the file is touched, so an interruption is recoverable.
        let op_id = insert_op(&state, batch_id, seq, m, "pending", None).await?;
        let (status, message) = match fs_move(from.clone(), to.clone()).await {
            Ok(()) => settle_after_move(&state, m, &from, &to).await?,
            Err(MoveError::TargetExists) => ("skipped", Some(MoveError::TargetExists.to_string())),
            Err(e) => ("failed", Some(e.to_string())),
        };
        match status {
            "moved" => {
                moved += 1;
                if let Some(parent) = from.parent() {
                    leftover_dirs.push(parent.to_path_buf());
                }
            }
            "skipped" => skipped += 1,
            _ => {
                failed += 1;
                // The writer may have created empty target folders.
                if let Some(parent) = to.parent() {
                    leftover_dirs.push(parent.to_path_buf());
                }
            }
        }
        set_op(&state, op_id, status, message).await?;
    }

    leftover_dirs.sort();
    leftover_dirs.dedup();
    let (removed, notes) = prune_all(guard, leftover_dirs).await;
    let removed: Vec<String> = removed.iter().map(|d| relative(&state, d)).collect();

    sqlx::query(
        "UPDATE move_batches
            SET status = 'done', moved = $1, skipped = $2, failed = $3,
                removed_dirs = $4, notes = $5, finished_at = now()
          WHERE id = $6",
    )
    .bind(moved)
    .bind(skipped)
    .bind(failed)
    .bind(serde_json::json!(removed))
    .bind(serde_json::json!(notes))
    .bind(batch_id)
    .execute(&state.db)
    .await?;
    Ok(batch_id)
}

async fn undo(State(state): State<AppState>, Path(id): Path<Uuid>) -> ApiResult<Json<Batch>> {
    let task = tokio::spawn(run_undo(state.clone(), id));
    task.await.map_err(ApiError::internal)??;
    Ok(Json(load_batch(&state, id, true).await?))
}

async fn run_undo(state: AppState, id: Uuid) -> ApiResult<()> {
    let _lock = state.library_lock.lock().await;
    reconcile_interrupted(&state).await?;
    let guard = prune_guard(&state).await?;
    let ops = sqlx::query(
        "SELECT id, inventory_item_id, from_path, to_path
           FROM move_ops
          WHERE batch_id = $1 AND status = 'moved'
          ORDER BY seq DESC",
    )
    .bind(id)
    .fetch_all(&state.db)
    .await?;

    // Claimed last, once everything that can fail without side effects has run.
    let claimed = sqlx::query(
        "UPDATE move_batches SET status = 'undoing' WHERE id = $1 AND status = 'done'",
    )
    .bind(id)
    .execute(&state.db)
    .await?;
    if claimed.rows_affected() == 0 {
        load_batch(&state, id, false).await?; // 404 when it does not exist
        return Err(ApiError::conflict("only a finished batch can be undone"));
    }

    let mut notes: Vec<String> = Vec::new();
    let mut leftover_dirs: Vec<PathBuf> = Vec::new();
    for op in ops {
        let op_id: Uuid = op.get("id");
        let item: Option<Uuid> = op.get("inventory_item_id");
        let from_rel: String = op.get("from_path");
        let to_rel: String = op.get("to_path");
        set_op(&state, op_id, "undo_pending", None).await?;
        let (status, message) = undo_one(&state, item, &from_rel, &to_rel, &mut leftover_dirs).await?;
        if let Some(m) = &message {
            notes.push(format!("{to_rel}: {m}"));
        }
        set_op(&state, op_id, status, message).await?;
    }

    leftover_dirs.sort();
    leftover_dirs.dedup();
    let (removed, prune_notes) = prune_all(guard, leftover_dirs).await;
    notes.extend(prune_notes);
    let removed: Vec<String> = removed.iter().map(|d| relative(&state, d)).collect();

    sqlx::query(
        "UPDATE move_batches
            SET status = 'undone', undone_at = now(),
                removed_dirs = removed_dirs || $1, notes = notes || $2
          WHERE id = $3",
    )
    .bind(serde_json::json!(removed))
    .bind(serde_json::json!(notes))
    .bind(id)
    .execute(&state.db)
    .await?;
    Ok(())
}

/// Move one file back. Returns the op's new status and an optional message.
async fn undo_one(
    state: &AppState,
    item: Option<Uuid>,
    from_rel: &str,
    to_rel: &str,
    leftover_dirs: &mut Vec<PathBuf>,
) -> ApiResult<(&'static str, Option<String>)> {
    let Some(item) = item else {
        return Ok(("undo_skipped", Some("inventory record was deleted".to_string())));
    };
    let current: Option<String> = sqlx::query_scalar("SELECT path FROM inventory_items WHERE id = $1")
        .bind(item)
        .fetch_optional(&state.db)
        .await?;
    if current.as_deref() != Some(to_rel) {
        return Ok(("undo_skipped", Some("file has moved since this batch".to_string())));
    }
    let from = absolute(state, from_rel)?;
    let to = absolute(state, to_rel)?;
    match fs_move(to.clone(), from.clone()).await {
        Ok(()) => {
            let reason = match update_path(state, item, to_rel, from_rel).await {
                Ok(true) => None,
                Ok(false) => Some("the inventory record changed during the undo".to_string()),
                Err(e) => Some(format!("the database update failed ({e})")),
            };
            let reason = match reason {
                None => None,
                Some(reason) => match row_path(state, item).await {
                    RowPath::At(path) if path == from_rel => None,
                    RowPath::Unknown => return Err(ApiError::internal(INTERRUPTED)),
                    _ => Some(reason),
                },
            };
            match reason {
                None => {
                    if let Some(parent) = to.parent() {
                        leftover_dirs.push(parent.to_path_buf());
                    }
                    Ok(("undone", None))
                }
                Some(reason) => {
                    let message = match fs_move(from, to).await {
                        Ok(()) => format!("{reason}; file left at the batch target"),
                        Err(e) => format!("{reason}; re-moving failed ({e})"),
                    };
                    Ok(("undo_failed", Some(message)))
                }
            }
        }
        Err(MoveError::TargetExists) => Ok((
            "undo_skipped",
            Some("the original location is taken".to_string()),
        )),
        Err(e) => Ok(("undo_failed", Some(e.to_string()))),
    }
}

// ---- reconciliation -------------------------------------------------------

/// Settle batches a crash, restart or lost database connection left in
/// `running` or `undoing`, by checking each pending op against the disk.
/// Takes the library lock; called once at startup.
pub async fn reconcile_at_startup(state: &AppState) -> ApiResult<usize> {
    let _lock = state.library_lock.lock().await;
    reconcile_interrupted(state).await
}

/// Same as `reconcile_at_startup`, for callers already holding the lock.
async fn reconcile_interrupted(state: &AppState) -> ApiResult<usize> {
    let batches = sqlx::query(
        "SELECT id, status FROM move_batches WHERE status IN ('running', 'undoing')",
    )
    .fetch_all(&state.db)
    .await?;

    for batch in &batches {
        let batch_id: Uuid = batch.get("id");
        let batch_status: String = batch.get("status");
        let ops = sqlx::query(
            "SELECT id, inventory_item_id, from_path, to_path, status
               FROM move_ops
              WHERE batch_id = $1 AND status IN ('pending', 'undo_pending')",
        )
        .bind(batch_id)
        .fetch_all(&state.db)
        .await?;
        for op in ops {
            reconcile_op(state, &op).await?;
        }

        if batch_status == "running" {
            sqlx::query(
                "UPDATE move_batches SET
                    status      = 'done',
                    moved       = (SELECT count(*) FROM move_ops WHERE batch_id = $1 AND status = 'moved'),
                    skipped     = (SELECT count(*) FROM move_ops WHERE batch_id = $1 AND status = 'skipped'),
                    failed      = (SELECT count(*) FROM move_ops WHERE batch_id = $1 AND status = 'failed'),
                    finished_at = COALESCE(finished_at, now()),
                    notes       = notes || '[\"the batch was interrupted and reconciled against the disk\"]'
                  WHERE id = $1",
            )
            .bind(batch_id)
            .execute(&state.db)
            .await?;
        } else {
            // Files still at their batch target can be undone again.
            sqlx::query(
                "UPDATE move_batches SET
                    status = CASE WHEN EXISTS (SELECT 1 FROM move_ops
                                                WHERE batch_id = $1 AND status = 'moved')
                                  THEN 'done' ELSE 'undone' END,
                    undone_at = COALESCE(undone_at, now()),
                    notes = notes || '[\"the undo was interrupted and reconciled; files still at their target can be undone again\"]'
                  WHERE id = $1",
            )
            .bind(batch_id)
            .execute(&state.db)
            .await?;
        }
        tracing::warn!(batch = %batch_id, status = %batch_status, "reconciled an interrupted move batch");
    }
    Ok(batches.len())
}

async fn reconcile_op(state: &AppState, op: &sqlx::postgres::PgRow) -> ApiResult<()> {
    let op_id: Uuid = op.get("id");
    let item: Option<Uuid> = op.get("inventory_item_id");
    let from_rel: String = op.get("from_path");
    let to_rel: String = op.get("to_path");
    let status: String = op.get("status");
    let from = absolute(state, &from_rel)?;
    let to = absolute(state, &to_rel)?;
    let (from_exists, to_exists, same) = {
        let (from, to) = (from.clone(), to.clone());
        tokio::task::spawn_blocking(move || {
            (
                std::fs::symlink_metadata(&from).is_ok(),
                std::fs::symlink_metadata(&to).is_ok(),
                same_file(&from, &to),
            )
        })
        .await
        .map_err(ApiError::internal)?
    };
    let row: Option<String> = match item {
        Some(item) => {
            sqlx::query_scalar("SELECT path FROM inventory_items WHERE id = $1")
                .bind(item)
                .fetch_optional(&state.db)
                .await?
        }
        None => None,
    };

    // Where the file really is decides the op; the row follows the file.
    let (new_status, message, row_should_be) = if status == "pending" {
        if to_exists && !from_exists {
            ("moved", "recovered after an interrupted batch", Some(&to_rel))
        } else {
            if from_exists && to_exists && same {
                // The hard link was made but the source not yet removed.
                let _ = tokio::fs::remove_file(&to).await;
            }
            if from_exists {
                ("failed", "interrupted before the move finished; file left at its original path", Some(&from_rel))
            } else {
                ("failed", "interrupted; file found at neither path", None)
            }
        }
    } else if from_exists && !to_exists {
        ("undone", "recovered after an interrupted undo", Some(&from_rel))
    } else {
        if from_exists && to_exists && same {
            let _ = tokio::fs::remove_file(&from).await;
        }
        ("moved", "undo interrupted; file still at the batch target", Some(&to_rel))
    };

    if let (Some(item), Some(target), Some(current)) = (item, row_should_be, row.as_ref()) {
        if current != target {
            let other = if target == &to_rel { &from_rel } else { &to_rel };
            update_path(state, item, other, target).await?;
        }
    }
    set_op(state, op_id, new_status, Some(message.to_string())).await
}

// ---- inbox ----------------------------------------------------------------

#[derive(Serialize)]
struct InboxStatus {
    folder: &'static str,
    exists: bool,
}

async fn inbox_status(State(state): State<AppState>) -> Json<InboxStatus> {
    let path = state.library_root.root().join(INBOX_FOLDER);
    let exists = tokio::fs::metadata(&path).await.is_ok_and(|m| m.is_dir());
    Json(InboxStatus {
        folder: INBOX_FOLDER,
        exists,
    })
}

async fn create_inbox(State(state): State<AppState>) -> ApiResult<Json<InboxStatus>> {
    let _lock = state.library_lock.lock().await;
    let path = state.library_root.root().join(INBOX_FOLDER);
    tokio::fs::create_dir_all(&path)
        .await
        .map_err(ApiError::internal)?;
    Ok(Json(InboxStatus {
        folder: INBOX_FOLDER,
        exists: true,
    }))
}

// ---- other files ----------------------------------------------------------

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct OtherFilesResponse {
    entries: Vec<OtherEntry>,
    total_files: u64,
    total_bytes: u64,
}

/// Read-only report of everything the scanner ignores (not zip or rar), so
/// nothing is silently left behind in emptied legacy folders.
async fn other_files(State(state): State<AppState>) -> ApiResult<Json<OtherFilesResponse>> {
    let root = state.library_root.root().to_path_buf();
    let exclude = vec![state.asset_cache_root.clone()];
    let entries = tokio::task::spawn_blocking(move || report_other_files(&root, &exclude))
        .await
        .map_err(ApiError::internal)?
        .map_err(ApiError::internal)?;
    Ok(Json(OtherFilesResponse {
        total_files: entries.iter().map(|e| e.files).sum(),
        total_bytes: entries.iter().map(|e| e.bytes).sum(),
        entries,
    }))
}
