use std::sync::Arc;

use super::InboxOutput;
use crate::domain::file_placement::{INBOX_FOLDER, inbox};
use crate::domain::library_folders::{LibraryFolders, LibraryFoldersError};
use crate::domain::library_lock_registry::LibraryLockRegistry;
use crate::kernel::{ErrorCode, ErrorKind};

#[derive(Debug, thiserror::Error)]
pub(crate) enum CreateInboxError {
    #[error(transparent)]
    Folders(#[from] LibraryFoldersError),
}

impl ErrorCode for CreateInboxError {
    fn code(&self) -> &'static str {
        "grimoire.unavailable"
    }

    fn kind(&self) -> ErrorKind {
        ErrorKind::Unavailable
    }
}

/// Creates the inbox folder new downloads go to.
pub(crate) struct CreateInboxUseCase {
    lock: Arc<dyn LibraryLockRegistry>,
    folders: Arc<dyn LibraryFolders>,
}

impl CreateInboxUseCase {
    pub(crate) fn new(
        lock: Arc<dyn LibraryLockRegistry>,
        folders: Arc<dyn LibraryFolders>,
    ) -> Self {
        Self { lock, folders }
    }

    #[tracing::instrument(name = "use_case.create_inbox", skip(self))]
    pub(crate) async fn execute(&self) -> Result<InboxOutput, CreateInboxError> {
        let _lease = self.lock.acquire().await;
        self.folders.create_folder(&inbox()).await?;
        Ok(InboxOutput {
            folder: INBOX_FOLDER,
            exists: true,
        })
    }
}
