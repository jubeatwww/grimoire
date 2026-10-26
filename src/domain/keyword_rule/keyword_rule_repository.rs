use async_trait::async_trait;

use super::{KeywordRule, KeywordRuleId};

#[async_trait]
pub(crate) trait KeywordRuleRepository: Send + Sync {
    async fn find_by_id(
        &self,
        id: KeywordRuleId,
    ) -> Result<Option<KeywordRule>, KeywordRuleRepositoryError>;

    /// Oldest first.
    async fn list_all(&self, limit: i64) -> Result<Vec<KeywordRule>, KeywordRuleRepositoryError>;

    /// Upsert. `KeywordTaken` when another rule already has this keyword for the same source.
    async fn save(&self, rule: &KeywordRule) -> Result<(), KeywordRuleRepositoryError>;

    /// Idempotent.
    async fn delete(&self, id: KeywordRuleId) -> Result<(), KeywordRuleRepositoryError>;
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum KeywordRuleRepositoryError {
    #[error("a rule for keyword {0} already exists for this source")]
    KeywordTaken(String),
    #[error("keyword rule storage failed: {0}")]
    Backend(#[source] Box<dyn std::error::Error + Send + Sync>),
}
