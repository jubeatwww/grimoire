use std::sync::Arc;

use super::MoveBatchOutput;
use crate::application::organization::{
    MovePlanAppService, MovePlanError, MoveReconciliationAppService, MoveReconciliationError,
    MoveSettlementAppService, MoveSettlementError,
};
use crate::domain::library_folders::{LibraryFolders, PruneStarts};
use crate::domain::library_lock_registry::LibraryLockRegistry;
use crate::domain::move_batch::{
    MoveBatch, MoveBatchError, MoveBatchId, MoveBatchRepository, MoveBatchRepositoryError,
};
use crate::kernel::{Clock, ErrorCode, ErrorKind};

#[derive(Debug)]
pub(crate) struct UndoMoveBatchInput {
    pub(crate) batch_id: MoveBatchId,
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum UndoMoveBatchError {
    #[error("no such batch")]
    NotFound(MoveBatchId),
    #[error(transparent)]
    Batch(#[from] MoveBatchError),
    #[error(transparent)]
    Settlement(#[from] MoveSettlementError),
    #[error(transparent)]
    Plan(#[from] MovePlanError),
    #[error(transparent)]
    Reconciliation(#[from] MoveReconciliationError),
    #[error(transparent)]
    Batches(#[from] MoveBatchRepositoryError),
    #[error("the undo task failed: {0}")]
    Task(String),
}

impl ErrorCode for UndoMoveBatchError {
    fn code(&self) -> &'static str {
        match self {
            Self::NotFound(_) => "move_batch.not_found",
            Self::Batch(e) => e.code(),
            Self::Settlement(_) => "move_batch.interrupted",
            Self::Plan(_) | Self::Reconciliation(_) | Self::Batches(_) => "grimoire.unavailable",
            Self::Task(_) => "move_batch.task_failed",
        }
    }

    fn kind(&self) -> ErrorKind {
        match self {
            Self::NotFound(_) => ErrorKind::NotFound,
            Self::Batch(e) => e.kind(),
            Self::Settlement(_) | Self::Plan(_) | Self::Reconciliation(_) | Self::Batches(_) => {
                ErrorKind::Unavailable
            }
            Self::Task(_) => ErrorKind::Internal,
        }
    }
}

/// Moves a finished batch's files back, newest first. A file that moved again since, or whose
/// original place is taken, stays where it is.
pub(crate) struct UndoMoveBatchUseCase {
    lock: Arc<dyn LibraryLockRegistry>,
    plans: Arc<MovePlanAppService>,
    settlement: Arc<MoveSettlementAppService>,
    reconciliation: Arc<MoveReconciliationAppService>,
    batches: Arc<dyn MoveBatchRepository>,
    folders: Arc<dyn LibraryFolders>,
    clock: Arc<dyn Clock>,
}

impl UndoMoveBatchUseCase {
    pub(crate) fn new(
        lock: Arc<dyn LibraryLockRegistry>,
        plans: Arc<MovePlanAppService>,
        settlement: Arc<MoveSettlementAppService>,
        reconciliation: Arc<MoveReconciliationAppService>,
        batches: Arc<dyn MoveBatchRepository>,
        folders: Arc<dyn LibraryFolders>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            lock,
            plans,
            settlement,
            reconciliation,
            batches,
            folders,
            clock,
        }
    }

    /// Runs detached, like a run: a dropped connection must not stop the undo halfway.
    #[tracing::instrument(name = "use_case.undo_move_batch", skip(self))]
    pub(crate) async fn execute(
        self: Arc<Self>,
        input: UndoMoveBatchInput,
    ) -> Result<MoveBatchOutput, UndoMoveBatchError> {
        tokio::spawn(async move { self.run(input.batch_id).await })
            .await
            .map_err(|e| UndoMoveBatchError::Task(e.to_string()))?
    }

    async fn run(&self, id: MoveBatchId) -> Result<MoveBatchOutput, UndoMoveBatchError> {
        let _lease = self.lock.acquire().await;
        self.reconciliation.reconcile().await?;
        let protected = self.plans.protected_folders().await?;
        let mut batch = self
            .batches
            .find_by_id(id)
            .await?
            .ok_or(UndoMoveBatchError::NotFound(id))?;
        let seqs = batch.moved_ops_newest_first();
        // Claimed last, once everything that can fail without side effects has run.
        let _undoing = batch.start_undo()?;
        self.batches.save(&batch).await?;

        let mut leftovers = Vec::new();
        for seq in seqs {
            let _logged = batch.log_undo(seq)?;
            self.record(&batch, seq).await?;
            let op = batch.op(seq)?.clone();
            let outcome = self.settlement.undo(&op).await?;
            leftovers.extend(outcome.folder_to_prune(&op));
            let _settled = batch.settle_undo(seq, outcome)?;
            self.record(&batch, seq).await?;
        }

        let pruned = self
            .folders
            .prune(PruneStarts::new(leftovers), &protected)
            .await;
        let (removed, notes) = pruned.into_parts();
        let _undone = batch.finish_undo(removed, notes, self.clock.now())?;
        self.batches.save(&batch).await?;
        Ok(MoveBatchOutput::from(&batch))
    }

    async fn record(&self, batch: &MoveBatch, seq: i32) -> Result<(), UndoMoveBatchError> {
        self.batches.record_op(batch.id(), batch.op(seq)?).await?;
        Ok(())
    }
}
