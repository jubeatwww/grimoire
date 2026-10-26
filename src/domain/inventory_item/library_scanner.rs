use async_trait::async_trait;

use super::ScannedFile;

/// Walks the library root for zip and rar files, skipping NAS / OS metadata folders.
#[async_trait]
pub(crate) trait LibraryScanner: Send + Sync {
    async fn scan(&self) -> Result<LibraryScan, LibraryScannerError>;
}

/// What one scan found, and the entries it could not read.
#[derive(Debug, Clone, Default)]
pub(crate) struct LibraryScan {
    files: Vec<ScannedFile>,
    warnings: Vec<String>,
}

impl LibraryScan {
    pub(crate) fn new(files: Vec<ScannedFile>, warnings: Vec<String>) -> Self {
        Self { files, warnings }
    }

    pub(crate) fn into_parts(self) -> (Vec<ScannedFile>, Vec<String>) {
        (self.files, self.warnings)
    }
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum LibraryScannerError {
    #[error("the library could not be scanned: {0}")]
    Unavailable(#[source] Box<dyn std::error::Error + Send + Sync>),
}
