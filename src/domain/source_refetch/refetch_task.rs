use crate::domain::game_work::{GameWork, GameWorkId};
use crate::domain::metadata_source::MetadataProvider;

/// One site id of one work to fetch again.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RefetchTask {
    work_id: GameWorkId,
    provider: MetadataProvider,
    external_id: String,
}

impl RefetchTask {
    /// Every site id of every work, works in the order given, sites DLsite, VNDB, Steam.
    pub(crate) fn for_works(works: &[GameWork]) -> Vec<Self> {
        works
            .iter()
            .flat_map(|work| {
                MetadataProvider::ALL
                    .into_iter()
                    .filter_map(move |provider| {
                        work.external_ids().get(provider).map(|id| Self {
                            work_id: work.id(),
                            provider,
                            external_id: id.to_owned(),
                        })
                    })
            })
            .collect()
    }

    pub(crate) fn work_id(&self) -> GameWorkId {
        self.work_id
    }

    pub(crate) fn provider(&self) -> MetadataProvider {
        self.provider
    }

    pub(crate) fn external_id(&self) -> &str {
        &self.external_id
    }
}
