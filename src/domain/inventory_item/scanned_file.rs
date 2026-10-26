use crate::domain::library_path::LibraryPath;
use crate::kernel::Timestamp;

/// A zip or rar as the scanner saw it on disk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ScannedFile {
    path: LibraryPath,
    file_name: String,
    extension: Option<String>,
    file_size: u64,
    modified_at: Timestamp,
    legacy_location: Option<String>,
}

impl ScannedFile {
    /// `legacy_location`: the top-level folder the file sat in when it was scanned.
    pub(crate) fn new(
        path: LibraryPath,
        file_name: String,
        extension: Option<String>,
        file_size: u64,
        modified_at: Timestamp,
        legacy_location: Option<String>,
    ) -> Self {
        Self {
            path,
            file_name,
            extension,
            file_size,
            modified_at,
            legacy_location,
        }
    }

    pub(crate) fn path(&self) -> &LibraryPath {
        &self.path
    }

    pub(crate) fn file_name(&self) -> &str {
        &self.file_name
    }

    pub(crate) fn extension(&self) -> Option<&str> {
        self.extension.as_deref()
    }

    pub(crate) fn file_size(&self) -> u64 {
        self.file_size
    }

    pub(crate) fn modified_at(&self) -> Timestamp {
        self.modified_at
    }

    pub(crate) fn legacy_location(&self) -> Option<&str> {
        self.legacy_location.as_deref()
    }
}
