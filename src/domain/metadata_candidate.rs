//! A search hit kept so the user can confirm it afterwards.

mod events;
mod metadata_candidate_repository;

use crate::domain::metadata_source::{MetadataProvider, SourceMatch};
use crate::kernel::Id;

pub(crate) use events::MetadataCandidateFound;
pub(crate) use metadata_candidate_repository::{
    MetadataCandidateRepository, MetadataCandidateRepositoryError,
};

pub(crate) type MetadataCandidateId = Id<MetadataCandidate>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MetadataCandidate {
    id: MetadataCandidateId,
    provider: MetadataProvider,
    source_url: String,
    query_used: String,
    found: SourceMatch,
}

impl MetadataCandidate {
    pub(crate) fn found(
        id: MetadataCandidateId,
        provider: MetadataProvider,
        query: &str,
        found: SourceMatch,
    ) -> (Self, MetadataCandidateFound) {
        let event = MetadataCandidateFound {
            candidate_id: id,
            provider,
            source_work_id: found.source_work_id().to_owned(),
        };
        let candidate = Self {
            id,
            provider,
            source_url: provider.work_url(found.source_work_id()),
            query_used: query.to_owned(),
            found,
        };
        (candidate, event)
    }

    /// Rebuilds a stored candidate. Repositories only.
    pub(crate) fn from_persisted(
        id: MetadataCandidateId,
        provider: MetadataProvider,
        source_url: String,
        query_used: String,
        found: SourceMatch,
    ) -> Self {
        Self {
            id,
            provider,
            source_url,
            query_used,
            found,
        }
    }

    pub(crate) fn id(&self) -> MetadataCandidateId {
        self.id
    }

    pub(crate) fn provider(&self) -> MetadataProvider {
        self.provider
    }

    pub(crate) fn source_url(&self) -> &str {
        &self.source_url
    }

    pub(crate) fn query_used(&self) -> &str {
        &self.query_used
    }

    pub(crate) fn source_match(&self) -> &SourceMatch {
        &self.found
    }
}
