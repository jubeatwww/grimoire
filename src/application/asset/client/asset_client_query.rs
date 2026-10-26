use std::sync::Arc;

use super::AssetOutput;
use crate::domain::asset::{Asset, AssetStore, AssetStoreError};
use crate::kernel::{ErrorCode, ErrorKind};

#[derive(Debug, thiserror::Error)]
pub(crate) enum AssetClientQueryError {
    #[error("no such asset: {0}")]
    NotFound(String),
    #[error(transparent)]
    Store(#[from] AssetStoreError),
}

impl ErrorCode for AssetClientQueryError {
    fn code(&self) -> &'static str {
        match self {
            Self::NotFound(_) => "asset.not_found",
            Self::Store(_) => "grimoire.unavailable",
        }
    }

    fn kind(&self) -> ErrorKind {
        match self {
            Self::NotFound(_) => ErrorKind::NotFound,
            Self::Store(_) => ErrorKind::Unavailable,
        }
    }
}

pub(crate) struct AssetClientQueryUseCase {
    store: Arc<dyn AssetStore>,
}

impl AssetClientQueryUseCase {
    pub(crate) fn new(store: Arc<dyn AssetStore>) -> Self {
        Self { store }
    }

    #[tracing::instrument(name = "use_case.find_asset", skip(self))]
    pub(crate) async fn find_asset(
        &self,
        asset: Asset,
    ) -> Result<AssetOutput, AssetClientQueryError> {
        let bytes = self
            .store
            .read(&asset)
            .await?
            .ok_or_else(|| AssetClientQueryError::NotFound(asset.name().to_owned()))?;
        Ok(AssetOutput {
            bytes,
            media_type: asset.media_type(),
        })
    }
}
