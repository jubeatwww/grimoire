use std::path::{Path, PathBuf};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use walkdir::WalkDir;

use crate::domain::inventory_item::{
    LibraryScan, LibraryScanner, LibraryScannerError, ScannedFile,
};
use crate::domain::library_path::LibraryPath;
use crate::kernel::Timestamp;

pub(crate) struct FsLibraryScanner {
    root: PathBuf,
}

impl FsLibraryScanner {
    pub(crate) fn new(root: PathBuf) -> Self {
        Self { root }
    }
}

/// Names never recursed into. Synology drops deleted files into `#recycle` and indexes
/// thumbnails into `@eaDir`; OS metadata files (`.DS_Store`, `Thumbs.db`) and the Windows
/// recycle bin add noise as well. Skipping these at the directory level keeps scan output
/// focused on the actual library and stops "I deleted a file" from showing up as a new ghost
/// entry.
const SKIPPED_DIR_NAMES: &[&str] = &[
    "#recycle",
    "@eaDir",
    ".DS_Store",
    "Thumbs.db",
    "$RECYCLE.BIN",
    ".Trash",
    ".Trashes",
    ".AppleDouble",
];

pub(super) fn is_skipped(entry: &walkdir::DirEntry) -> bool {
    let name = entry.file_name().to_string_lossy();
    SKIPPED_DIR_NAMES
        .iter()
        .any(|skip| name.eq_ignore_ascii_case(skip))
}

#[async_trait]
impl LibraryScanner for FsLibraryScanner {
    async fn scan(&self) -> Result<LibraryScan, LibraryScannerError> {
        let root = self.root.clone();
        tokio::task::spawn_blocking(move || scan_blocking(&root))
            .await
            .map_err(|e| LibraryScannerError::Unavailable(Box::new(e)))?
            .map_err(|e| LibraryScannerError::Unavailable(Box::new(e)))
    }
}

fn scan_blocking(root: &Path) -> Result<LibraryScan, std::io::Error> {
    let mut files = Vec::new();
    let mut warnings = Vec::new();

    for entry in WalkDir::new(root)
        .min_depth(1)
        .into_iter()
        .filter_entry(|e| !is_skipped(e))
    {
        let entry = match entry {
            Ok(entry) => entry,
            Err(err) => {
                warnings.push(err.to_string());
                continue;
            }
        };
        if !entry.file_type().is_file() {
            continue;
        }

        let path = entry.path();
        let extension = path
            .extension()
            .and_then(|value| value.to_str())
            .map(|value| value.to_ascii_lowercase());
        if !matches!(extension.as_deref(), Some("zip" | "rar")) {
            continue;
        }

        let metadata = entry.metadata().map_err(std::io::Error::from)?;
        let modified_at: DateTime<Utc> = metadata.modified()?.into();
        let file_name = path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or_default()
            .to_string();
        let relative = path
            .strip_prefix(root)
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/");
        let library_path = match LibraryPath::parse(&relative) {
            Ok(p) => p,
            Err(e) => {
                warnings.push(format!("{relative}: {e}"));
                continue;
            }
        };
        files.push(ScannedFile::new(
            library_path,
            file_name,
            extension,
            metadata.len(),
            Timestamp::from_utc(modified_at),
            legacy_location(root, path),
        ));
    }

    Ok(LibraryScan::new(files, warnings))
}

/// The first component below the root: the top-level folder, or the file itself at the root.
fn legacy_location(root: &Path, path: &Path) -> Option<String> {
    path.strip_prefix(root)
        .ok()
        .and_then(|relative| relative.components().next())
        .and_then(|component| component.as_os_str().to_str())
        .map(ToOwned::to_owned)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn scans_zip_and_rar_files_with_legacy_location() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures/library");
        let (files, _warnings) = FsLibraryScanner::new(root)
            .scan()
            .await
            .unwrap()
            .into_parts();
        assert_eq!(files.len(), 4);
        assert!(files.iter().any(|file| {
            file.file_name() == "mixed_sim_strategy.rar"
                && file.legacy_location() == Some("SIM+SLG")
                && file.extension() == Some("rar")
        }));
    }
}
