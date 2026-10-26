use serde::Serialize;

use super::InventoryItemId;
use crate::domain::game_work::GameWorkId;
use crate::domain::library_path::LibraryPath;

#[must_use = "publish the event or discard it explicitly"]
#[derive(Debug, Clone, Serialize)]
pub(crate) struct InventoryItemDiscovered {
    pub(crate) item_id: InventoryItemId,
    pub(crate) path: LibraryPath,
}

#[must_use = "publish the event or discard it explicitly"]
#[derive(Debug, Clone, Serialize)]
pub(crate) struct InventoryItemSkipped {
    pub(crate) item_id: InventoryItemId,
}

#[must_use = "publish the event or discard it explicitly"]
#[derive(Debug, Clone, Serialize)]
pub(crate) struct InventoryItemExcluded {
    pub(crate) item_id: InventoryItemId,
}

#[must_use = "publish the event or discard it explicitly"]
#[derive(Debug, Clone, Serialize)]
pub(crate) struct InventoryItemReset {
    pub(crate) item_id: InventoryItemId,
}

#[must_use = "publish the event or discard it explicitly"]
#[derive(Debug, Clone, Serialize)]
pub(crate) struct InventoryItemLinked {
    pub(crate) item_id: InventoryItemId,
    pub(crate) work_id: GameWorkId,
}
