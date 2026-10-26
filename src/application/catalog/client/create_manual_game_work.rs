use std::sync::Arc;

use serde::Deserialize;

use super::LinkGameWorkOutput;
use crate::domain::catalog_recorder::{CatalogRecorder, CatalogRecorderError};
use crate::domain::game_work::GameWork;
use crate::domain::inventory_item::{
    InventoryItemId, InventoryItemRepository, InventoryItemRepositoryError,
};
use crate::kernel::{Clock, ErrorCode, ErrorKind, IdGenerator};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CreateManualGameWorkInput {
    pub(crate) inventory_item_id: InventoryItemId,
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum CreateManualGameWorkError {
    #[error("inventory item {0} not found")]
    ItemNotFound(InventoryItemId),
    #[error(transparent)]
    Inventory(#[from] InventoryItemRepositoryError),
    #[error(transparent)]
    Recorder(#[from] CatalogRecorderError),
}

impl ErrorCode for CreateManualGameWorkError {
    fn code(&self) -> &'static str {
        match self {
            Self::ItemNotFound(_) => "inventory_item.not_found",
            Self::Inventory(_) | Self::Recorder(_) => "grimoire.unavailable",
        }
    }

    fn kind(&self) -> ErrorKind {
        match self {
            Self::ItemNotFound(_) => ErrorKind::NotFound,
            Self::Inventory(_) | Self::Recorder(_) => ErrorKind::Unavailable,
        }
    }
}

/// An empty work for a file no site knows, titled after the file, so the user can fill it in.
/// A file that already has a work keeps it: no orphan work is created.
pub(crate) struct CreateManualGameWorkUseCase {
    inventory: Arc<dyn InventoryItemRepository>,
    recorder: Arc<dyn CatalogRecorder>,
    ids: Arc<dyn IdGenerator>,
    clock: Arc<dyn Clock>,
}

impl CreateManualGameWorkUseCase {
    pub(crate) fn new(
        inventory: Arc<dyn InventoryItemRepository>,
        recorder: Arc<dyn CatalogRecorder>,
        ids: Arc<dyn IdGenerator>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            inventory,
            recorder,
            ids,
            clock,
        }
    }

    #[tracing::instrument(name = "use_case.create_manual_game_work", skip(self))]
    pub(crate) async fn execute(
        &self,
        input: CreateManualGameWorkInput,
    ) -> Result<LinkGameWorkOutput, CreateManualGameWorkError> {
        let mut item = self
            .inventory
            .find_by_id(input.inventory_item_id)
            .await?
            .ok_or(CreateManualGameWorkError::ItemNotFound(
                input.inventory_item_id,
            ))?;
        if let Some(existing) = item.work_id() {
            return Ok(LinkGameWorkOutput {
                game_work_id: existing,
            });
        }
        let (work, _entered) =
            GameWork::enter_manually(self.ids.next(), item.file().file_name(), self.clock.now());
        let _linked = item.link_to(work.id());
        self.recorder.record(&work, None, Some(&item)).await?;
        Ok(LinkGameWorkOutput {
            game_work_id: work.id(),
        })
    }
}

#[cfg(test)]
mod tests {
    use chrono::{TimeZone, Utc};

    use super::*;
    use crate::domain::catalog_recorder::fakes::FakeCatalogRecorder;
    use crate::domain::game_work::GameWorkRepository;
    use crate::domain::game_work::fakes::FakeGameWorkRepository;
    use crate::domain::inventory_item::fakes::FakeInventoryItemRepository;
    use crate::domain::inventory_item::{InventoryItem, OrganizationStatus, ScannedFile};
    use crate::domain::library_path::LibraryPath;
    use crate::kernel::{FakeClock, FakeIdGenerator, Timestamp};

    fn now() -> Timestamp {
        Timestamp::from_utc(Utc.with_ymd_and_hms(2026, 3, 1, 12, 0, 0).unwrap())
    }

    async fn setup() -> (
        CreateManualGameWorkUseCase,
        Arc<FakeInventoryItemRepository>,
        Arc<FakeGameWorkRepository>,
        InventoryItemId,
    ) {
        let inventory = Arc::new(FakeInventoryItemRepository::default());
        let works = Arc::new(FakeGameWorkRepository::default());
        let item_id = InventoryItemId::from_raw(uuid::Uuid::from_u128(99));
        let file = ScannedFile::new(
            LibraryPath::parse("未分類/Odd Game.zip").unwrap(),
            "Odd Game.zip".into(),
            Some("zip".into()),
            1,
            now(),
            Some("未分類".into()),
        );
        let (item, _discovered) = InventoryItem::discover(item_id, file);
        inventory.save(&item).await.unwrap();
        let recorder = Arc::new(FakeCatalogRecorder::new(works.clone(), inventory.clone()));
        let use_case = CreateManualGameWorkUseCase::new(
            inventory.clone(),
            recorder,
            Arc::new(FakeIdGenerator::default()),
            Arc::new(FakeClock::at(now())),
        );
        (use_case, inventory, works, item_id)
    }

    #[tokio::test]
    async fn creates_a_work_titled_after_the_file_and_links_it() {
        let (use_case, inventory, works, item_id) = setup().await;
        let output = use_case
            .execute(CreateManualGameWorkInput {
                inventory_item_id: item_id,
            })
            .await
            .unwrap();
        let work = works
            .find_by_id(output.game_work_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(work.identity().display_title(), "Odd Game.zip");
        assert_eq!(work.enriched_at(), Some(now()));
        let item = inventory.find_by_id(item_id).await.unwrap().unwrap();
        assert_eq!(item.work_id(), Some(output.game_work_id));
        assert_eq!(item.status(), OrganizationStatus::Confirmed);
    }

    #[tokio::test]
    async fn a_linked_file_keeps_its_work() {
        let (use_case, _inventory, works, item_id) = setup().await;
        let first = use_case
            .execute(CreateManualGameWorkInput {
                inventory_item_id: item_id,
            })
            .await
            .unwrap();
        let second = use_case
            .execute(CreateManualGameWorkInput {
                inventory_item_id: item_id,
            })
            .await
            .unwrap();
        assert_eq!(first.game_work_id, second.game_work_id);
        assert_eq!(works.all().len(), 1);
    }
}
