use std::sync::Arc;

use serde::Deserialize;

use super::{GameWorkEnrichmentAppService, LinkGameWorkOutput};
use crate::domain::catalog_recorder::{CatalogRecorder, CatalogRecorderError};
use crate::domain::game_work::{
    DetailPrecedence, GameWorkFactory, GameWorkRepository, GameWorkRepositoryError, WorkLookup,
};
use crate::domain::inventory_item::{
    InventoryItemId, InventoryItemRepository, InventoryItemRepositoryError,
};
use crate::domain::metadata_candidate::{
    MetadataCandidateId, MetadataCandidateRepository, MetadataCandidateRepositoryError,
};
use crate::kernel::{ErrorCode, ErrorKind, IdGenerator};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ConfirmMetadataCandidateInput {
    pub(crate) candidate_id: MetadataCandidateId,
    pub(crate) inventory_item_id: InventoryItemId,
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum ConfirmMetadataCandidateError {
    #[error("inventory item {0} not found")]
    ItemNotFound(InventoryItemId),
    #[error("metadata candidate {0} not found")]
    CandidateNotFound(MetadataCandidateId),
    #[error(transparent)]
    Inventory(#[from] InventoryItemRepositoryError),
    #[error(transparent)]
    Candidates(#[from] MetadataCandidateRepositoryError),
    #[error(transparent)]
    Works(#[from] GameWorkRepositoryError),
    #[error(transparent)]
    Recorder(#[from] CatalogRecorderError),
}

impl ErrorCode for ConfirmMetadataCandidateError {
    fn code(&self) -> &'static str {
        match self {
            Self::ItemNotFound(_) => "inventory_item.not_found",
            Self::CandidateNotFound(_) => "metadata_candidate.not_found",
            Self::Inventory(_) | Self::Candidates(_) | Self::Works(_) | Self::Recorder(_) => {
                "grimoire.unavailable"
            }
        }
    }

    fn kind(&self) -> ErrorKind {
        match self {
            Self::ItemNotFound(_) | Self::CandidateNotFound(_) => ErrorKind::NotFound,
            Self::Inventory(_) | Self::Candidates(_) | Self::Works(_) | Self::Recorder(_) => {
                ErrorKind::Unavailable
            }
        }
    }
}

/// Links a file to the work of a search hit, registering the work on first sight. The site's
/// page is merged in as a bonus: a site that is down does not block the confirm.
pub(crate) struct ConfirmMetadataCandidateUseCase {
    inventory: Arc<dyn InventoryItemRepository>,
    candidates: Arc<dyn MetadataCandidateRepository>,
    factory: GameWorkFactory,
    enrichment: Arc<GameWorkEnrichmentAppService>,
    recorder: Arc<dyn CatalogRecorder>,
    ids: Arc<dyn IdGenerator>,
}

impl ConfirmMetadataCandidateUseCase {
    pub(crate) fn new(
        inventory: Arc<dyn InventoryItemRepository>,
        candidates: Arc<dyn MetadataCandidateRepository>,
        works: Arc<dyn GameWorkRepository>,
        enrichment: Arc<GameWorkEnrichmentAppService>,
        recorder: Arc<dyn CatalogRecorder>,
        ids: Arc<dyn IdGenerator>,
    ) -> Self {
        Self {
            inventory,
            candidates,
            factory: GameWorkFactory::new(works),
            enrichment,
            recorder,
            ids,
        }
    }

    #[tracing::instrument(name = "use_case.confirm_metadata_candidate", skip(self))]
    pub(crate) async fn execute(
        &self,
        input: ConfirmMetadataCandidateInput,
    ) -> Result<LinkGameWorkOutput, ConfirmMetadataCandidateError> {
        let mut item = self
            .inventory
            .find_by_id(input.inventory_item_id)
            .await?
            .ok_or(ConfirmMetadataCandidateError::ItemNotFound(
                input.inventory_item_id,
            ))?;
        let candidate = self
            .candidates
            .find_by_id(input.candidate_id)
            .await?
            .ok_or(ConfirmMetadataCandidateError::CandidateNotFound(
                input.candidate_id,
            ))?;
        let provider = candidate.provider();
        let found = candidate.source_match();
        let lookup = self
            .factory
            .find_or_register(
                self.ids.next(),
                provider,
                found.source_work_id(),
                found.title(),
                found.circle().map(str::to_owned),
            )
            .await?;
        let mut work = match lookup {
            WorkLookup::Existing(mut work) => {
                if let Some(cover) = found.cover_url() {
                    let _prefilled = work.prefill_cover(cover.to_owned());
                }
                work
            }
            WorkLookup::Registered(work) => work,
        };
        let source_data = match self
            .enrichment
            .fetch(provider, found.source_work_id())
            .await
        {
            Ok(Some(detail)) => {
                Some(
                    self.enrichment
                        .apply(&mut work, provider, detail, DetailPrecedence::Merge),
                )
            }
            _ => None,
        };
        let _linked = item.link_to(work.id());
        self.recorder
            .record(&work, source_data.as_ref(), Some(&item))
            .await?;
        Ok(LinkGameWorkOutput {
            game_work_id: work.id(),
        })
    }
}
