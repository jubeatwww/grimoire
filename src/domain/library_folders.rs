//! Folders of the library: pruning emptied ones, the inbox, and what the scanner leaves out.

use async_trait::async_trait;

use crate::domain::library_path::LibraryPath;

#[async_trait]
pub(crate) trait LibraryFolders: Send + Sync {
    /// Removes each start folder and its parents while they hold only NAS / OS metadata, never
    /// touching a protected folder.
    async fn prune(&self, starts: PruneStarts, protected: &ProtectedFolders) -> PrunedFolders;

    /// Folders `prune` would remove once the `leaving` files are gone and the `arriving` files
    /// are in, relative to the root. Reads, never writes.
    async fn preview_prune(
        &self,
        leaving: Vec<LibraryPath>,
        arriving: Vec<LibraryPath>,
        protected: &ProtectedFolders,
    ) -> Result<Vec<String>, LibraryFoldersError>;

    async fn folder_exists(&self, folder: &LibraryPath) -> bool;

    async fn create_folder(&self, folder: &LibraryPath) -> Result<(), LibraryFoldersError>;

    /// Everything below the root that is not a zip or rar, collapsed to archive-free folders.
    async fn report_other_files(&self) -> Result<Vec<OtherFileEntry>, LibraryFoldersError>;
}

/// Folders files left (or were meant to arrive in), where pruning starts: each once, in path
/// order (component by component).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct PruneStarts(Vec<LibraryPath>);

impl PruneStarts {
    pub(crate) fn new(mut folders: Vec<LibraryPath>) -> Self {
        folders.sort_by(|a, b| a.as_str().split('/').cmp(b.as_str().split('/')));
        folders.dedup();
        Self(folders)
    }

    pub(crate) fn folders(&self) -> &[LibraryPath] {
        &self.0
    }
}

/// Folders pruning must leave alone, besides the root.
#[derive(Debug, Clone, Default)]
pub(crate) struct ProtectedFolders(Vec<LibraryPath>);

impl ProtectedFolders {
    pub(crate) fn new(folders: Vec<LibraryPath>) -> Self {
        Self(folders)
    }

    pub(crate) fn folders(&self) -> &[LibraryPath] {
        &self.0
    }
}

/// What a prune removed (relative to the root, deepest first per start) and what stopped it
/// early.
#[derive(Debug, Clone, Default)]
pub(crate) struct PrunedFolders {
    removed: Vec<String>,
    errors: Vec<String>,
}

impl PrunedFolders {
    pub(crate) fn new(removed: Vec<String>, errors: Vec<String>) -> Self {
        Self { removed, errors }
    }

    pub(crate) fn into_parts(self) -> (Vec<String>, Vec<String>) {
        (self.removed, self.errors)
    }
}

/// One line of the "other files" report: a file the scanner ignores, or a whole folder whose
/// subtree holds no zip or rar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OtherFileEntry {
    path: String,
    is_dir: bool,
    files: u64,
    bytes: u64,
}

impl OtherFileEntry {
    pub(crate) fn new(path: String, is_dir: bool, files: u64, bytes: u64) -> Self {
        Self {
            path,
            is_dir,
            files,
            bytes,
        }
    }

    pub(crate) fn path(&self) -> &str {
        &self.path
    }

    pub(crate) fn is_dir(&self) -> bool {
        self.is_dir
    }

    pub(crate) fn files(&self) -> u64 {
        self.files
    }

    pub(crate) fn bytes(&self) -> u64 {
        self.bytes
    }
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum LibraryFoldersError {
    #[error("the library folders could not be read: {0}")]
    Unavailable(#[source] Box<dyn std::error::Error + Send + Sync>),
}
