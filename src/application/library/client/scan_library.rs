use std::sync::Arc;

use super::ScanLibraryOutput;
use crate::domain::inventory_item::{
    InventoryItem, InventoryItemRepository, InventoryItemRepositoryError, LibraryScanner,
    LibraryScannerError,
};
use crate::domain::library_lock_registry::LibraryLockRegistry;
use crate::kernel::{ErrorCode, ErrorKind, IdGenerator};

#[derive(Debug, thiserror::Error)]
pub(crate) enum ScanLibraryError {
    #[error(transparent)]
    Scanner(#[from] LibraryScannerError),
    #[error(transparent)]
    Inventory(#[from] InventoryItemRepositoryError),
}

impl ErrorCode for ScanLibraryError {
    fn code(&self) -> &'static str {
        "grimoire.unavailable"
    }

    fn kind(&self) -> ErrorKind {
        ErrorKind::Unavailable
    }
}

/// Walks the NAS and records what is there: new files are added, known ones refreshed, and
/// files that went away marked missing.
pub(crate) struct ScanLibraryUseCase {
    scanner: Arc<dyn LibraryScanner>,
    inventory: Arc<dyn InventoryItemRepository>,
    lock: Arc<dyn LibraryLockRegistry>,
    ids: Arc<dyn IdGenerator>,
}

impl ScanLibraryUseCase {
    pub(crate) fn new(
        scanner: Arc<dyn LibraryScanner>,
        inventory: Arc<dyn InventoryItemRepository>,
        lock: Arc<dyn LibraryLockRegistry>,
        ids: Arc<dyn IdGenerator>,
    ) -> Self {
        Self {
            scanner,
            inventory,
            lock,
            ids,
        }
    }

    #[tracing::instrument(name = "use_case.scan_library", skip(self))]
    pub(crate) async fn execute(&self) -> Result<ScanLibraryOutput, ScanLibraryError> {
        // Never scan while files are being moved.
        let _lease = self.lock.acquire().await;
        let (files, warnings) = self.scanner.scan().await?.into_parts();
        // A known path keeps its existing record; the new id is only used for new files.
        let found: Vec<InventoryItem> = files
            .into_iter()
            .map(|file| InventoryItem::discover(self.ids.next(), file).0)
            .collect();
        let marked_missing = self.inventory.record_scan(&found).await?;
        Ok(ScanLibraryOutput {
            scanned: found.len(),
            marked_missing,
            warnings,
        })
    }
}
