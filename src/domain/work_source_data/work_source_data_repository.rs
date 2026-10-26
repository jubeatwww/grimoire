use async_trait::async_trait;

use super::WorkSourceData;

#[async_trait]
pub(crate) trait WorkSourceDataRepository: Send + Sync {
    /// Upsert by work and source.
    async fn save(&self, data: &WorkSourceData) -> Result<(), WorkSourceDataRepositoryError>;
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum WorkSourceDataRepositoryError {
    #[error("source data storage failed: {0}")]
    Backend(#[source] Box<dyn std::error::Error + Send + Sync>),
}
