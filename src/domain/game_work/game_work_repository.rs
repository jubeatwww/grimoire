use async_trait::async_trait;

use super::{GameWork, GameWorkId};
use crate::domain::metadata_source::MetadataProvider;

#[async_trait]
pub(crate) trait GameWorkRepository: Send + Sync {
    async fn find_by_id(&self, id: GameWorkId)
    -> Result<Option<GameWork>, GameWorkRepositoryError>;

    /// The work linked to this id on the site, if any.
    async fn find_by_external_id(
        &self,
        provider: MetadataProvider,
        external_id: &str,
    ) -> Result<Option<GameWork>, GameWorkRepositoryError>;

    /// Works linked to at least one site, oldest first.
    async fn list_linked(&self, limit: i64) -> Result<Vec<GameWork>, GameWorkRepositoryError>;

    /// Upsert.
    async fn save(&self, work: &GameWork) -> Result<(), GameWorkRepositoryError>;
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum GameWorkRepositoryError {
    #[error("game work storage failed: {0}")]
    Backend(#[source] Box<dyn std::error::Error + Send + Sync>),
}

#[cfg(test)]
pub(crate) mod fakes {
    use std::collections::HashMap;
    use std::sync::Mutex;

    use async_trait::async_trait;

    use super::{GameWorkRepository, GameWorkRepositoryError};
    use crate::domain::game_work::{GameWork, GameWorkId};
    use crate::domain::metadata_source::MetadataProvider;

    #[derive(Default)]
    pub(crate) struct FakeGameWorkRepository(Mutex<HashMap<GameWorkId, GameWork>>);

    impl FakeGameWorkRepository {
        pub(crate) fn all(&self) -> Vec<GameWork> {
            self.0.lock().unwrap().values().cloned().collect()
        }
    }

    #[async_trait]
    impl GameWorkRepository for FakeGameWorkRepository {
        async fn find_by_id(
            &self,
            id: GameWorkId,
        ) -> Result<Option<GameWork>, GameWorkRepositoryError> {
            Ok(self.0.lock().unwrap().get(&id).cloned())
        }

        async fn find_by_external_id(
            &self,
            provider: MetadataProvider,
            external_id: &str,
        ) -> Result<Option<GameWork>, GameWorkRepositoryError> {
            Ok(self
                .0
                .lock()
                .unwrap()
                .values()
                .find(|w| w.external_ids().get(provider) == Some(external_id))
                .cloned())
        }

        async fn list_linked(&self, limit: i64) -> Result<Vec<GameWork>, GameWorkRepositoryError> {
            let linked = self
                .0
                .lock()
                .unwrap()
                .values()
                .filter(|w| {
                    MetadataProvider::ALL
                        .iter()
                        .any(|p| w.external_ids().get(*p).is_some())
                })
                .take(usize::try_from(limit).unwrap_or(0))
                .cloned()
                .collect();
            Ok(linked)
        }

        async fn save(&self, work: &GameWork) -> Result<(), GameWorkRepositoryError> {
            self.0.lock().unwrap().insert(work.id(), work.clone());
            Ok(())
        }
    }
}
