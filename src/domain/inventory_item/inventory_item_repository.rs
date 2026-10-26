use async_trait::async_trait;

use super::{InventoryItem, InventoryItemId};
use crate::domain::library_path::LibraryPath;

#[async_trait]
pub(crate) trait InventoryItemRepository: Send + Sync {
    async fn find_by_id(
        &self,
        id: InventoryItemId,
    ) -> Result<Option<InventoryItem>, InventoryItemRepositoryError>;

    /// Upsert. An existing item keeps its file facts; only its work link and status change.
    async fn save(&self, item: &InventoryItem) -> Result<(), InventoryItemRepositoryError>;

    /// Idempotent.
    async fn delete(&self, id: InventoryItemId) -> Result<(), InventoryItemRepositoryError>;

    /// Records a finished scan of the local source: new files are added, known ones get their
    /// file facts refreshed and are no longer missing, and every other local file is marked
    /// missing. Returns how many were newly marked missing.
    async fn record_scan(
        &self,
        found: &[InventoryItem],
    ) -> Result<u64, InventoryItemRepositoryError>;

    /// Deletes every item marked missing. Returns how many went.
    async fn delete_missing(&self) -> Result<u64, InventoryItemRepositoryError>;

    /// Points the item at `to`, only while it is still at `from`. False when it was not there.
    async fn relocate(
        &self,
        id: InventoryItemId,
        from: &LibraryPath,
        to: &LibraryPath,
    ) -> Result<bool, InventoryItemRepositoryError>;
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum InventoryItemRepositoryError {
    #[error("inventory storage failed: {0}")]
    Backend(#[source] Box<dyn std::error::Error + Send + Sync>),
}

#[cfg(test)]
pub(crate) mod fakes {
    use std::collections::HashMap;
    use std::sync::Mutex;

    use async_trait::async_trait;

    use super::{InventoryItemRepository, InventoryItemRepositoryError};
    use crate::domain::inventory_item::{InventoryItem, InventoryItemId, ScannedFile};
    use crate::domain::library_path::LibraryPath;

    #[derive(Default)]
    pub(crate) struct FakeInventoryItemRepository(Mutex<HashMap<InventoryItemId, InventoryItem>>);

    #[async_trait]
    impl InventoryItemRepository for FakeInventoryItemRepository {
        async fn find_by_id(
            &self,
            id: InventoryItemId,
        ) -> Result<Option<InventoryItem>, InventoryItemRepositoryError> {
            Ok(self.0.lock().unwrap().get(&id).cloned())
        }

        async fn save(&self, item: &InventoryItem) -> Result<(), InventoryItemRepositoryError> {
            self.0.lock().unwrap().insert(item.id(), item.clone());
            Ok(())
        }

        async fn delete(&self, id: InventoryItemId) -> Result<(), InventoryItemRepositoryError> {
            self.0.lock().unwrap().remove(&id);
            Ok(())
        }

        async fn record_scan(
            &self,
            found: &[InventoryItem],
        ) -> Result<u64, InventoryItemRepositoryError> {
            let mut items = self.0.lock().unwrap();
            for item in found {
                items.entry(item.id()).or_insert_with(|| item.clone());
            }
            Ok(0)
        }

        async fn delete_missing(&self) -> Result<u64, InventoryItemRepositoryError> {
            Ok(0)
        }

        async fn relocate(
            &self,
            id: InventoryItemId,
            from: &LibraryPath,
            to: &LibraryPath,
        ) -> Result<bool, InventoryItemRepositoryError> {
            let mut items = self.0.lock().unwrap();
            let Some(item) = items.get_mut(&id).filter(|i| i.path() == from) else {
                return Ok(false);
            };
            let f = item.file().clone();
            let moved = ScannedFile::new(
                to.clone(),
                f.file_name().to_owned(),
                f.extension().map(str::to_owned),
                f.file_size(),
                f.modified_at(),
                f.legacy_location().map(str::to_owned),
            );
            *item = InventoryItem::from_persisted(item.id(), moved, item.work_id(), item.status());
            Ok(true)
        }
    }
}
