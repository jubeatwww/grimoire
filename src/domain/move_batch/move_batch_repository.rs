use async_trait::async_trait;

use super::{MoveBatch, MoveBatchId, MoveOp};

#[async_trait]
pub(crate) trait MoveBatchRepository: Send + Sync {
    /// With every op.
    async fn find_by_id(
        &self,
        id: MoveBatchId,
    ) -> Result<Option<MoveBatch>, MoveBatchRepositoryError>;

    /// Newest first, with every op.
    async fn list_recent(&self, limit: i64) -> Result<Vec<MoveBatch>, MoveBatchRepositoryError>;

    /// Batches a crash, restart or lost database connection left running or undoing.
    async fn list_interrupted(
        &self,
        limit: i64,
    ) -> Result<Vec<MoveBatch>, MoveBatchRepositoryError>;

    /// Upsert of the batch and all its ops.
    async fn save(&self, batch: &MoveBatch) -> Result<(), MoveBatchRepositoryError>;

    /// Writes one op's current state: the log entry a move or undo leaves before and after its
    /// file is touched. Cheaper than saving the whole batch after every file.
    async fn record_op(
        &self,
        batch: MoveBatchId,
        op: &MoveOp,
    ) -> Result<(), MoveBatchRepositoryError>;
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum MoveBatchRepositoryError {
    #[error("move batch storage failed: {0}")]
    Backend(#[source] Box<dyn std::error::Error + Send + Sync>),
}
