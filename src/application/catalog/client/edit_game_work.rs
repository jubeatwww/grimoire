use std::sync::Arc;

use serde::Deserialize;

use crate::domain::game_work::{GameWorkRepository, GameWorkRepositoryError, WorkEdit};
use crate::domain::inventory_item::{
    InventoryItemId, InventoryItemRepository, InventoryItemRepositoryError,
};
use crate::kernel::{ErrorCode, ErrorKind};

/// See `WorkEdit` for what a missing field and an empty string mean.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct EditGameWorkInput {
    pub(crate) inventory_item_id: InventoryItemId,
    pub(crate) display_title: Option<String>,
    pub(crate) work_type: Option<String>,
    pub(crate) work_type_label: Option<String>,
    pub(crate) cover_image_url: Option<String>,
    pub(crate) preview_image_urls: Option<Vec<String>>,
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum EditGameWorkError {
    #[error("inventory item {0} not found")]
    ItemNotFound(InventoryItemId),
    #[error("inventory item {0} has no work yet")]
    NotLinked(InventoryItemId),
    #[error(transparent)]
    Inventory(#[from] InventoryItemRepositoryError),
    #[error(transparent)]
    Works(#[from] GameWorkRepositoryError),
}

impl ErrorCode for EditGameWorkError {
    fn code(&self) -> &'static str {
        match self {
            Self::ItemNotFound(_) => "inventory_item.not_found",
            Self::NotLinked(_) => "inventory_item.not_linked",
            Self::Inventory(_) | Self::Works(_) => "grimoire.unavailable",
        }
    }

    fn kind(&self) -> ErrorKind {
        match self {
            Self::ItemNotFound(_) => ErrorKind::NotFound,
            Self::NotLinked(_) => ErrorKind::Invalid,
            Self::Inventory(_) | Self::Works(_) => ErrorKind::Unavailable,
        }
    }
}

/// A manual edit of a file's work from the item panel.
pub(crate) struct EditGameWorkUseCase {
    inventory: Arc<dyn InventoryItemRepository>,
    works: Arc<dyn GameWorkRepository>,
}

impl EditGameWorkUseCase {
    pub(crate) fn new(
        inventory: Arc<dyn InventoryItemRepository>,
        works: Arc<dyn GameWorkRepository>,
    ) -> Self {
        Self { inventory, works }
    }

    #[tracing::instrument(name = "use_case.edit_game_work", skip(self))]
    pub(crate) async fn execute(&self, input: EditGameWorkInput) -> Result<(), EditGameWorkError> {
        let item_id = input.inventory_item_id;
        let item = self
            .inventory
            .find_by_id(item_id)
            .await?
            .ok_or(EditGameWorkError::ItemNotFound(item_id))?;
        let work_id = item
            .work_id()
            .ok_or(EditGameWorkError::NotLinked(item_id))?;
        let mut work = self
            .works
            .find_by_id(work_id)
            .await?
            .ok_or(EditGameWorkError::NotLinked(item_id))?;
        let _edited = work.edit(WorkEdit::new(
            input.display_title,
            input.work_type,
            input.work_type_label,
            input.cover_image_url,
            input.preview_image_urls,
        ));
        self.works.save(&work).await?;
        Ok(())
    }
}
