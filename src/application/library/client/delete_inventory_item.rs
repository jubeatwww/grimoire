use std::sync::Arc;

use serde::Deserialize;

use crate::domain::inventory_item::{
    InventoryItemId, InventoryItemRepository, InventoryItemRepositoryError,
};
use crate::kernel::{ErrorCode, ErrorKind};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DeleteInventoryItemInput {
    pub(crate) inventory_item_id: InventoryItemId,
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum DeleteInventoryItemError {
    #[error("inventory item {0} not found")]
    NotFound(InventoryItemId),
    #[error(transparent)]
    Inventory(#[from] InventoryItemRepositoryError),
}

impl ErrorCode for DeleteInventoryItemError {
    fn code(&self) -> &'static str {
        match self {
            Self::NotFound(_) => "inventory_item.not_found",
            Self::Inventory(_) => "grimoire.unavailable",
        }
    }

    fn kind(&self) -> ErrorKind {
        match self {
            Self::NotFound(_) => ErrorKind::NotFound,
            Self::Inventory(_) => ErrorKind::Unavailable,
        }
    }
}

/// Drops an orphan record once its file is gone for good (for example after the Synology
/// recycle bin was emptied). The linked work stays, in case other files reference it.
pub(crate) struct DeleteInventoryItemUseCase {
    inventory: Arc<dyn InventoryItemRepository>,
}

impl DeleteInventoryItemUseCase {
    pub(crate) fn new(inventory: Arc<dyn InventoryItemRepository>) -> Self {
        Self { inventory }
    }

    #[tracing::instrument(name = "use_case.delete_inventory_item", skip(self))]
    pub(crate) async fn execute(
        &self,
        input: DeleteInventoryItemInput,
    ) -> Result<(), DeleteInventoryItemError> {
        let id = input.inventory_item_id;
        self.inventory
            .find_by_id(id)
            .await?
            .ok_or(DeleteInventoryItemError::NotFound(id))?;
        self.inventory.delete(id).await?;
        Ok(())
    }
}
