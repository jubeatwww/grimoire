use serde::Serialize;

use crate::domain::inventory_item::InventoryItemId;
use crate::domain::move_batch::{MoveBatch, MoveBatchId, MoveBatchStatus, MoveOp, MoveOpStatus};
use crate::kernel::Timestamp;

#[derive(Debug, Serialize)]
pub(crate) struct ListMoveBatchesOutput {
    batches: Vec<MoveBatchListItemOutput>,
}

impl From<&[MoveBatch]> for ListMoveBatchesOutput {
    fn from(batches: &[MoveBatch]) -> Self {
        Self {
            batches: batches.iter().map(MoveBatchListItemOutput::from).collect(),
        }
    }
}

/// A batch in the history list, without its ops.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct MoveBatchListItemOutput {
    id: MoveBatchId,
    status: MoveBatchStatus,
    moved: i32,
    skipped: i32,
    failed: i32,
    removed_dirs: Vec<String>,
    notes: Vec<String>,
    created_at: Timestamp,
    finished_at: Option<Timestamp>,
    undone_at: Option<Timestamp>,
}

impl From<&MoveBatch> for MoveBatchListItemOutput {
    fn from(batch: &MoveBatch) -> Self {
        let counts = batch.counts();
        let times = batch.times();
        Self {
            id: batch.id(),
            status: batch.status(),
            moved: counts.moved(),
            skipped: counts.skipped(),
            failed: counts.failed(),
            removed_dirs: batch.notes().removed_dirs().to_vec(),
            notes: batch.notes().notes().to_vec(),
            created_at: times.created_at(),
            finished_at: times.finished_at(),
            undone_at: times.undone_at(),
        }
    }
}

/// A batch with every op.
#[derive(Debug, Serialize)]
pub(crate) struct MoveBatchOutput {
    #[serde(flatten)]
    batch: MoveBatchListItemOutput,
    ops: Vec<MoveOpOutput>,
}

impl From<&MoveBatch> for MoveBatchOutput {
    fn from(batch: &MoveBatch) -> Self {
        Self {
            batch: MoveBatchListItemOutput::from(batch),
            ops: batch.ops().iter().map(MoveOpOutput::from).collect(),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct MoveOpOutput {
    seq: i32,
    inventory_item_id: Option<InventoryItemId>,
    from_path: String,
    to_path: String,
    status: MoveOpStatus,
    message: Option<String>,
}

impl From<&MoveOp> for MoveOpOutput {
    fn from(op: &MoveOp) -> Self {
        Self {
            seq: op.seq(),
            inventory_item_id: op.item_id(),
            from_path: op.from().to_string(),
            to_path: op.to().to_string(),
            status: op.status(),
            message: op.message().map(str::to_owned),
        }
    }
}
