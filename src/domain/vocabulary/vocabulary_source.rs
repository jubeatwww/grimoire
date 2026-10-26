use async_trait::async_trait;

use super::Vocabulary;

#[async_trait]
pub(crate) trait VocabularySource: Send + Sync {
    /// The categories and codes as stored right now.
    async fn current(&self) -> Result<Vocabulary, VocabularySourceError>;
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum VocabularySourceError {
    #[error("vocabulary storage failed: {0}")]
    Backend(#[source] Box<dyn std::error::Error + Send + Sync>),
}
