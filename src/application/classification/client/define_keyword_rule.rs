use std::sync::Arc;

use serde::Deserialize;

use super::KeywordRuleOutput;
use crate::domain::keyword_rule::{
    Keyword, KeywordRule, KeywordRuleError, KeywordRuleId, KeywordRuleRepository,
    KeywordRuleRepositoryError,
};
use crate::domain::metadata_source::MetadataProvider;
use crate::domain::vocabulary::{VocabularySource, VocabularySourceError};
use crate::kernel::{ErrorCode, ErrorKind, IdGenerator};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DefineKeywordRuleInput {
    /// Present to edit an existing rule.
    pub(crate) id: Option<KeywordRuleId>,
    /// `None` applies the keyword to every source.
    pub(crate) source: Option<MetadataProvider>,
    pub(crate) keyword: Keyword,
    pub(crate) code: String,
    #[serde(default)]
    pub(crate) contests_type: bool,
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum DefineKeywordRuleError {
    #[error("no such rule")]
    NotFound(KeywordRuleId),
    #[error(transparent)]
    Rule(#[from] KeywordRuleError),
    #[error(transparent)]
    Vocabulary(#[from] VocabularySourceError),
    #[error(transparent)]
    Rules(#[from] KeywordRuleRepositoryError),
}

impl ErrorCode for DefineKeywordRuleError {
    fn code(&self) -> &'static str {
        match self {
            Self::NotFound(_) => "keyword_rule.not_found",
            Self::Rule(e) => e.code(),
            Self::Rules(KeywordRuleRepositoryError::KeywordTaken(_)) => {
                "keyword_rule.keyword_taken"
            }
            Self::Vocabulary(_) | Self::Rules(_) => "grimoire.unavailable",
        }
    }

    fn kind(&self) -> ErrorKind {
        match self {
            Self::NotFound(_) => ErrorKind::NotFound,
            Self::Rule(e) => e.kind(),
            Self::Rules(KeywordRuleRepositoryError::KeywordTaken(_)) => ErrorKind::Conflict,
            Self::Vocabulary(_) | Self::Rules(_) => ErrorKind::Unavailable,
        }
    }
}

/// Defines a keyword rule, or redefines one when an id is given.
pub(crate) struct DefineKeywordRuleUseCase {
    vocabulary: Arc<dyn VocabularySource>,
    rules: Arc<dyn KeywordRuleRepository>,
    ids: Arc<dyn IdGenerator>,
}

impl DefineKeywordRuleUseCase {
    pub(crate) fn new(
        vocabulary: Arc<dyn VocabularySource>,
        rules: Arc<dyn KeywordRuleRepository>,
        ids: Arc<dyn IdGenerator>,
    ) -> Self {
        Self {
            vocabulary,
            rules,
            ids,
        }
    }

    #[tracing::instrument(name = "use_case.define_keyword_rule", skip(self))]
    pub(crate) async fn execute(
        &self,
        input: DefineKeywordRuleInput,
    ) -> Result<KeywordRuleOutput, DefineKeywordRuleError> {
        let vocabulary = self.vocabulary.current().await?;
        let rule = match input.id {
            Some(id) => {
                let mut rule = self
                    .rules
                    .find_by_id(id)
                    .await?
                    .ok_or(DefineKeywordRuleError::NotFound(id))?;
                let _defined = rule.redefine(
                    &vocabulary,
                    input.source,
                    &input.keyword,
                    &input.code,
                    input.contests_type,
                )?;
                rule
            }
            None => {
                let (rule, _defined) = KeywordRule::define(
                    &vocabulary,
                    self.ids.next(),
                    input.source,
                    &input.keyword,
                    &input.code,
                    input.contests_type,
                )?;
                rule
            }
        };
        self.rules.save(&rule).await?;
        Ok(KeywordRuleOutput::from(&rule))
    }
}
