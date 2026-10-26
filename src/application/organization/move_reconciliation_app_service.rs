use std::sync::Arc;

use crate::domain::inventory_item::{InventoryItemRepository, InventoryItemRepositoryError};
use crate::domain::library_files::{LibraryFiles, LibraryFilesError};
use crate::domain::move_batch::{
    MoveBatch, MoveBatchError, MoveBatchRepository, MoveBatchRepositoryError,
};
use crate::kernel::Clock;

/// Interrupted batches are rare; this only guards against a runaway table.
const INTERRUPTED_LIMIT: i64 = 1000;

#[derive(Debug, thiserror::Error)]
pub(crate) enum MoveReconciliationError {
    #[error(transparent)]
    Batches(#[from] MoveBatchRepositoryError),
    #[error(transparent)]
    Inventory(#[from] InventoryItemRepositoryError),
    #[error(transparent)]
    Files(#[from] LibraryFilesError),
    #[error(transparent)]
    Batch(#[from] MoveBatchError),
}

/// Settles batches a crash, restart or lost database connection left running or undoing, by
/// checking each pending op against the disk. Runs at startup and before every run or undo; the
/// caller holds the library lock.
pub(crate) struct MoveReconciliationAppService {
    batches: Arc<dyn MoveBatchRepository>,
    inventory: Arc<dyn InventoryItemRepository>,
    files: Arc<dyn LibraryFiles>,
    clock: Arc<dyn Clock>,
}

impl MoveReconciliationAppService {
    pub(crate) fn new(
        batches: Arc<dyn MoveBatchRepository>,
        inventory: Arc<dyn InventoryItemRepository>,
        files: Arc<dyn LibraryFiles>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            batches,
            inventory,
            files,
            clock,
        }
    }

    /// Returns how many batches were settled.
    pub(crate) async fn reconcile(&self) -> Result<usize, MoveReconciliationError> {
        let batches = self.batches.list_interrupted(INTERRUPTED_LIMIT).await?;
        let count = batches.len();
        for mut batch in batches {
            let status = batch.status();
            for seq in batch.interrupted_ops() {
                self.reconcile_op(&mut batch, seq).await?;
            }
            let _recovered = batch.recover(self.clock.now());
            self.batches.save(&batch).await?;
            tracing::warn!(batch = %batch.id(), status = status.as_str(), "reconciled an interrupted move batch");
        }
        Ok(count)
    }

    async fn reconcile_op(
        &self,
        batch: &mut MoveBatch,
        seq: i32,
    ) -> Result<(), MoveReconciliationError> {
        let op = batch.op(seq)?.clone();
        let probe = self.files.probe_move(op.from(), op.to()).await?;
        let (recovery, _reconciled) = batch.reconcile_op(seq, probe)?;
        if let Some(link) = recovery.remove_link() {
            self.files.remove_link(link).await;
        }
        if let Some(item) = op.item_id()
            && let Some(current) = self.inventory.find_by_id(item).await?
            && let Some((from, to)) = recovery.record_move(current.path())
        {
            self.inventory.relocate(item, from, to).await?;
        }
        self.batches.record_op(batch.id(), batch.op(seq)?).await?;
        Ok(())
    }
}
