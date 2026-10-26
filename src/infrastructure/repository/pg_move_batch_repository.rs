use std::collections::HashMap;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::types::Json;
use sqlx::{PgExecutor, PgPool};
use uuid::Uuid;

use crate::domain::inventory_item::InventoryItemId;
use crate::domain::library_path::LibraryPath;
use crate::domain::move_batch::{
    BatchNotes, BatchTimes, MoveBatch, MoveBatchId, MoveBatchRepository, MoveBatchRepositoryError,
    MoveBatchStatus, MoveCounts, MoveOp, MoveOpId, MoveOpStatus,
};
use crate::kernel::Timestamp;

pub(crate) struct PgMoveBatchRepository {
    pool: PgPool,
}

impl PgMoveBatchRepository {
    pub(crate) fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct MoveBatchRow {
    id: Uuid,
    status: String,
    moved: i32,
    skipped: i32,
    failed: i32,
    removed_dirs: Json<Vec<String>>,
    notes: Json<Vec<String>>,
    created_at: DateTime<Utc>,
    finished_at: Option<DateTime<Utc>>,
    undone_at: Option<DateTime<Utc>>,
}

#[derive(sqlx::FromRow)]
struct MoveOpRow {
    id: Uuid,
    batch_id: Uuid,
    seq: i32,
    inventory_item_id: Option<Uuid>,
    from_path: String,
    to_path: String,
    status: String,
    message: Option<String>,
}

fn batch_status(raw: &str) -> Option<MoveBatchStatus> {
    match raw {
        "running" => Some(MoveBatchStatus::Running),
        "done" => Some(MoveBatchStatus::Done),
        "undoing" => Some(MoveBatchStatus::Undoing),
        "undone" => Some(MoveBatchStatus::Undone),
        _ => None,
    }
}

fn op_status(raw: &str) -> Option<MoveOpStatus> {
    match raw {
        "pending" => Some(MoveOpStatus::Pending),
        "moved" => Some(MoveOpStatus::Moved),
        "skipped" => Some(MoveOpStatus::Skipped),
        "failed" => Some(MoveOpStatus::Failed),
        "undo_pending" => Some(MoveOpStatus::UndoPending),
        "undone" => Some(MoveOpStatus::Undone),
        "undo_skipped" => Some(MoveOpStatus::UndoSkipped),
        "undo_failed" => Some(MoveOpStatus::UndoFailed),
        _ => None,
    }
}

fn backend(e: sqlx::Error) -> MoveBatchRepositoryError {
    MoveBatchRepositoryError::Backend(Box::new(e))
}

fn corrupt(what: String) -> MoveBatchRepositoryError {
    MoveBatchRepositoryError::Backend(what.into())
}

impl TryFrom<MoveOpRow> for MoveOp {
    type Error = MoveBatchRepositoryError;

    fn try_from(row: MoveOpRow) -> Result<Self, Self::Error> {
        let path = |raw: &str| {
            LibraryPath::parse(raw).map_err(|e| corrupt(format!("move op {}: {e}", row.id)))
        };
        let status = op_status(&row.status).ok_or_else(|| {
            corrupt(format!(
                "move op {}: unknown status {:?}",
                row.id, row.status
            ))
        })?;
        Ok(MoveOp::from_persisted(
            MoveOpId::from_raw(row.id),
            row.seq,
            row.inventory_item_id.map(InventoryItemId::from_raw),
            path(&row.from_path)?,
            path(&row.to_path)?,
            status,
            row.message,
        ))
    }
}

fn batch_from(row: MoveBatchRow, ops: Vec<MoveOp>) -> Result<MoveBatch, MoveBatchRepositoryError> {
    let status = batch_status(&row.status).ok_or_else(|| {
        corrupt(format!(
            "move batch {}: unknown status {:?}",
            row.id, row.status
        ))
    })?;
    Ok(MoveBatch::from_persisted(
        MoveBatchId::from_raw(row.id),
        status,
        MoveCounts::new(row.moved, row.skipped, row.failed),
        BatchNotes::new(row.removed_dirs.0, row.notes.0),
        BatchTimes::new(
            Timestamp::from_utc(row.created_at),
            row.finished_at.map(Timestamp::from_utc),
            row.undone_at.map(Timestamp::from_utc),
        ),
        ops,
    ))
}

const BATCH_COLUMNS: &str =
    "id, status, moved, skipped, failed, removed_dirs, notes, created_at, finished_at, undone_at";
const OP_COLUMNS: &str =
    "id, batch_id, seq, inventory_item_id, from_path, to_path, status, message";

/// Only the outcome of an op ever changes. Its inventory link is left alone, so a link the
/// database already cleared (the record was deleted) is never restored.
async fn upsert_op<'e>(
    executor: impl PgExecutor<'e>,
    batch: MoveBatchId,
    op: &MoveOp,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO move_ops (id, batch_id, seq, inventory_item_id, from_path, to_path, status, message)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
         ON CONFLICT (id) DO UPDATE SET
            status = EXCLUDED.status, message = EXCLUDED.message, updated_at = now()",
    )
    .bind(op.id().raw())
    .bind(batch.raw())
    .bind(op.seq())
    .bind(op.item_id().map(InventoryItemId::raw))
    .bind(op.from().as_str())
    .bind(op.to().as_str())
    .bind(op.status().as_str())
    .bind(op.message())
    .execute(executor)
    .await?;
    Ok(())
}

