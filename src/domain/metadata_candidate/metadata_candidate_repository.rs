use async_trait::async_trait;

use super::{MetadataCandidate, MetadataCandidateId};

#[async_trait]
pub(crate) trait MetadataCandidateRepository: Send + Sync {
    async fn find_by_id(
        &self,
        id: MetadataCandidateId,
    ) -> Result<Option<MetadataCandidate>, MetadataCandidateRepositoryError>;

    /// Stores a new candidate; a candidate never changes, so saving one twice keeps the first.
    async fn save(
        &self,
        candidate: &MetadataCandidate,
    ) -> Result<(), MetadataCandidateRepositoryError>;
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum MetadataCandidateRepositoryError {
    #[error("metadata candidate storage failed: {0}")]
    Backend(#[source] Box<dyn std::error::Error + Send + Sync>),
}
