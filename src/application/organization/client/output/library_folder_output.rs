use serde::Serialize;

use crate::domain::library_folders::OtherFileEntry;

#[derive(Debug, Serialize)]
pub(crate) struct InboxOutput {
    pub(crate) folder: &'static str,
    pub(crate) exists: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ListOtherFilesOutput {
    entries: Vec<OtherFileListItemOutput>,
    total_files: u64,
    total_bytes: u64,
}

impl From<&[OtherFileEntry]> for ListOtherFilesOutput {
    fn from(entries: &[OtherFileEntry]) -> Self {
        Self {
            total_files: entries.iter().map(OtherFileEntry::files).sum(),
            total_bytes: entries.iter().map(OtherFileEntry::bytes).sum(),
            entries: entries.iter().map(OtherFileListItemOutput::from).collect(),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct OtherFileListItemOutput {
    path: String,
    is_dir: bool,
    files: u64,
    bytes: u64,
}

impl From<&OtherFileEntry> for OtherFileListItemOutput {
    fn from(entry: &OtherFileEntry) -> Self {
        Self {
            path: entry.path().to_owned(),
            is_dir: entry.is_dir(),
            files: entry.files(),
            bytes: entry.bytes(),
        }
    }
}
