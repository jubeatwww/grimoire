use std::sync::Arc;

use crate::domain::keyword_rule::{
    KeywordRuleId, KeywordRuleRepository, KeywordRuleRepositoryError,
};
use crate::kernel::{ErrorCode, ErrorKind};

#[derive(Debug)]
pub(crate) struct RemoveKeywordRuleInput {
    pub(crate) id: KeywordRuleId,
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum RemoveKeywordRuleError {
    #[error("no such rule")]
    NotFound(KeywordRuleId),
    #[error(transparent)]
    Rules(#[from] KeywordRuleRepositoryError),
}

impl ErrorCode for RemoveKeywordRuleError {
    fn code(&self) -> &'static str {
        match self {
            Self::NotFound(_) => "keyword_rule.not_found",
            Self::Rules(_) => "grimoire.unavailable",
        }
    }

    fn kind(&self) -> ErrorKind {
        match self {
            Self::NotFound(_) => ErrorKind::NotFound,
            Self::Rules(_) => ErrorKind::Unavailable,
        }
    }
}

pub(crate) struct RemoveKeywordRuleUseCase {
    rules: Arc<dyn KeywordRuleRepository>,
}

impl RemoveKeywordRuleUseCase {
    pub(crate) fn new(rules: Arc<dyn KeywordRuleRepository>) -> Self {
        Self { rules }
    }

    #[tracing::instrument(name = "use_case.remove_keyword_rule", skip(self))]
    pub(crate) async fn execute(
        &self,
        input: RemoveKeywordRuleInput,
    ) -> Result<(), RemoveKeywordRuleError> {
        self.rules
            .find_by_id(input.id)
            .await?
            .ok_or(RemoveKeywordRuleError::NotFound(input.id))?;
        self.rules.delete(input.id).await?;
        Ok(())
    }
}
