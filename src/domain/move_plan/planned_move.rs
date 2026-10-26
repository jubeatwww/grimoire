use crate::domain::file_placement::FilePlacement;
use crate::domain::game_work::GameWorkId;
use crate::domain::inventory_item::InventoryItemId;
use crate::domain::library_path::LibraryPath;

use super::MoveConflict;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PlannedMove {
    item_id: InventoryItemId,
    work_id: GameWorkId,
    title: String,
    category_id: String,
    from: LibraryPath,
    to: LibraryPath,
    conflict: Option<MoveConflict>,
}

impl PlannedMove {
    pub(crate) fn new(placement: &FilePlacement, to: LibraryPath) -> Self {
        Self {
            item_id: placement.item_id(),
            work_id: placement.work().work_id(),
            title: placement.work().title().to_owned(),
            category_id: placement.work().category_id().to_owned(),
            from: placement.path().clone(),
            to,
            conflict: None,
        }
    }

    pub(super) fn block(&mut self, conflict: Option<MoveConflict>) {
        self.conflict = conflict;
    }

    pub(crate) fn item_id(&self) -> InventoryItemId {
        self.item_id
    }

    pub(crate) fn work_id(&self) -> GameWorkId {
        self.work_id
    }

    pub(crate) fn title(&self) -> &str {
        &self.title
    }

    pub(crate) fn category_id(&self) -> &str {
        &self.category_id
    }

    pub(crate) fn from(&self) -> &LibraryPath {
        &self.from
    }

    pub(crate) fn to(&self) -> &LibraryPath {
        &self.to
    }

    pub(crate) fn conflict(&self) -> Option<MoveConflict> {
        self.conflict
    }
}