impl PgMoveBatchRepository {
    /// The batches of these rows with their ops, in row order.
    async fn with_ops(
        &self,
        rows: Vec<MoveBatchRow>,
    ) -> Result<Vec<MoveBatch>, MoveBatchRepositoryError> {
        let ids: Vec<Uuid> = rows.iter().map(|r| r.id).collect();
        let op_rows: Vec<MoveOpRow> = sqlx::query_as(&format!(
            "SELECT {OP_COLUMNS} FROM move_ops WHERE batch_id = ANY($1) ORDER BY batch_id, seq"
        ))
        .bind(&ids)
        .fetch_all(&self.pool)
        .await
        .map_err(backend)?;
        let mut ops: HashMap<Uuid, Vec<MoveOp>> = HashMap::new();
        for row in op_rows {
            let batch_id = row.batch_id;
            ops.entry(batch_id)
                .or_default()
                .push(MoveOp::try_from(row)?);
        }
        rows.into_iter()
            .map(|row| {
                let batch_ops = ops.remove(&row.id).unwrap_or_default();
                batch_from(row, batch_ops)
            })
            .collect()
    }
}

#[async_trait]
impl MoveBatchRepository for PgMoveBatchRepository {
    async fn find_by_id(
        &self,
        id: MoveBatchId,
    ) -> Result<Option<MoveBatch>, MoveBatchRepositoryError> {
        let row: Option<MoveBatchRow> = sqlx::query_as(&format!(
            "SELECT {BATCH_COLUMNS} FROM move_batches WHERE id = $1"
        ))
        .bind(id.raw())
        .fetch_optional(&self.pool)
        .await
        .map_err(backend)?;
        let Some(row) = row else { return Ok(None) };
        Ok(self.with_ops(vec![row]).await?.pop())
    }

    async fn list_recent(&self, limit: i64) -> Result<Vec<MoveBatch>, MoveBatchRepositoryError> {
        let rows: Vec<MoveBatchRow> = sqlx::query_as(&format!(
            "SELECT {BATCH_COLUMNS} FROM move_batches ORDER BY created_at DESC LIMIT $1"
        ))
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(backend)?;
        self.with_ops(rows).await
    }

    async fn list_interrupted(
        &self,
        limit: i64,
    ) -> Result<Vec<MoveBatch>, MoveBatchRepositoryError> {
        let rows: Vec<MoveBatchRow> = sqlx::query_as(&format!(
            "SELECT {BATCH_COLUMNS} FROM move_batches
              WHERE status IN ('running', 'undoing')
              ORDER BY created_at LIMIT $1"
        ))
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(backend)?;
        self.with_ops(rows).await
    }

    async fn save(&self, batch: &MoveBatch) -> Result<(), MoveBatchRepositoryError> {
        let counts = batch.counts();
        let times = batch.times();
        let mut tx = self.pool.begin().await.map_err(backend)?;
        sqlx::query(
            "INSERT INTO move_batches (id, status, moved, skipped, failed, removed_dirs, notes,
                                       created_at, finished_at, undone_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
             ON CONFLICT (id) DO UPDATE SET
                status       = EXCLUDED.status,
                moved        = EXCLUDED.moved,
                skipped      = EXCLUDED.skipped,
                failed       = EXCLUDED.failed,
                removed_dirs = EXCLUDED.removed_dirs,
                notes        = EXCLUDED.notes,
                finished_at  = EXCLUDED.finished_at,
                undone_at    = EXCLUDED.undone_at",
        )
        .bind(batch.id().raw())
        .bind(batch.status().as_str())
        .bind(counts.moved())
        .bind(counts.skipped())
        .bind(counts.failed())
        .bind(Json(batch.notes().removed_dirs()))
        .bind(Json(batch.notes().notes()))
        .bind(times.created_at().as_utc())
        .bind(times.finished_at().map(Timestamp::as_utc))
        .bind(times.undone_at().map(Timestamp::as_utc))
        .execute(&mut *tx)
        .await
        .map_err(backend)?;
        for op in batch.ops() {
            upsert_op(&mut *tx, batch.id(), op).await.map_err(backend)?;
        }
        tx.commit().await.map_err(backend)
    }

    async fn record_op(
        &self,
        batch: MoveBatchId,
        op: &MoveOp,
    ) -> Result<(), MoveBatchRepositoryError> {
        upsert_op(&self.pool, batch, op).await.map_err(backend)
    }
}
