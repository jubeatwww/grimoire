use std::sync::Arc;

use serde::Deserialize;

use crate::domain::inventory_item::{
    InventoryItem, InventoryItemId, InventoryItemRepository, InventoryItemRepositoryError,
};
use crate::kernel::{ErrorCode, ErrorKind};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TriageInventoryItemInput {
    pub(crate) inventory_item_id: InventoryItemId,
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum InventoryItemTriageError {
    #[error("inventory item {0} not found")]
    NotFound(InventoryItemId),
    #[error(transparent)]
    Inventory(#[from] InventoryItemRepositoryError),
}

impl ErrorCode for InventoryItemTriageError {
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

/// The matching queue's quick actions on one file: each loads the file, changes where it
/// stands in the queue and stores it.
pub(crate) struct InventoryItemTriageUseCase {
    inventory: Arc<dyn InventoryItemRepository>,
}

impl InventoryItemTriageUseCase {
    pub(crate) fn new(inventory: Arc<dyn InventoryItemRepository>) -> Self {
        Self { inventory }
    }

    async fn change(
        &self,
        id: InventoryItemId,
        action: impl FnOnce(&mut InventoryItem),
    ) -> Result<(), InventoryItemTriageError> {
        let mut item = self
            .inventory
            .find_by_id(id)
            .await?
            .ok_or(InventoryItemTriageError::NotFound(id))?;
        action(&mut item);
        self.inventory.save(&item).await?;
        Ok(())
    }

    /// No site had the work; try again later, for example through VNDB.
    #[tracing::instrument(name = "use_case.skip_inventory_item", skip(self))]
    pub(crate) async fn skip(
        &self,
        input: TriageInventoryItemInput,
    ) -> Result<(), InventoryItemTriageError> {
        self.change(input.inventory_item_id, |item| {
            let _skipped = item.skip();
        })
        .await
    }

    /// Not a game: never process it.
    #[tracing::instrument(name = "use_case.exclude_inventory_item", skip(self))]
    pub(crate) async fn exclude(
        &self,
        input: TriageInventoryItemInput,
    ) -> Result<(), InventoryItemTriageError> {
        self.change(input.inventory_item_id, |item| {
            let _excluded = item.exclude();
        })
        .await
    }

    /// Back to the queue, unlinked from its work.
    #[tracing::instrument(name = "use_case.reset_inventory_item", skip(self))]
    pub(crate) async fn reset(
        &self,
        input: TriageInventoryItemInput,
    ) -> Result<(), InventoryItemTriageError> {
        self.change(input.inventory_item_id, |item| {
            let _reset = item.reset();
        })
        .await
    }
}
