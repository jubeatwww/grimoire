use std::collections::HashMap;

use async_trait::async_trait;

use super::FilePlacement;
use crate::domain::library_path::LibraryPath;

/// What the move planner reads from the library's records.
#[async_trait]
pub(crate) trait PlacementSource: Send + Sync {
    /// Present, non-excluded local files of works with a confirmed category, in folder order.
    async fn confirmed_placements(&self) -> Result<Vec<FilePlacement>, PlacementSourceError>;

    /// Records already at any of these paths, with whether each is marked missing.
    async fn occupants(
        &self,
        paths: &[LibraryPath],
    ) -> Result<HashMap<LibraryPath, bool>, PlacementSourceError>;
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum PlacementSourceError {
    #[error("placements could not be read: {0}")]
    Backend(#[source] Box<dyn std::error::Error + Send + Sync>),
}
