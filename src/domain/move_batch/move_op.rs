use super::MoveOpStatus;
use crate::domain::inventory_item::InventoryItemId;
use crate::domain::library_path::LibraryPath;
use crate::kernel::Id;

pub(crate) type MoveOpId = Id<MoveOp>;

/// One file of a batch: where it came from, where it went, and how that ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MoveOp {
    id: MoveOpId,
    seq: i32,
    item_id: Option<InventoryItemId>,
    from: LibraryPath,
    to: LibraryPath,
    status: MoveOpStatus,
    message: Option<String>,
}

impl MoveOp {
    /// Rebuilds a stored op. Repositories and the batch only. `item_id` is gone once the
    /// inventory record was deleted.
    pub(crate) fn from_persisted(
        id: MoveOpId,
        seq: i32,
        item_id: Option<InventoryItemId>,
        from: LibraryPath,
        to: LibraryPath,
        status: MoveOpStatus,
        message: Option<String>,
    ) -> Self {
        Self {
            id,
            seq,
            item_id,
            from,
            to,
            status,
            message,
        }
    }

    pub(super) fn change(&mut self, status: MoveOpStatus, message: Option<String>) {
        self.status = status;
        self.message = message;
    }

    pub(crate) fn id(&self) -> MoveOpId {
        self.id
    }

    pub(crate) fn seq(&self) -> i32 {
        self.seq
    }

    pub(crate) fn item_id(&self) -> Option<InventoryItemId> {
        self.item_id
    }

    pub(crate) fn from(&self) -> &LibraryPath {
        &self.from
    }

    pub(crate) fn to(&self) -> &LibraryPath {
        &self.to
    }

    pub(crate) fn status(&self) -> MoveOpStatus {
        self.status
    }

    pub(crate) fn message(&self) -> Option<&str> {
        self.message.as_deref()
    }
}
