use std::sync::Arc;

use async_trait::async_trait;

use super::{DownloadInventoryItemOutput, InventoryItemListItemOutput, ListInventoryItemsOutput};
use crate::domain::inventory_item::{
    InventoryItemId, InventoryItemRepository, InventoryItemRepositoryError,
};
use crate::domain::library_files::{LibraryFiles, LibraryFilesError};
use crate::kernel::{ErrorCode, ErrorKind};

/// The whole library is listed at once; this only guards against a runaway table.
const LIBRARY_LIST_LIMIT: i64 = 100_000;

/// Read model of the library list: files joined with their works and every source's tags.
#[async_trait]
pub(crate) trait LibraryClientQueryRepository: Send + Sync {
    /// By file name.
    async fn list_items(
        &self,
        limit: i64,
    ) -> Result<Vec<InventoryItemListItemOutput>, LibraryClientQueryRepositoryError>;
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum LibraryClientQueryRepositoryError {
    #[error("the library list could not be read: {0}")]
    Backend(#[source] Box<dyn std::error::Error + Send + Sync>),
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum LibraryClientQueryError {
    #[error("inventory item {0} not found")]
    NotFound(InventoryItemId),
    #[error("the file of inventory item {0} is not on disk")]
    FileMissing(InventoryItemId),
    #[error(transparent)]
    List(#[from] LibraryClientQueryRepositoryError),
    #[error(transparent)]
    Inventory(#[from] InventoryItemRepositoryError),
    #[error(transparent)]
    Files(#[from] LibraryFilesError),
}

impl ErrorCode for LibraryClientQueryError {
    fn code(&self) -> &'static str {
        match self {
            Self::NotFound(_) => "inventory_item.not_found",
            Self::FileMissing(_) => "inventory_item.file_missing",
            Self::List(_) | Self::Inventory(_) | Self::Files(_) => "grimoire.unavailable",
        }
    }

    fn kind(&self) -> ErrorKind {
        match self {
            Self::NotFound(_) | Self::FileMissing(_) => ErrorKind::NotFound,
            Self::List(_) | Self::Inventory(_) | Self::Files(_) => ErrorKind::Unavailable,
        }
    }
}

pub(crate) struct LibraryClientQueryUseCase {
    items: Arc<dyn LibraryClientQueryRepository>,
    inventory: Arc<dyn InventoryItemRepository>,
    files: Arc<dyn LibraryFiles>,
}

impl LibraryClientQueryUseCase {
    pub(crate) fn new(
        items: Arc<dyn LibraryClientQueryRepository>,
        inventory: Arc<dyn InventoryItemRepository>,
        files: Arc<dyn LibraryFiles>,
    ) -> Self {
        Self {
            items,
            inventory,
            files,
        }
    }

    #[tracing::instrument(name = "use_case.list_library", skip(self))]
    pub(crate) async fn list_items(
        &self,
    ) -> Result<ListInventoryItemsOutput, LibraryClientQueryError> {
        Ok(ListInventoryItemsOutput {
            items: self.items.list_items(LIBRARY_LIST_LIMIT).await?,
        })
    }

    /// Where an item's file is, to stream it.
    #[tracing::instrument(name = "use_case.find_download", skip(self))]
    pub(crate) async fn find_download(
        &self,
        id: InventoryItemId,
    ) -> Result<DownloadInventoryItemOutput, LibraryClientQueryError> {
        let item = self
            .inventory
            .find_by_id(id)
            .await?
            .ok_or(LibraryClientQueryError::NotFound(id))?;
        let on_disk = self.files.exist(std::slice::from_ref(item.path())).await?;
        if !on_disk.first().copied().unwrap_or(false) {
            return Err(LibraryClientQueryError::FileMissing(id));
        }
        Ok(DownloadInventoryItemOutput {
            path: self.files.locate(item.path()),
            file_name: item.file().file_name().to_owned(),
        })
    }
}
