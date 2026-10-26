use std::sync::Arc;

use super::VocabularyOutput;
use crate::domain::vocabulary::{VocabularySource, VocabularySourceError};
use crate::kernel::{ErrorCode, ErrorKind};

#[derive(Debug, thiserror::Error)]
pub(crate) enum VocabularyClientQueryError {
    #[error(transparent)]
    Vocabulary(#[from] VocabularySourceError),
}

impl ErrorCode for VocabularyClientQueryError {
    fn code(&self) -> &'static str {
        "grimoire.unavailable"
    }

    fn kind(&self) -> ErrorKind {
        ErrorKind::Unavailable
    }
}

pub(crate) struct VocabularyClientQueryUseCase {
    vocabulary: Arc<dyn VocabularySource>,
}

impl VocabularyClientQueryUseCase {
    pub(crate) fn new(vocabulary: Arc<dyn VocabularySource>) -> Self {
        Self { vocabulary }
    }

    #[tracing::instrument(name = "use_case.find_vocabulary", skip(self))]
    pub(crate) async fn find_vocabulary(
        &self,
    ) -> Result<VocabularyOutput, VocabularyClientQueryError> {
        Ok(VocabularyOutput::from(&self.vocabulary.current().await?))
    }
}
