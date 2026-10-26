use std::sync::Arc;

use serde::Deserialize;

use super::WorkTypeRuleOutput;
use crate::domain::metadata_source::MetadataProvider;
use crate::domain::vocabulary::{VocabularySource, VocabularySourceError};
use crate::domain::work_type_rule::{
    WorkType, WorkTypeRule, WorkTypeRuleError, WorkTypeRuleRepository, WorkTypeRuleRepositoryError,
};
use crate::kernel::{ErrorCode, ErrorKind};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DefineWorkTypeRuleInput {
    pub(crate) source: MetadataProvider,
    pub(crate) work_type: WorkType,
    /// `None` means "decide from the tags".
    pub(crate) code: Option<String>,
    pub(crate) confident: bool,
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum DefineWorkTypeRuleError {
    #[error(transparent)]
    Rule(#[from] WorkTypeRuleError),
    #[error(transparent)]
    Vocabulary(#[from] VocabularySourceError),
    #[error(transparent)]
    Rules(#[from] WorkTypeRuleRepositoryError),
}

impl ErrorCode for DefineWorkTypeRuleError {
    fn code(&self) -> &'static str {
        match self {
            Self::Rule(e) => e.code(),
            Self::Vocabulary(_) | Self::Rules(_) => "grimoire.unavailable",
        }
    }

    fn kind(&self) -> ErrorKind {
        match self {
            Self::Rule(e) => e.kind(),
            Self::Vocabulary(_) | Self::Rules(_) => ErrorKind::Unavailable,
        }
    }
}

/// Defines a work type rule, replacing the one for the same source and work type.
pub(crate) struct DefineWorkTypeRuleUseCase {
    vocabulary: Arc<dyn VocabularySource>,
    rules: Arc<dyn WorkTypeRuleRepository>,
}

impl DefineWorkTypeRuleUseCase {
    pub(crate) fn new(
        vocabulary: Arc<dyn VocabularySource>,
        rules: Arc<dyn WorkTypeRuleRepository>,
    ) -> Self {
        Self { vocabulary, rules }
    }

    #[tracing::instrument(name = "use_case.define_work_type_rule", skip(self))]
    pub(crate) async fn execute(
        &self,
        input: DefineWorkTypeRuleInput,
    ) -> Result<WorkTypeRuleOutput, DefineWorkTypeRuleError> {
        let vocabulary = self.vocabulary.current().await?;
        let (rule, _defined) = WorkTypeRule::define(
            &vocabulary,
            input.source,
            &input.work_type,
            input.code.as_deref(),
            input.confident,
        )?;
        self.rules.save(&rule).await?;
        Ok(WorkTypeRuleOutput::from(&rule))
    }
}
