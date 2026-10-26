use std::sync::Arc;

use serde::Deserialize;

use super::ConfirmClassificationsOutput;
use crate::domain::game_work::{
    ClassificationRecorder, ClassificationRecorderError, GameWorkId, GameWorkRepository,
    GameWorkRepositoryError,
};
use crate::domain::vocabulary::{
    CategoryId, VocabularyError, VocabularySource, VocabularySourceError,
};
use crate::kernel::{Clock, ErrorCode, ErrorKind};

#[derive(Debug, Deserialize)]
pub(crate) struct ConfirmClassificationsInput {
    pub(crate) items: Vec<ClassificationItemInput>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ClassificationItemInput {
    pub(crate) game_work_id: GameWorkId,
    /// `None` (or blank) clears the category and its confirmation.
    #[serde(default, deserialize_with = "CategoryId::deserialize_optional")]
    pub(crate) primary_category: Option<CategoryId>,
    #[serde(default)]
    pub(crate) genre_facets: Vec<String>,
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum ConfirmClassificationsError {
    #[error("no such game work: {0}")]
    NotFound(GameWorkId),
    #[error(transparent)]
    Vocabulary(#[from] VocabularyError),
    #[error(transparent)]
    Source(#[from] VocabularySourceError),
    #[error(transparent)]
    Works(#[from] GameWorkRepositoryError),
    #[error(transparent)]
    Recorder(#[from] ClassificationRecorderError),
}

impl ErrorCode for ConfirmClassificationsError {
    fn code(&self) -> &'static str {
        match self {
            Self::NotFound(_) => "game_work.not_found",
            Self::Vocabulary(e) => e.code(),
            Self::Source(_) | Self::Works(_) | Self::Recorder(_) => "grimoire.unavailable",
        }
    }

    fn kind(&self) -> ErrorKind {
        match self {
            Self::NotFound(_) => ErrorKind::NotFound,
            Self::Vocabulary(e) => e.kind(),
            Self::Source(_) | Self::Works(_) | Self::Recorder(_) => ErrorKind::Unavailable,
        }
    }
}

/// Sets the primary category and genre codes of one or more works. Any manual change counts as
/// confirmation. Every item is checked before anything is written, and all are written together.
pub(crate) struct ConfirmClassificationsUseCase {
    vocabulary: Arc<dyn VocabularySource>,
    works: Arc<dyn GameWorkRepository>,
    recorder: Arc<dyn ClassificationRecorder>,
    clock: Arc<dyn Clock>,
}

impl ConfirmClassificationsUseCase {
    pub(crate) fn new(
        vocabulary: Arc<dyn VocabularySource>,
        works: Arc<dyn GameWorkRepository>,
        recorder: Arc<dyn ClassificationRecorder>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            vocabulary,
            works,
            recorder,
            clock,
        }
    }

    #[tracing::instrument(name = "use_case.confirm_classifications", skip(self))]
    pub(crate) async fn execute(
        &self,
        input: ConfirmClassificationsInput,
    ) -> Result<ConfirmClassificationsOutput, ConfirmClassificationsError> {
        let vocabulary = self.vocabulary.current().await?;
        let at = self.clock.now();
        let mut works = Vec::with_capacity(input.items.len());
        for item in &input.items {
            let mut work = self
                .works
                .find_by_id(item.game_work_id)
                .await?
                .ok_or(ConfirmClassificationsError::NotFound(item.game_work_id))?;
            let _classified = work.classify(
                &vocabulary,
                item.primary_category.as_ref(),
                &item.genre_facets,
                at,
            )?;
            works.push(work);
        }
        self.recorder.record(&works).await?;
        Ok(ConfirmClassificationsOutput {
            updated: works.len(),
        })
    }
}
