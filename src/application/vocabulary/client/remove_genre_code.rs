use std::sync::Arc;

use serde::Deserialize;

use crate::domain::genre_code::{GenreCodeRepository, GenreCodeRepositoryError};
use crate::domain::vocabulary::{
    GenreCodeUsage, GenreCodeUsageError, VocabularyError, VocabularySource, VocabularySourceError,
};
use crate::kernel::{ErrorCode, ErrorKind};

#[derive(Debug, Deserialize)]
pub(crate) struct RemoveGenreCodeInput {
    pub(crate) code: String,
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum RemoveGenreCodeError {
    #[error("no such code: {0}")]
    NotFound(String),
    #[error(transparent)]
    Vocabulary(#[from] VocabularyError),
    #[error(transparent)]
    Source(#[from] VocabularySourceError),
    #[error(transparent)]
    Usage(#[from] GenreCodeUsageError),
    #[error(transparent)]
    Codes(#[from] GenreCodeRepositoryError),
}

impl ErrorCode for RemoveGenreCodeError {
    fn code(&self) -> &'static str {
        match self {
            Self::NotFound(_) => "genre_code.not_found",
            Self::Vocabulary(e) => e.code(),
            Self::Source(_) | Self::Usage(_) | Self::Codes(_) => "grimoire.unavailable",
        }
    }

    fn kind(&self) -> ErrorKind {
        match self {
            Self::NotFound(_) => ErrorKind::NotFound,
            Self::Vocabulary(e) => e.kind(),
            Self::Source(_) | Self::Usage(_) | Self::Codes(_) => ErrorKind::Unavailable,
        }
    }
}

/// Removes a code nobody uses. A category's own code is never removed: confirming a category
/// falls back to it as the work's code.
pub(crate) struct RemoveGenreCodeUseCase {
    vocabulary: Arc<dyn VocabularySource>,
    usage: Arc<dyn GenreCodeUsage>,
    codes: Arc<dyn GenreCodeRepository>,
}

impl RemoveGenreCodeUseCase {
    pub(crate) fn new(
        vocabulary: Arc<dyn VocabularySource>,
        usage: Arc<dyn GenreCodeUsage>,
        codes: Arc<dyn GenreCodeRepository>,
    ) -> Self {
        Self {
            vocabulary,
            usage,
            codes,
        }
    }

    #[tracing::instrument(name = "use_case.remove_genre_code", skip(self))]
    pub(crate) async fn execute(
        &self,
        input: RemoveGenreCodeInput,
    ) -> Result<(), RemoveGenreCodeError> {
        let vocabulary = self.vocabulary.current().await?;
        let in_works = self.usage.codes_in_works().await?;
        let in_rules = self.usage.codes_in_rules().await?;
        vocabulary.check_removal(&input.code, &in_works, &in_rules)?;
        self.codes
            .find_by_code(&input.code)
            .await?
            .ok_or_else(|| RemoveGenreCodeError::NotFound(input.code.clone()))?;
        self.codes.delete(&input.code).await?;
        Ok(())
    }
}
