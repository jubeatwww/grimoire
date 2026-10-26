use std::sync::Arc;

use serde::Deserialize;

use super::DeleteInventoryItemFileOutput;
use crate::domain::inventory_item::{
    InventoryItemId, InventoryItemRepository, InventoryItemRepositoryError,
};
use crate::domain::library_files::{FileRemoval, LibraryFiles, LibraryFilesError};
use crate::domain::library_lock_registry::LibraryLockRegistry;
use crate::kernel::{ErrorCode, ErrorKind};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DeleteInventoryItemFileInput {
    pub(crate) inventory_item_id: InventoryItemId,
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum DeleteInventoryItemFileError {
    #[error("inventory item {0} not found")]
    NotFound(InventoryItemId),
    #[error(transparent)]
    Inventory(#[from] InventoryItemRepositoryError),
    #[error(transparent)]
    Files(#[from] LibraryFilesError),
}

impl ErrorCode for DeleteInventoryItemFileError {
    fn code(&self) -> &'static str {
        match self {
            Self::NotFound(_) => "inventory_item.not_found",
            Self::Inventory(_) | Self::Files(_) => "grimoire.unavailable",
        }
    }

    fn kind(&self) -> ErrorKind {
        match self {
            Self::NotFound(_) => ErrorKind::NotFound,
            Self::Inventory(_) | Self::Files(_) => ErrorKind::Unavailable,
        }
    }
}

/// Deletes a file on the NAS and its record, for cleaning up duplicates. The linked work stays.
pub(crate) struct DeleteInventoryItemFileUseCase {
    inventory: Arc<dyn InventoryItemRepository>,
    files: Arc<dyn LibraryFiles>,
    lock: Arc<dyn LibraryLockRegistry>,
}

impl DeleteInventoryItemFileUseCase {
    pub(crate) fn new(
        inventory: Arc<dyn InventoryItemRepository>,
        files: Arc<dyn LibraryFiles>,
        lock: Arc<dyn LibraryLockRegistry>,
    ) -> Self {
        Self {
            inventory,
            files,
            lock,
        }
    }

    #[tracing::instrument(name = "use_case.delete_inventory_item_file", skip(self))]
    pub(crate) async fn execute(
        &self,
        input: DeleteInventoryItemFileInput,
    ) -> Result<DeleteInventoryItemFileOutput, DeleteInventoryItemFileError> {
        // Never delete a file while files are being moved or scanned.
        let _lease = self.lock.acquire().await;
        let id = input.inventory_item_id;
        let item = self
            .inventory
            .find_by_id(id)
            .await?
            .ok_or(DeleteInventoryItemFileError::NotFound(id))?;
        let removal = self.files.remove_file(item.path()).await?;
        self.inventory.delete(id).await?;
        Ok(DeleteInventoryItemFileOutput {
            file_deleted: removal == FileRemoval::Removed,
            file_missing: removal == FileRemoval::AlreadyGone,
        })
    }
}
