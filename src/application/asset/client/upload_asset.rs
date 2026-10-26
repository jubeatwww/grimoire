use std::sync::Arc;

use super::UploadAssetOutput;
use crate::domain::asset::{Asset, AssetError, AssetStore, AssetStoreError};
use crate::kernel::{ErrorCode, ErrorKind, IdGenerator};

/// An uploaded image and the name it had on the user's disk.
#[derive(Debug)]
pub(crate) struct UploadAssetInput {
    pub(crate) original_name: String,
    pub(crate) bytes: Vec<u8>,
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum UploadAssetError {
    #[error(transparent)]
    Asset(#[from] AssetError),
    #[error(transparent)]
    Store(#[from] AssetStoreError),
}

impl ErrorCode for UploadAssetError {
    fn code(&self) -> &'static str {
        match self {
            Self::Asset(e) => e.code(),
            Self::Store(_) => "grimoire.unavailable",
        }
    }

    fn kind(&self) -> ErrorKind {
        match self {
            Self::Asset(e) => e.kind(),
            Self::Store(_) => ErrorKind::Unavailable,
        }
    }
}

pub(crate) struct UploadAssetUseCase {
    store: Arc<dyn AssetStore>,
    ids: Arc<dyn IdGenerator>,
}

impl UploadAssetUseCase {
    pub(crate) fn new(store: Arc<dyn AssetStore>, ids: Arc<dyn IdGenerator>) -> Self {
        Self { store, ids }
    }

    #[tracing::instrument(name = "use_case.upload_asset", skip(self, input))]
    pub(crate) async fn execute(
        &self,
        input: UploadAssetInput,
    ) -> Result<UploadAssetOutput, UploadAssetError> {
        let asset = Asset::accept_upload(self.ids.next(), &input.original_name, input.bytes.len())?;
        self.store.store(&asset, &input.bytes).await?;
        Ok(UploadAssetOutput {
            name: asset.name().to_owned(),
        })
    }
}
