use async_trait::async_trait;

use super::GameWork;

/// Records a batch classification: every work is stored or none is.
#[async_trait]
pub(crate) trait ClassificationRecorder: Send + Sync {
    async fn record(&self, works: &[GameWork]) -> Result<(), ClassificationRecorderError>;
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum ClassificationRecorderError {
    #[error("classification storage failed: {0}")]
    Backend(#[source] Box<dyn std::error::Error + Send + Sync>),
}
