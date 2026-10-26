use async_trait::async_trait;

use super::Asset;

#[async_trait]
pub(crate) trait AssetStore: Send + Sync {
    async fn store(&self, asset: &Asset, bytes: &[u8]) -> Result<(), AssetStoreError>;

    /// `None` when no such image is cached.
    async fn read(&self, asset: &Asset) -> Result<Option<Vec<u8>>, AssetStoreError>;
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum AssetStoreError {
    #[error("the asset cache failed: {0}")]
    Unavailable(#[source] Box<dyn std::error::Error + Send + Sync>),
}
