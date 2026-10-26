//! The library's files on the NAS, as the move and delete flows need them.

use std::path::PathBuf;

use async_trait::async_trait;

use crate::domain::library_path::LibraryPath;

#[async_trait]
pub(crate) trait LibraryFiles: Send + Sync {
    /// Whether something exists at each path, in order.
    async fn exist(&self, paths: &[LibraryPath]) -> Result<Vec<bool>, LibraryFilesError>;

    /// Moves a file, creating the target's folders, without ever replacing anything at `to`.
    async fn move_file(&self, from: &LibraryPath, to: &LibraryPath) -> Result<(), FileMoveError>;

    async fn remove_file(&self, path: &LibraryPath) -> Result<FileRemoval, LibraryFilesError>;

    /// What is at both ends of a move, to settle one that was interrupted.
    async fn probe_move(
        &self,
        from: &LibraryPath,
        to: &LibraryPath,
    ) -> Result<MoveProbe, LibraryFilesError>;

    /// Removes the extra hard link an interrupted move left behind. Best effort.
    async fn remove_link(&self, path: &LibraryPath);

    /// Where the file lives on this machine, for streaming a download.
    fn locate(&self, path: &LibraryPath) -> PathBuf;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FileRemoval {
    Removed,
    AlreadyGone,
}

/// Both ends of a move as found on disk.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct MoveProbe {
    source_exists: bool,
    target_exists: bool,
    same_file: bool,
}

impl MoveProbe {
    /// `same_file`: both paths are hard links of one file.
    pub(crate) fn new(source_exists: bool, target_exists: bool, same_file: bool) -> Self {
        Self {
            source_exists,
            target_exists,
            same_file,
        }
    }

    pub(crate) fn source_exists(self) -> bool {
        self.source_exists
    }

    pub(crate) fn target_exists(self) -> bool {
        self.target_exists
    }

    pub(crate) fn same_file(self) -> bool {
        self.same_file
    }
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum FileMoveError {
    #[error("source file is missing")]
    SourceMissing,
    #[error("target already exists")]
    TargetExists,
    #[error("{0}")]
    Unavailable(#[source] Box<dyn std::error::Error + Send + Sync>),
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum LibraryFilesError {
    #[error("the library files could not be read: {0}")]
    Unavailable(#[source] Box<dyn std::error::Error + Send + Sync>),
}
