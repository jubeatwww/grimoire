use async_trait::async_trait;

/// Where genre codes are stored outside the vocabulary, as written there (with prefixes and
/// levels), so a code still in use is not removed.
#[async_trait]
pub(crate) trait GenreCodeUsage: Send + Sync {
    async fn codes_in_works(&self) -> Result<Vec<String>, GenreCodeUsageError>;

    async fn codes_in_rules(&self) -> Result<Vec<String>, GenreCodeUsageError>;
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum GenreCodeUsageError {
    #[error("genre code usage could not be read: {0}")]
    Backend(#[source] Box<dyn std::error::Error + Send + Sync>),
}
