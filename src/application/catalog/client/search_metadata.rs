use std::collections::HashMap;
use std::sync::Arc;

use serde::Deserialize;

use super::{MetadataCandidateOutput, SearchMetadataOutput};
use crate::domain::metadata_candidate::{
    MetadataCandidate, MetadataCandidateRepository, MetadataCandidateRepositoryError,
};
use crate::domain::metadata_source::{MetadataProvider, MetadataSource, SourceMatch};
use crate::kernel::{ErrorCode, ErrorKind, IdGenerator};

#[derive(Debug, Deserialize)]
pub(crate) struct SearchMetadataInput {
    pub(crate) query: String,
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum SearchMetadataError {
    #[error(transparent)]
    Candidates(#[from] MetadataCandidateRepositoryError),
}

impl ErrorCode for SearchMetadataError {
    fn code(&self) -> &'static str {
        "grimoire.unavailable"
    }

    fn kind(&self) -> ErrorKind {
        ErrorKind::Unavailable
    }
}

/// Searches every site at once and keeps the hits so one can be confirmed. A site that fails
/// simply contributes nothing.
pub(crate) struct SearchMetadataUseCase {
    sources: HashMap<MetadataProvider, Arc<dyn MetadataSource>>,
    candidates: Arc<dyn MetadataCandidateRepository>,
    ids: Arc<dyn IdGenerator>,
}

impl SearchMetadataUseCase {
    pub(crate) fn new(
        sources: HashMap<MetadataProvider, Arc<dyn MetadataSource>>,
        candidates: Arc<dyn MetadataCandidateRepository>,
        ids: Arc<dyn IdGenerator>,
    ) -> Self {
        Self {
            sources,
            candidates,
            ids,
        }
    }

    async fn search_one(&self, provider: MetadataProvider, query: &str) -> Vec<SourceMatch> {
        let Some(source) = self.sources.get(&provider) else {
            return Vec::new();
        };
        match source.search(query).await {
            Ok(found) => found,
            Err(e) => {
                tracing::warn!(source = provider.name(), error = %e, "metadata search failed");
                Vec::new()
            }
        }
    }

    #[tracing::instrument(name = "use_case.search_metadata", skip(self))]
    pub(crate) async fn execute(
        &self,
        input: SearchMetadataInput,
    ) -> Result<SearchMetadataOutput, SearchMetadataError> {
        let query = input.query.as_str();
        let (dlsite, vndb, steam) = tokio::join!(
            self.search_one(MetadataProvider::Dlsite, query),
            self.search_one(MetadataProvider::Vndb, query),
            self.search_one(MetadataProvider::Steam, query),
        );
        let found = [
            (MetadataProvider::Dlsite, dlsite),
            (MetadataProvider::Vndb, vndb),
            (MetadataProvider::Steam, steam),
        ];
        let mut candidates = Vec::new();
        for (provider, matches) in found {
            for hit in matches {
                let (candidate, _found) =
                    MetadataCandidate::found(self.ids.next(), provider, query, hit);
                self.candidates.save(&candidate).await?;
                candidates.push(MetadataCandidateOutput::from(&candidate));
            }
        }
        Ok(SearchMetadataOutput { candidates })
    }
}
