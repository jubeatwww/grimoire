use async_trait::async_trait;

use super::WorkTypeRule;
use crate::domain::metadata_source::MetadataProvider;

#[async_trait]
pub(crate) trait WorkTypeRuleRepository: Send + Sync {
    /// `work_type` exactly as stored.
    async fn find_by_source_and_work_type(
        &self,
        source: MetadataProvider,
        work_type: &str,
    ) -> Result<Option<WorkTypeRule>, WorkTypeRuleRepositoryError>;

    /// By source, then work type.
    async fn list_all(&self, limit: i64) -> Result<Vec<WorkTypeRule>, WorkTypeRuleRepositoryError>;

    /// Upsert by source and work type.
    async fn save(&self, rule: &WorkTypeRule) -> Result<(), WorkTypeRuleRepositoryError>;

    /// Idempotent.
    async fn delete(&self, rule: &WorkTypeRule) -> Result<(), WorkTypeRuleRepositoryError>;
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum WorkTypeRuleRepositoryError {
    #[error("work type rule storage failed: {0}")]
    Backend(#[source] Box<dyn std::error::Error + Send + Sync>),
}
