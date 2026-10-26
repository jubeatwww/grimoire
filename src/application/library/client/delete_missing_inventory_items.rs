use std::sync::Arc;

use super::DeleteMissingInventoryItemsOutput;
use crate::domain::inventory_item::{InventoryItemRepository, InventoryItemRepositoryError};
use crate::kernel::{ErrorCode, ErrorKind};

#[derive(Debug, thiserror::Error)]
pub(crate) enum DeleteMissingInventoryItemsError {
    #[error(transparent)]
    Inventory(#[from] InventoryItemRepositoryError),
}

impl ErrorCode for DeleteMissingInventoryItemsError {
    fn code(&self) -> &'static str {
        "grimoire.unavailable"
    }

    fn kind(&self) -> ErrorKind {
        ErrorKind::Unavailable
    }
}

/// One-shot cleanup of every record flagged missing.
pub(crate) struct DeleteMissingInventoryItemsUseCase {
    inventory: Arc<dyn InventoryItemRepository>,
}

impl DeleteMissingInventoryItemsUseCase {
    pub(crate) fn new(inventory: Arc<dyn InventoryItemRepository>) -> Self {
        Self { inventory }
    }

    #[tracing::instrument(name = "use_case.delete_missing_inventory_items", skip(self))]
    pub(crate) async fn execute(
        &self,
    ) -> Result<DeleteMissingInventoryItemsOutput, DeleteMissingInventoryItemsError> {
        Ok(DeleteMissingInventoryItemsOutput {
            deleted: self.inventory.delete_missing().await?,
        })
    }
}
