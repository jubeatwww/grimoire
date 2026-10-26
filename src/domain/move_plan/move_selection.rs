use serde::Deserialize;

use crate::domain::inventory_item::InventoryItemId;

/// The files the user picked to move: at least one.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(try_from = "Vec<InventoryItemId>")]
pub(crate) struct MoveSelection(Vec<InventoryItemId>);

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("select at least one file")]
pub(crate) struct NothingSelected;

impl MoveSelection {
    pub(crate) fn ids(&self) -> &[InventoryItemId] {
        &self.0
    }
}

impl TryFrom<Vec<InventoryItemId>> for MoveSelection {
    type Error = NothingSelected;

    fn try_from(ids: Vec<InventoryItemId>) -> Result<Self, Self::Error> {
        if ids.is_empty() {
            return Err(NothingSelected);
        }
        Ok(Self(ids))
    }
}
