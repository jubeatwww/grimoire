use std::sync::Arc;

use serde::Deserialize;

use super::{GameWorkEnrichmentAppService, LinkGameWorkOutput};
use crate::domain::catalog_recorder::{CatalogRecorder, CatalogRecorderError};
use crate::domain::game_work::{
    DetailPrecedence, GameWorkFactory, GameWorkRepository, GameWorkRepositoryError,
};
use crate::domain::inventory_item::{
    InventoryItemId, InventoryItemRepository, InventoryItemRepositoryError,
};
use crate::domain::metadata_source::{MetadataProvider, MetadataSourceError};
use crate::kernel::{ErrorCode, ErrorKind, IdGenerator};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LinkGameWorkInput {
    pub(crate) inventory_item_id: InventoryItemId,
    /// A pasted DLsite RJ code, VNDB id, or a store URL of any of the sites.
    pub(crate) workno_or_url: String,
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum LinkGameWorkError {
    #[error("inventory item {0} not found")]
    ItemNotFound(InventoryItemId),
    #[error("no DLsite, VNDB or Steam work id in {0:?}")]
    Unrecognized(String),
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

impl ErrorCode for LinkGameWorkError {
    fn code(&self) -> &'static str {
        match self {
            Self::ItemNotFound(_) => "inventory_item.not_found",
            Self::Unrecognized(_) => "game_work.unrecognized_work_id",
            Self::UnknownWork { .. } => "game_work.unknown_on_source",
            Self::Source(_) => "metadata_source.unavailable",
            Self::Inventory(_) | Self::Works(_) | Self::Recorder(_) => "grimoire.unavailable",
        }
    }

    fn kind(&self) -> ErrorKind {
        match self {
            Self::ItemNotFound(_) | Self::UnknownWork { .. } => ErrorKind::NotFound,
            Self::Unrecognized(_) => ErrorKind::Invalid,
            Self::Source(_) | Self::Inventory(_) | Self::Works(_) | Self::Recorder(_) => {
                ErrorKind::Unavailable
            }
        }
    }
}

/// Links a file to a work by a pasted id or URL. The user picked this source, so its page
/// overwrites the work's detail and title.
pub(crate) struct LinkGameWorkUseCase {
    inventory: Arc<dyn InventoryItemRepository>,
    factory: GameWorkFactory,
    enrichment: Arc<GameWorkEnrichmentAppService>,
    recorder: Arc<dyn CatalogRecorder>,
    ids: Arc<dyn IdGenerator>,
}

impl LinkGameWorkUseCase {
    pub(crate) fn new(
        inventory: Arc<dyn InventoryItemRepository>,
        works: Arc<dyn GameWorkRepository>,
        enrichment: Arc<GameWorkEnrichmentAppService>,
        recorder: Arc<dyn CatalogRecorder>,
        ids: Arc<dyn IdGenerator>,
    ) -> Self {
        Self {
            inventory,
            factory: GameWorkFactory::new(works),
            enrichment,
            recorder,
            ids,
        }
    }

    #[tracing::instrument(name = "use_case.link_game_work", skip(self))]
    pub(crate) async fn execute(
        &self,
        input: LinkGameWorkInput,
    ) -> Result<LinkGameWorkOutput, LinkGameWorkError> {
        let mut item = self
            .inventory
            .find_by_id(input.inventory_item_id)
            .await?
            .ok_or(LinkGameWorkError::ItemNotFound(input.inventory_item_id))?;
        let (provider, external_id) = self
            .enrichment
            .recognize(&input.workno_or_url)
            .ok_or_else(|| LinkGameWorkError::Unrecognized(input.workno_or_url.clone()))?;
        let detail = self
            .enrichment
            .fetch(provider, &external_id)
            .await?
            .ok_or_else(|| LinkGameWorkError::UnknownWork {
                provider,
                id: external_id.clone(),
            })?;
        let title = detail.work_name().unwrap_or(&external_id).to_owned();
        let circle = detail.maker_name().map(str::to_owned);
        let mut work = self
            .factory
            .find_or_register(self.ids.next(), provider, &external_id, &title, circle)
            .await?
            .into_work();
        let source_data =
            self.enrichment
                .apply(&mut work, provider, detail, DetailPrecedence::Overwrite);
        let _linked = item.link_to(work.id());
        self.recorder
            .record(&work, Some(&source_data), Some(&item))
            .await?;
        Ok(LinkGameWorkOutput {
            game_work_id: work.id(),
        })
    }
}
