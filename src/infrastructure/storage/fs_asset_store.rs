use std::path::PathBuf;

use async_trait::async_trait;
use tokio::io::AsyncReadExt;

use crate::domain::asset::{Asset, AssetStore, AssetStoreError};

pub(crate) struct FsAssetStore {
    root: PathBuf,
}

impl FsAssetStore {
    pub(crate) fn new(root: PathBuf) -> Self {
        Self { root }
    }
}

fn unavailable(e: std::io::Error) -> AssetStoreError {
    AssetStoreError::Unavailable(Box::new(e))
}

#[async_trait]
impl AssetStore for FsAssetStore {
    async fn store(&self, asset: &Asset, bytes: &[u8]) -> Result<(), AssetStoreError> {
        tokio::fs::write(self.root.join(asset.name()), bytes)
            .await
            .map_err(unavailable)
    }

    async fn read(&self, asset: &Asset) -> Result<Option<Vec<u8>>, AssetStoreError> {
        let Ok(mut file) = tokio::fs::File::open(self.root.join(asset.name())).await else {
            return Ok(None);
        };
        let metadata = file.metadata().await.map_err(unavailable)?;
        let mut buf = Vec::with_capacity(usize::try_from(metadata.len()).unwrap_or(0));
        file.read_to_end(&mut buf).await.map_err(unavailable)?;
        Ok(Some(buf))
    }
}
