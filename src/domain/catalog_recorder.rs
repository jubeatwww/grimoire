use async_trait::async_trait;

use crate::domain::game_work::GameWork;
use crate::domain::inventory_item::InventoryItem;
use crate::domain::work_source_data::WorkSourceData;

/// Records one cataloguing step: a work, what a site said about it, and the file now linked to
/// it. Everything given is stored together or not at all, so a failed link never leaves an
/// orphan work behind.
#[async_trait]
pub(crate) trait CatalogRecorder: Send + Sync {
    async fn record(
        &self,
        work: &GameWork,
        source_data: Option<&WorkSourceData>,
        item: Option<&InventoryItem>,
    ) -> Result<(), CatalogRecorderError>;
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum CatalogRecorderError {
    #[error("catalogue storage failed: {0}")]
    Backend(#[source] Box<dyn std::error::Error + Send + Sync>),
}

#[cfg(test)]
pub(crate) mod fakes {
    use std::sync::Arc;

    use async_trait::async_trait;

    use super::{CatalogRecorder, CatalogRecorderError};
    use crate::domain::game_work::fakes::FakeGameWorkRepository;
    use crate::domain::game_work::{GameWork, GameWorkRepository};
    use crate::domain::inventory_item::fakes::FakeInventoryItemRepository;
    use crate::domain::inventory_item::{InventoryItem, InventoryItemRepository};
    use crate::domain::work_source_data::WorkSourceData;

    /// Writes into the fake repositories, so tests read the recorded state back through them.
    pub(crate) struct FakeCatalogRecorder {
        works: Arc<FakeGameWorkRepository>,
        inventory: Arc<FakeInventoryItemRepository>,
    }

    impl FakeCatalogRecorder {
        pub(crate) fn new(
            works: Arc<FakeGameWorkRepository>,
            inventory: Arc<FakeInventoryItemRepository>,
        ) -> Self {
            Self { works, inventory }
        }
    }

    #[async_trait]
    impl CatalogRecorder for FakeCatalogRecorder {
        async fn record(
            &self,
            work: &GameWork,
            _source_data: Option<&WorkSourceData>,
            item: Option<&InventoryItem>,
        ) -> Result<(), CatalogRecorderError> {
            self.works
                .save(work)
                .await
                .map_err(|e| CatalogRecorderError::Backend(Box::new(e)))?;
            if let Some(item) = item {
                self.inventory
                    .save(item)
                    .await
                    .map_err(|e| CatalogRecorderError::Backend(Box::new(e)))?;
            }
            Ok(())
        }
    }
}
