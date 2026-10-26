use std::sync::Arc;

use serde::Serialize;

use crate::application::organization::{MoveReconciliationAppService, MoveReconciliationError};
use crate::domain::library_lock_registry::LibraryLockRegistry;
use crate::kernel::{ErrorCode, ErrorKind};

#[derive(Debug, thiserror::Error)]
pub(crate) enum ReconcileMoveBatchesError {
    #[error(transparent)]
    Reconciliation(#[from] MoveReconciliationError),
}

impl ErrorCode for ReconcileMoveBatchesError {
    fn code(&self) -> &'static str {
        "grimoire.unavailable"
    }

    fn kind(&self) -> ErrorKind {
        ErrorKind::Unavailable
    }
}

/// How many interrupted batches were settled.
#[derive(Debug, Serialize)]
pub(crate) struct ReconcileMoveBatchesOutput {
    pub(crate) reconciled: usize,
}

/// Run once at startup: settles move batches a crash or restart left half-done.
pub(crate) struct ReconcileMoveBatchesUseCase {
    lock: Arc<dyn LibraryLockRegistry>,
    reconciliation: Arc<MoveReconciliationAppService>,
}

impl ReconcileMoveBatchesUseCase {
    pub(crate) fn new(
        lock: Arc<dyn LibraryLockRegistry>,
        reconciliation: Arc<MoveReconciliationAppService>,
    ) -> Self {
        Self {
            lock,
            reconciliation,
        }
    }

    #[tracing::instrument(name = "use_case.reconcile_move_batches", skip(self))]
    pub(crate) async fn execute(
        &self,
    ) -> Result<ReconcileMoveBatchesOutput, ReconcileMoveBatchesError> {
        let _lease = self.lock.acquire().await;
        Ok(ReconcileMoveBatchesOutput {
            reconciled: self.reconciliation.reconcile().await?,
        })
    }
}
