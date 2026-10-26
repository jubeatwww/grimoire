use std::sync::Arc;

use super::{GameWork, GameWorkId, GameWorkRepository, GameWorkRepositoryError};
use crate::domain::metadata_source::MetadataProvider;

/// Finds the work a site id belongs to, or registers a new one: a site id belongs to one work.
pub(crate) struct GameWorkFactory {
    works: Arc<dyn GameWorkRepository>,
}

/// The outcome of a lookup by site id.
pub(crate) enum WorkLookup {
    Existing(GameWork),
    Registered(GameWork),
}

impl WorkLookup {
    pub(crate) fn into_work(self) -> GameWork {
        match self {
            Self::Existing(work) | Self::Registered(work) => work,
        }
    }
}

impl GameWorkFactory {
    pub(crate) fn new(works: Arc<dyn GameWorkRepository>) -> Self {
        Self { works }
    }

    /// `new_id` is used only when no work has this site id yet. A registered work is not stored
    /// here; the caller saves it with the rest of its changes.
    pub(crate) async fn find_or_register(
        &self,
        new_id: GameWorkId,
        provider: MetadataProvider,
        external_id: &str,
        title: &str,
        circle: Option<String>,
    ) -> Result<WorkLookup, GameWorkRepositoryError> {
        if let Some(existing) = self
            .works
            .find_by_external_id(provider, external_id)
            .await?
        {
            return Ok(WorkLookup::Existing(existing));
        }
        let (work, _registered) = GameWork::register(new_id, provider, external_id, title, circle);
        Ok(WorkLookup::Registered(work))
    }
}
