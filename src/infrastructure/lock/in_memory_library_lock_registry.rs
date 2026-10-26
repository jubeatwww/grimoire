use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::Mutex;

use crate::domain::library_lock_registry::{LibraryLease, LibraryLockRegistry};

/// One process serves the library, so an in-process mutex is the whole lock.
#[derive(Default)]
pub(crate) struct InMemoryLibraryLockRegistry(Arc<Mutex<()>>);

#[async_trait]
impl LibraryLockRegistry for InMemoryLibraryLockRegistry {
    async fn acquire(&self) -> LibraryLease {
        LibraryLease::new(Arc::clone(&self.0).lock_owned().await)
    }
}
