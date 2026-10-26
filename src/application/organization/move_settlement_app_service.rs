use std::sync::Arc;
use std::time::Duration;

use crate::domain::inventory_item::{InventoryItemId, InventoryItemRepository};
use crate::domain::library_files::{FileMoveError, LibraryFiles};
use crate::domain::library_path::LibraryPath;
use crate::domain::move_batch::{MoveOp, MoveOutcome, UndoOutcome};
use crate::domain::move_plan::PlannedMove;

const REREAD_ATTEMPTS: u32 = 3;
const REREAD_PAUSE: Duration = Duration::from_millis(500);

#[derive(Debug, thiserror::Error)]
pub(crate) enum MoveSettlementError {
    /// The database could not be reached while a file was between two places. The batch is
    /// left as it is, for the reconciliation that runs before the next batch or on restart.
    #[error(
        "the database became unreachable mid-batch; it will be reconciled before the next batch or on restart"
    )]
    Interrupted,
}

/// Where the database says an item is.
enum RecordedPath {
    At(LibraryPath),
    Gone,
    /// The database could not be reached.
    Unknown,
}

/// Keeping a file and its inventory record together around a move or an undo. When the record
/// cannot follow the file, the file goes back.
pub(crate) struct MoveSettlementAppService {
    inventory: Arc<dyn InventoryItemRepository>,
    files: Arc<dyn LibraryFiles>,
}

impl MoveSettlementAppService {
    pub(crate) fn new(
        inventory: Arc<dyn InventoryItemRepository>,
        files: Arc<dyn LibraryFiles>,
    ) -> Self {
        Self { inventory, files }
    }

    /// Where the record is, retrying briefly. Used after an update whose outcome is unclear
    /// (the commit may have landed before the error).
    async fn recorded_path(&self, item: InventoryItemId) -> RecordedPath {
        for attempt in 0..REREAD_ATTEMPTS {
            if attempt > 0 {
                tokio::time::sleep(REREAD_PAUSE).await;
            }
            match self.inventory.find_by_id(item).await {
                Ok(Some(found)) => return RecordedPath::At(found.path().clone()),
                Ok(None) => return RecordedPath::Gone,
                Err(e) => tracing::warn!(error = %e, "re-reading inventory path failed"),
            }
        }
        RecordedPath::Unknown
    }

    /// Moves one planned file and points its record at the new path. If the record cannot
    /// follow, finds out what the database really holds before deciding to move the file back.
    pub(crate) async fn move_file(
        &self,
        planned: &PlannedMove,
    ) -> Result<MoveOutcome, MoveSettlementError> {
        let (item, from, to) = (planned.item_id(), planned.from(), planned.to());
        match self.files.move_file(from, to).await {
            Ok(()) => {}
            Err(e @ FileMoveError::TargetExists) => return Ok(MoveOutcome::Skipped(e.to_string())),
            Err(e) => return Ok(MoveOutcome::Failed(e.to_string())),
        }
        let reason = match self.inventory.relocate(item, from, to).await {
            Ok(true) => return Ok(MoveOutcome::Moved),
            Ok(false) => "the inventory record changed during the move".to_string(),
            Err(e) => format!("the database update failed ({e})"),
        };
        let reason = match self.recorded_path(item).await {
            // The update did land; only its reply was lost.
            RecordedPath::At(path) if &path == to => return Ok(MoveOutcome::Moved),
            RecordedPath::Unknown => return Err(MoveSettlementError::Interrupted),
            RecordedPath::Gone => "the inventory record was deleted during the move".to_string(),
            RecordedPath::At(_) => reason,
        };
        let message = match self.files.move_file(to, from).await {
            Ok(()) => format!("{reason}; file moved back"),
            Err(e) => format!("{reason}; moving back failed ({e}); file is at the target"),
        };
        Ok(MoveOutcome::Failed(message))
    }

    /// Moves one file of a finished batch back to where it came from.
    pub(crate) async fn undo(&self, op: &MoveOp) -> Result<UndoOutcome, MoveSettlementError> {
        let Some(item) = op.item_id() else {
            return Ok(UndoOutcome::Skipped(
                "inventory record was deleted".to_string(),
            ));
        };
        let current = self
            .inventory
            .find_by_id(item)
            .await
            .map_err(|_| MoveSettlementError::Interrupted)?;
        if current.as_ref().map(|i| i.path()) != Some(op.to()) {
            return Ok(UndoOutcome::Skipped(
                "file has moved since this batch".to_string(),
            ));
        }
        let (from, to) = (op.from(), op.to());
        match self.files.move_file(to, from).await {
            Ok(()) => {}
            Err(FileMoveError::TargetExists) => {
                return Ok(UndoOutcome::Skipped(
                    "the original location is taken".to_string(),
                ));
            }
            Err(e) => return Ok(UndoOutcome::Failed(e.to_string())),
        }
        let reason = match self.inventory.relocate(item, to, from).await {
            Ok(true) => None,
            Ok(false) => Some("the inventory record changed during the undo".to_string()),
            Err(e) => Some(format!("the database update failed ({e})")),
        };
        let reason = match reason {
            None => None,
            Some(reason) => match self.recorded_path(item).await {
                RecordedPath::At(path) if &path == from => None,
                RecordedPath::Unknown => return Err(MoveSettlementError::Interrupted),
                _ => Some(reason),
            },
        };
        let Some(reason) = reason else {
            return Ok(UndoOutcome::Undone);
        };
        let message = match self.files.move_file(from, to).await {
            Ok(()) => format!("{reason}; file left at the batch target"),
            Err(e) => format!("{reason}; re-moving failed ({e})"),
        };
        Ok(UndoOutcome::Failed(message))
    }
}
