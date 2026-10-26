//! A file in the library: one zip or rar the scanner found below the library root.

mod events;
mod inventory_item_repository;
mod library_scanner;
mod organization_status;
mod scanned_file;

use crate::domain::game_work::GameWorkId;
use crate::domain::library_path::LibraryPath;
use crate::kernel::Id;

pub(crate) use events::{
    InventoryItemDiscovered, InventoryItemExcluded, InventoryItemLinked, InventoryItemReset,
    InventoryItemSkipped,
};
pub(crate) use inventory_item_repository::{InventoryItemRepository, InventoryItemRepositoryError};
pub(crate) use library_scanner::{LibraryScan, LibraryScanner, LibraryScannerError};
pub(crate) use organization_status::OrganizationStatus;
pub(crate) use scanned_file::ScannedFile;

#[cfg(test)]
pub(crate) mod fakes {
    pub(crate) use super::inventory_item_repository::fakes::FakeInventoryItemRepository;
}

pub(crate) type InventoryItemId = Id<InventoryItem>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct InventoryItem {
    id: InventoryItemId,
    file: ScannedFile,
    work_id: Option<GameWorkId>,
    status: OrganizationStatus,
}

impl InventoryItem {
    /// A file the scanner found that the library did not know yet.
    pub(crate) fn discover(
        id: InventoryItemId,
        file: ScannedFile,
    ) -> (Self, InventoryItemDiscovered) {
        let discovered = InventoryItemDiscovered {
            item_id: id,
            path: file.path().clone(),
        };
        let item = Self {
            id,
            file,
            work_id: None,
            status: OrganizationStatus::Pending,
        };
        (item, discovered)
    }

    /// No source had the work; try again later, for example through VNDB.
    pub(crate) fn skip(&mut self) -> InventoryItemSkipped {
        self.status = OrganizationStatus::NoMatch;
        InventoryItemSkipped { item_id: self.id }
    }

    /// Not a game (a compilation, junk): never process it.
    pub(crate) fn exclude(&mut self) -> InventoryItemExcluded {
        self.status = OrganizationStatus::Ignored;
        InventoryItemExcluded { item_id: self.id }
    }

    /// Back to the queue, unlinked from its work.
    pub(crate) fn reset(&mut self) -> InventoryItemReset {
        self.work_id = None;
        self.status = OrganizationStatus::Pending;
        InventoryItemReset { item_id: self.id }
    }

    pub(crate) fn link_to(&mut self, work_id: GameWorkId) -> InventoryItemLinked {
        self.work_id = Some(work_id);
        self.status = OrganizationStatus::Confirmed;
        InventoryItemLinked {
            item_id: self.id,
            work_id,
        }
    }

    /// Rebuilds a stored item. Repositories only.
    pub(crate) fn from_persisted(
        id: InventoryItemId,
        file: ScannedFile,
        work_id: Option<GameWorkId>,
        status: OrganizationStatus,
    ) -> Self {
        Self {
            id,
            file,
            work_id,
            status,
        }
    }

    pub(crate) fn id(&self) -> InventoryItemId {
        self.id
    }

    pub(crate) fn file(&self) -> &ScannedFile {
        &self.file
    }

    pub(crate) fn path(&self) -> &LibraryPath {
        self.file.path()
    }

    pub(crate) fn work_id(&self) -> Option<GameWorkId> {
        self.work_id
    }

    pub(crate) fn status(&self) -> OrganizationStatus {
        self.status
    }
}
