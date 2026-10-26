use async_trait::async_trait;

/// One lock over everything that writes to the library (moves, undo, the inbox) and over scans,
/// so a scan never sees a half-moved library.
#[async_trait]
pub(crate) trait LibraryLockRegistry: Send + Sync {
    async fn acquire(&self) -> LibraryLease;
}

/// Held while working on the library; dropping it releases the lock.
pub(crate) struct LibraryLease {
    _guard: Box<dyn std::any::Any + Send>,
}

impl LibraryLease {
    pub(crate) fn new(guard: impl std::any::Any + Send) -> Self {
        Self {
            _guard: Box::new(guard),
        }
    }
}
