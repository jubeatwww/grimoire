use async_trait::async_trait;

use super::GenreCode;

#[async_trait]
pub(crate) trait GenreCodeRepository: Send + Sync {
    async fn find_by_code(&self, code: &str)
    -> Result<Option<GenreCode>, GenreCodeRepositoryError>;

    /// Upsert. An existing code keeps its place in the list.
    async fn save(&self, code: &GenreCode) -> Result<(), GenreCodeRepositoryError>;

    /// Idempotent.
    async fn delete(&self, code: &str) -> Result<(), GenreCodeRepositoryError>;
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum GenreCodeRepositoryError {
    #[error("genre code storage failed: {0}")]
    Backend(#[source] Box<dyn std::error::Error + Send + Sync>),
}
