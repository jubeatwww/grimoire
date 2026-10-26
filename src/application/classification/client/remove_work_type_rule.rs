use std::sync::Arc;

use crate::domain::metadata_source::MetadataProvider;
use crate::domain::work_type_rule::{WorkTypeRuleRepository, WorkTypeRuleRepositoryError};
use crate::kernel::{ErrorCode, ErrorKind};

/// The rule's key; the work type exactly as stored.
#[derive(Debug)]
pub(crate) struct RemoveWorkTypeRuleInput {
    pub(crate) source: MetadataProvider,
    pub(crate) work_type: String,
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum RemoveWorkTypeRuleError {
    #[error("no such rule")]
    NotFound,
    #[error(transparent)]
    Rules(#[from] WorkTypeRuleRepositoryError),
}

impl ErrorCode for RemoveWorkTypeRuleError {
    fn code(&self) -> &'static str {
        match self {
            Self::NotFound => "work_type_rule.not_found",
            Self::Rules(_) => "grimoire.unavailable",
        }
    }

    fn kind(&self) -> ErrorKind {
        match self {
            Self::NotFound => ErrorKind::NotFound,
            Self::Rules(_) => ErrorKind::Unavailable,
        }
    }
}

pub(crate) struct RemoveWorkTypeRuleUseCase {
    rules: Arc<dyn WorkTypeRuleRepository>,
}

impl RemoveWorkTypeRuleUseCase {
    pub(crate) fn new(rules: Arc<dyn WorkTypeRuleRepository>) -> Self {
        Self { rules }
    }

    #[tracing::instrument(name = "use_case.remove_work_type_rule", skip(self))]
    pub(crate) async fn execute(
        &self,
        input: RemoveWorkTypeRuleInput,
    ) -> Result<(), RemoveWorkTypeRuleError> {
        let rule = self
            .rules
            .find_by_source_and_work_type(input.source, &input.work_type)
            .await?
            .ok_or(RemoveWorkTypeRuleError::NotFound)?;
        self.rules.delete(&rule).await?;
        Ok(())
    }
}
