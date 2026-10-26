use std::sync::Arc;

use serde::Deserialize;

use super::{
    InboxOutput, ListMoveBatchesOutput, ListOtherFilesOutput, MoveBatchOutput, MovePlanOutput,
    PreviewMovesOutput,
};
use crate::application::organization::{MovePlanAppService, MovePlanError};
use crate::domain::file_placement::{INBOX_FOLDER, inbox};
use crate::domain::inventory_item::InventoryItemId;
use crate::domain::library_folders::{LibraryFolders, LibraryFoldersError};
use crate::domain::library_path::LibraryPath;
use crate::domain::move_batch::{MoveBatchId, MoveBatchRepository, MoveBatchRepositoryError};
use crate::kernel::{ErrorCode, ErrorKind};

const BATCH_HISTORY_LIMIT: i64 = 50;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PreviewMovesInput {
    pub(crate) inventory_item_ids: Vec<InventoryItemId>,
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum OrganizationClientQueryError {
    #[error("no such batch")]
    BatchNotFound(MoveBatchId),
    #[error(transparent)]
    Plan(#[from] MovePlanError),
    #[error(transparent)]
    Batches(#[from] MoveBatchRepositoryError),
    #[error(transparent)]
    Folders(#[from] LibraryFoldersError),
}

impl ErrorCode for OrganizationClientQueryError {
    fn code(&self) -> &'static str {
        match self {
            Self::BatchNotFound(_) => "move_batch.not_found",
            Self::Plan(_) | Self::Batches(_) | Self::Folders(_) => "grimoire.unavailable",
        }
    }

    fn kind(&self) -> ErrorKind {
        match self {
            Self::BatchNotFound(_) => ErrorKind::NotFound,
            Self::Plan(_) | Self::Batches(_) | Self::Folders(_) => ErrorKind::Unavailable,
        }
    }
}

pub(crate) struct OrganizationClientQueryUseCase {
    plans: Arc<MovePlanAppService>,
    batches: Arc<dyn MoveBatchRepository>,
    folders: Arc<dyn LibraryFolders>,
}

impl OrganizationClientQueryUseCase {
    pub(crate) fn new(
        plans: Arc<MovePlanAppService>,
        batches: Arc<dyn MoveBatchRepository>,
        folders: Arc<dyn LibraryFolders>,
    ) -> Self {
        Self {
            plans,
            batches,
            folders,
        }
    }

    #[tracing::instrument(name = "use_case.find_move_plan", skip(self))]
    pub(crate) async fn find_plan(&self) -> Result<MovePlanOutput, OrganizationClientQueryError> {
        Ok(MovePlanOutput::from(&self.plans.plan(None).await?))
    }

    /// The selected moves and the folders they would empty. Reads, never writes.
    #[tracing::instrument(name = "use_case.preview_moves", skip(self))]
    pub(crate) async fn preview(
        &self,
        input: PreviewMovesInput,
    ) -> Result<PreviewMovesOutput, OrganizationClientQueryError> {
        let plan = self.plans.plan(Some(&input.inventory_item_ids)).await?;
        let protected = self.plans.protected_folders().await?;
        let movable = plan.moves().iter().filter(|m| m.conflict().is_none());
        let (leaving, arriving): (Vec<LibraryPath>, Vec<LibraryPath>) =
            movable.map(|m| (m.from().clone(), m.to().clone())).unzip();
        let removable_dirs = self
            .folders
            .preview_prune(leaving, arriving, &protected)
            .await?;
        Ok(PreviewMovesOutput::new(&plan, removable_dirs))
    }

    #[tracing::instrument(name = "use_case.list_move_batches", skip(self))]
    pub(crate) async fn list_batches(
        &self,
    ) -> Result<ListMoveBatchesOutput, OrganizationClientQueryError> {
        let batches = self.batches.list_recent(BATCH_HISTORY_LIMIT).await?;
        Ok(ListMoveBatchesOutput::from(batches.as_slice()))
    }

    #[tracing::instrument(name = "use_case.find_move_batch", skip(self))]
    pub(crate) async fn find_batch(
        &self,
        id: MoveBatchId,
    ) -> Result<MoveBatchOutput, OrganizationClientQueryError> {
        let batch = self
            .batches
            .find_by_id(id)
            .await?
            .ok_or(OrganizationClientQueryError::BatchNotFound(id))?;
        Ok(MoveBatchOutput::from(&batch))
    }

    #[tracing::instrument(name = "use_case.find_inbox", skip(self))]
    pub(crate) async fn find_inbox(&self) -> InboxOutput {
        InboxOutput {
            folder: INBOX_FOLDER,
            exists: self.folders.folder_exists(&inbox()).await,
        }
    }

    /// Everything the scanner ignores (not zip or rar), so nothing is silently left behind in
    /// emptied legacy folders.
    #[tracing::instrument(name = "use_case.list_other_files", skip(self))]
    pub(crate) async fn list_other_files(
        &self,
    ) -> Result<ListOtherFilesOutput, OrganizationClientQueryError> {
        let entries = self.folders.report_other_files().await?;
        Ok(ListOtherFilesOutput::from(entries.as_slice()))
    }
}
