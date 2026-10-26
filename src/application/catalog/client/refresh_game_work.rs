use std::sync::Arc;

use serde::Deserialize;

use super::GameWorkEnrichmentAppService;
use crate::domain::catalog_recorder::{CatalogRecorder, CatalogRecorderError};
use crate::domain::game_work::{DetailPrecedence, GameWorkRepository, GameWorkRepositoryError};
use crate::domain::inventory_item::{
    InventoryItemId, InventoryItemRepository, InventoryItemRepositoryError,
};
use crate::domain::metadata_source::{MetadataProvider, MetadataSourceError};
use crate::kernel::{ErrorCode, ErrorKind};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RefreshGameWorkInput {
    pub(crate) inventory_item_id: InventoryItemId,
    pub(crate) source: MetadataProvider,
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum RefreshGameWorkError {
    #[error("inventory item {0} has no work")]
    WorkNotFound(InventoryItemId),
    #[error("the work is not linked to {}", .0.name())]
    NotLinked(MetadataProvider),
    #[error("{} has no work {id}", provider.name())]
    UnknownWork {
        provider: MetadataProvider,
        id: String,
    },
    #[error(transparent)]
    Source(#[from] MetadataSourceError),
    #[error(transparent)]
    Inventory(#[from] InventoryItemRepositoryError),
    #[error(transparent)]
    Works(#[from] GameWorkRepositoryError),
    #[error(transparent)]
    Recorder(#[from] CatalogRecorderError),
}

impl ErrorCode for RefreshGameWorkError {
    fn code(&self) -> &'static str {
        match self {
            Self::WorkNotFound(_) => "game_work.not_found",
            Self::NotLinked(_) => "game_work.not_linked_to_source",
            Self::UnknownWork { .. } => "game_work.unknown_on_source",
            Self::Source(_) => "metadata_source.unavailable",
            Self::Inventory(_) | Self::Works(_) | Self::Recorder(_) => "grimoire.unavailable",
        }
    }

    fn kind(&self) -> ErrorKind {
        match self {
            Self::WorkNotFound(_) | Self::UnknownWork { .. } => ErrorKind::NotFound,
            Self::NotLinked(_) => ErrorKind::Invalid,
            Self::Source(_) | Self::Inventory(_) | Self::Works(_) | Self::Recorder(_) => {
                ErrorKind::Unavailable
            }
        }
    }
}

/// Reads the work's page from one of its sites again and overwrites the work's detail with it.
pub(crate) struct RefreshGameWorkUseCase {
    inventory: Arc<dyn InventoryItemRepository>,
    works: Arc<dyn GameWorkRepository>,
    enrichment: Arc<GameWorkEnrichmentAppService>,
    recorder: Arc<dyn CatalogRecorder>,
}

impl RefreshGameWorkUseCase {
    pub(crate) fn new(
        inventory: Arc<dyn InventoryItemRepository>,
        works: Arc<dyn GameWorkRepository>,
        enrichment: Arc<GameWorkEnrichmentAppService>,
        recorder: Arc<dyn CatalogRecorder>,
    ) -> Self {
        Self {
            inventory,
            works,
            enrichment,
            recorder,
        }
    }

    #[tracing::instrument(name = "use_case.refresh_game_work", skip(self))]
    pub(crate) async fn execute(
        &self,
        input: RefreshGameWorkInput,
    ) -> Result<(), RefreshGameWorkError> {
        let not_found = RefreshGameWorkError::WorkNotFound(input.inventory_item_id);
        let item = self.inventory.find_by_id(input.inventory_item_id).await?;
        let Some(work_id) = item.and_then(|i| i.work_id()) else {
            return Err(not_found);
        };
        let mut work = self.works.find_by_id(work_id).await?.ok_or(not_found)?;
        let external_id = work
            .external_ids()
            .get(input.source)
            .ok_or(RefreshGameWorkError::NotLinked(input.source))?
            .to_owned();
        let detail = self
            .enrichment
            .fetch(input.source, &external_id)
            .await?
            .ok_or(RefreshGameWorkError::UnknownWork {
                provider: input.source,
                id: external_id,
            })?;
        let source_data =
            self.enrichment
                .apply(&mut work, input.source, detail, DetailPrecedence::Overwrite);
        self.recorder
            .record(&work, Some(&source_data), None)
            .await?;
        Ok(())
    }
}
