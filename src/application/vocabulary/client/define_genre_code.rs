use std::sync::Arc;

use serde::Deserialize;

use super::GenreCodeOutput;
use crate::domain::genre_code::{
    CodeDefinition, CodeLabel, CodeSpelling, GenreCode, GenreCodeRepository,
    GenreCodeRepositoryError,
};
use crate::domain::vocabulary::{VocabularyError, VocabularySource, VocabularySourceError};
use crate::kernel::{ErrorCode, ErrorKind};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DefineGenreCodeInput {
    pub(crate) code: CodeSpelling,
    pub(crate) label: CodeLabel,
    pub(crate) category_id: String,
    pub(crate) family: Option<String>,
    #[serde(default)]
    pub(crate) accepts_prefix: bool,
    #[serde(default)]
    pub(crate) aliases: Vec<String>,
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum DefineGenreCodeError {
    #[error(transparent)]
    Vocabulary(#[from] VocabularyError),
    #[error(transparent)]
    Source(#[from] VocabularySourceError),
    #[error(transparent)]
    Codes(#[from] GenreCodeRepositoryError),
}

impl ErrorCode for DefineGenreCodeError {
    fn code(&self) -> &'static str {
        match self {
            Self::Vocabulary(e) => e.code(),
            Self::Source(_) | Self::Codes(_) => "grimoire.unavailable",
        }
    }

    fn kind(&self) -> ErrorKind {
        match self {
            Self::Vocabulary(e) => e.kind(),
            Self::Source(_) | Self::Codes(_) => ErrorKind::Unavailable,
        }
    }
}

/// Adds a genre code or edits one (label, category, family, aliases).
pub(crate) struct DefineGenreCodeUseCase {
    vocabulary: Arc<dyn VocabularySource>,
    codes: Arc<dyn GenreCodeRepository>,
}

impl DefineGenreCodeUseCase {
    pub(crate) fn new(
        vocabulary: Arc<dyn VocabularySource>,
        codes: Arc<dyn GenreCodeRepository>,
    ) -> Self {
        Self { vocabulary, codes }
    }

    #[tracing::instrument(name = "use_case.define_genre_code", skip(self))]
    pub(crate) async fn execute(
        &self,
        input: DefineGenreCodeInput,
    ) -> Result<GenreCodeOutput, DefineGenreCodeError> {
        let vocabulary = self.vocabulary.current().await?;
        let definition = CodeDefinition::new(input.code, input.label, input.category_id)
            .with_family(input.family)
            .with_prefixes(input.accepts_prefix)
            .with_aliases(input.aliases);
        let (code, _defined) = GenreCode::define(&vocabulary, definition)?;
        self.codes.save(&code).await?;
        Ok(GenreCodeOutput::from(&code))
    }
}
