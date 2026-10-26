use std::sync::Arc;

use serde::Deserialize;

use super::MoveBatchOutput;
use crate::application::organization::{
    MovePlanAppService, MovePlanError, MoveReconciliationAppService, MoveReconciliationError,
    MoveSettlementAppService, MoveSettlementError,
};
use crate::domain::library_folders::{LibraryFolders, PruneStarts};
use crate::domain::library_lock_registry::LibraryLockRegistry;
use crate::domain::move_batch::{
    MoveBatch, MoveBatchError, MoveBatchRepository, MoveBatchRepositoryError,
};
use crate::domain::move_plan::MoveSelection;
use crate::kernel::{Clock, ErrorCode, ErrorKind, IdGenerator};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ExecuteMovesInput {
    pub(crate) inventory_item_ids: MoveSelection,
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum ExecuteMovesError {
    #[error("none of the selected files needs to move")]
    NothingToMove,
    #[error(transparent)]
    Settlement(#[from] MoveSettlementError),
    #[error(transparent)]
    Plan(#[from] MovePlanError),
    #[error(transparent)]
    Reconciliation(#[from] MoveReconciliationError),
    #[error(transparent)]
    Batches(#[from] MoveBatchRepositoryError),
    #[error(transparent)]
    Batch(#[from] MoveBatchError),
    #[error("the batch task failed: {0}")]
    Task(String),
}

impl ErrorCode for ExecuteMovesError {
    fn code(&self) -> &'static str {
        match self {
            Self::NothingToMove => "move_batch.nothing_to_move",
            Self::Settlement(_) => "move_batch.interrupted",
            Self::Batch(e) => e.code(),
            Self::Plan(_) | Self::Reconciliation(_) | Self::Batches(_) => "grimoire.unavailable",
            Self::Task(_) => "move_batch.task_failed",
        }
    }

    fn kind(&self) -> ErrorKind {
        match self {
            Self::NothingToMove => ErrorKind::Invalid,
            Self::Settlement(_) | Self::Plan(_) | Self::Reconciliation(_) | Self::Batches(_) => {
                ErrorKind::Unavailable
            }
            Self::Batch(e) => e.kind(),
            Self::Task(_) => ErrorKind::Internal,
        }
    }
}

/// Moves the selected files where they belong, logging each one before its file is touched,
/// then prunes the folders they left.
pub(crate) struct ExecuteMovesUseCase {
    lock: Arc<dyn LibraryLockRegistry>,
    plans: Arc<MovePlanAppService>,
    settlement: Arc<MoveSettlementAppService>,
    reconciliation: Arc<MoveReconciliationAppService>,
    batches: Arc<dyn MoveBatchRepository>,
    folders: Arc<dyn LibraryFolders>,
    ids: Arc<dyn IdGenerator>,
    clock: Arc<dyn Clock>,
}

impl ExecuteMovesUseCase {
    #[allow(
        clippy::too_many_arguments,
        reason = "one collaborator per capability a run touches"
    )]
    pub(crate) fn new(
        lock: Arc<dyn LibraryLockRegistry>,
        plans: Arc<MovePlanAppService>,
        settlement: Arc<MoveSettlementAppService>,
        reconciliation: Arc<MoveReconciliationAppService>,
        batches: Arc<dyn MoveBatchRepository>,
        folders: Arc<dyn LibraryFolders>,
        ids: Arc<dyn IdGenerator>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            lock,
            plans,
            settlement,
            reconciliation,
            batches,
            folders,
            ids,
            clock,
        }
    }

    /// Runs detached: a dropped connection must not stop a batch between a move and its record,
    /// so the run keeps going if the caller goes away. Hence `self: Arc<Self>`.
    #[tracing::instrument(name = "use_case.execute_moves", skip(self))]
    pub(crate) async fn execute(
        self: Arc<Self>,
        input: ExecuteMovesInput,
    ) -> Result<MoveBatchOutput, ExecuteMovesError> {
        tokio::spawn(async move { self.run(input.inventory_item_ids).await })
            .await
            .map_err(|e| ExecuteMovesError::Task(e.to_string()))?
    }

    async fn run(&self, selection: MoveSelection) -> Result<MoveBatchOutput, ExecuteMovesError> {
        let _lease = self.lock.acquire().await;
        self.reconciliation.reconcile().await?;
        let plan = self.plans.plan(Some(selection.ids())).await?;
        if plan.is_empty() {
            return Err(ExecuteMovesError::NothingToMove);
        }
        let protected = self.plans.protected_folders().await?;

        let (mut batch, _started) = MoveBatch::start(self.ids.next(), self.clock.now());
        self.batches.save(&batch).await?;
        let mut leftovers = Vec::new();
        for planned in plan.moves() {
            if let Some(conflict) = planned.conflict() {
                let logged =
                    batch.log_skip(self.ids.next(), planned, conflict.message().to_owned())?;
                self.record(&batch, logged.seq).await?;
                continue;
            }
            // Logged before the file is touched, so an interruption is recoverable.
            let logged = batch.log_move(self.ids.next(), planned)?;
            self.record(&batch, logged.seq).await?;
            let outcome = self.settlement.move_file(planned).await?;
            leftovers.extend(outcome.folder_to_prune(planned));
            let _settled = batch.settle_move(logged.seq, outcome)?;
            self.record(&batch, logged.seq).await?;
        }

        let pruned = self
            .folders
            .prune(PruneStarts::new(leftovers), &protected)
            .await;
        let (removed, notes) = pruned.into_parts();
        let _finished = batch.finish(removed, notes, self.clock.now())?;
        self.batches.save(&batch).await?;
        Ok(MoveBatchOutput::from(&batch))
    }

    async fn record(&self, batch: &MoveBatch, seq: i32) -> Result<(), ExecuteMovesError> {
        self.batches.record_op(batch.id(), batch.op(seq)?).await?;
        Ok(())
    }
}
