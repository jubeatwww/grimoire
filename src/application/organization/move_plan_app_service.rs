use std::sync::Arc;

use crate::domain::file_placement::{PlacementSource, PlacementSourceError, protected_folders};
use crate::domain::inventory_item::InventoryItemId;
use crate::domain::library_files::{LibraryFiles, LibraryFilesError};
use crate::domain::library_folders::ProtectedFolders;
use crate::domain::move_plan::MovePlan;
use crate::domain::vocabulary::{VocabularySource, VocabularySourceError};

#[derive(Debug, thiserror::Error)]
pub(crate) enum MovePlanError {
    #[error(transparent)]
    Placements(#[from] PlacementSourceError),
    #[error(transparent)]
    Files(#[from] LibraryFilesError),
    #[error(transparent)]
    Vocabulary(#[from] VocabularySourceError),
}

/// Planning moves from the records and the disk, shared by the plan view, the preview and the
/// run itself (which never trusts paths from the client).
pub(crate) struct MovePlanAppService {
    placements: Arc<dyn PlacementSource>,
    files: Arc<dyn LibraryFiles>,
    vocabulary: Arc<dyn VocabularySource>,
}

impl MovePlanAppService {
    pub(crate) fn new(
        placements: Arc<dyn PlacementSource>,
        files: Arc<dyn LibraryFiles>,
        vocabulary: Arc<dyn VocabularySource>,
    ) -> Self {
        Self {
            placements,
            files,
            vocabulary,
        }
    }

    /// Every file of a confirmed work that is not where it belongs, with conflicts marked.
    /// `only` limits the result to those files; conflicts are judged against the whole plan.
    pub(crate) async fn plan(
        &self,
        only: Option<&[InventoryItemId]>,
    ) -> Result<MovePlan, MovePlanError> {
        let placements = self.placements.confirmed_placements().await?;
        let mut plan = MovePlan::draft(&placements);
        let targets = plan.targets();
        let occupants = self.placements.occupants(&targets).await?;
        let on_disk = self.files.exist(&targets).await?;
        plan.mark_conflicts(&occupants, &on_disk);
        if let Some(only) = only {
            plan.keep_only(only);
        }
        Ok(plan)
    }

    pub(crate) async fn protected_folders(&self) -> Result<ProtectedFolders, MovePlanError> {
        Ok(protected_folders(&self.vocabulary.current().await?))
    }
}
