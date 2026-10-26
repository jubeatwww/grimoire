use std::sync::Arc;

use async_trait::async_trait;

use super::{
    CategorySuggestionOutput, CommonTagListItemOutput, KeywordRuleOutput,
    ListCategorySuggestionsOutput, ListClassificationRulesOutput, ListCommonTagsOutput,
    WorkTypeRuleOutput,
};
use crate::domain::category_suggestion::{CategorySuggestion, ClassificationRules, WorkFacts};
use crate::domain::game_work::GameWorkId;
use crate::domain::keyword_rule::{KeywordRuleRepository, KeywordRuleRepositoryError};
use crate::domain::metadata_source::MetadataProvider;
use crate::domain::vocabulary::{VocabularySource, VocabularySourceError};
use crate::domain::work_type_rule::{WorkTypeRuleRepository, WorkTypeRuleRepositoryError};
use crate::kernel::{ErrorCode, ErrorKind};

/// Rules are few; this only guards against a runaway table.
const RULE_LIMIT: i64 = 10_000;
/// Works awaiting a category; this only guards against a runaway table.
const UNCLASSIFIED_LIMIT: i64 = 100_000;
/// Tags read before filtering out those a keyword already covers.
const COMMON_TAG_CANDIDATES: i64 = 1000;
const COMMON_TAG_LIMIT: usize = 150;

/// Read model of the classification views.
#[async_trait]
pub(crate) trait ClassificationClientQueryRepository: Send + Sync {
    /// Works that still need a category (unconfirmed, with at least one present, non-excluded
    /// file), by title, with what their suggestion is derived from.
    async fn list_unclassified_facts(
        &self,
        limit: i64,
    ) -> Result<Vec<(GameWorkId, WorkFacts)>, ClassificationClientQueryRepositoryError>;

    /// Source tags on unconfirmed works, most frequent first.
    async fn list_common_tags(
        &self,
        limit: i64,
    ) -> Result<Vec<CommonTagListItemOutput>, ClassificationClientQueryRepositoryError>;
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum ClassificationClientQueryRepositoryError {
    #[error("classification data could not be read: {0}")]
    Backend(#[source] Box<dyn std::error::Error + Send + Sync>),
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum ClassificationClientQueryError {
    #[error(transparent)]
    Facts(#[from] ClassificationClientQueryRepositoryError),
    #[error(transparent)]
    Vocabulary(#[from] VocabularySourceError),
    #[error(transparent)]
    WorkTypeRules(#[from] WorkTypeRuleRepositoryError),
    #[error(transparent)]
    KeywordRules(#[from] KeywordRuleRepositoryError),
}

impl ErrorCode for ClassificationClientQueryError {
    fn code(&self) -> &'static str {
        "grimoire.unavailable"
    }

    fn kind(&self) -> ErrorKind {
        ErrorKind::Unavailable
    }
}

pub(crate) struct ClassificationClientQueryUseCase {
    facts: Arc<dyn ClassificationClientQueryRepository>,
    vocabulary: Arc<dyn VocabularySource>,
    work_type_rules: Arc<dyn WorkTypeRuleRepository>,
    keyword_rules: Arc<dyn KeywordRuleRepository>,
}

impl ClassificationClientQueryUseCase {
    pub(crate) fn new(
        facts: Arc<dyn ClassificationClientQueryRepository>,
        vocabulary: Arc<dyn VocabularySource>,
        work_type_rules: Arc<dyn WorkTypeRuleRepository>,
        keyword_rules: Arc<dyn KeywordRuleRepository>,
    ) -> Self {
        Self {
            facts,
            vocabulary,
            work_type_rules,
            keyword_rules,
        }
    }

    async fn rules(&self) -> Result<ClassificationRules, ClassificationClientQueryError> {
        Ok(ClassificationRules::new(
            self.work_type_rules.list_all(RULE_LIMIT).await?,
            self.keyword_rules.list_all(RULE_LIMIT).await?,
        ))
    }

    /// A suggestion for every work that still needs a category.
    #[tracing::instrument(name = "use_case.list_suggestions", skip(self))]
    pub(crate) async fn list_suggestions(
        &self,
    ) -> Result<ListCategorySuggestionsOutput, ClassificationClientQueryError> {
        let vocabulary = self.vocabulary.current().await?;
        let rules = self.rules().await?;
        let works = self
            .facts
            .list_unclassified_facts(UNCLASSIFIED_LIMIT)
            .await?;
        let suggestions = works
            .iter()
            .map(|(id, facts)| {
                let suggestion = CategorySuggestion::derive(&vocabulary, &rules, facts);
                CategorySuggestionOutput::from((*id, &suggestion))
            })
            .collect();
        Ok(ListCategorySuggestionsOutput { suggestions })
    }

    #[tracing::instrument(name = "use_case.list_rules", skip(self))]
    pub(crate) async fn list_rules(
        &self,
    ) -> Result<ListClassificationRulesOutput, ClassificationClientQueryError> {
        let rules = self.rules().await?;
        Ok(ListClassificationRulesOutput {
            work_type_rules: rules
                .work_types()
                .iter()
                .map(WorkTypeRuleOutput::from)
                .collect(),
            keyword_rules: rules
                .keywords()
                .iter()
                .map(KeywordRuleOutput::from)
                .collect(),
        })
    }

    /// The most frequent source tags on unclassified works that no keyword rule matches yet:
    /// the raw material for new keyword rules.
    #[tracing::instrument(name = "use_case.list_common_tags", skip(self))]
    pub(crate) async fn list_common_tags(
        &self,
    ) -> Result<ListCommonTagsOutput, ClassificationClientQueryError> {
        let keywords = self.keyword_rules.list_all(RULE_LIMIT).await?;
        let tags = self
            .facts
            .list_common_tags(COMMON_TAG_CANDIDATES)
            .await?
            .into_iter()
            .filter(|tag| {
                let Some(source) = MetadataProvider::parse(&tag.source) else {
                    return true;
                };
                !keywords
                    .iter()
                    .any(|rule| rule.applies_to(source) && rule.matches_tag(&tag.label))
            })
            .take(COMMON_TAG_LIMIT)
            .collect();
        Ok(ListCommonTagsOutput { tags })
    }
}
