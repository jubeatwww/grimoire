use std::path::PathBuf;

use serde::Serialize;

#[derive(Debug, Serialize)]
pub(crate) struct ScanLibraryOutput {
    pub(crate) scanned: usize,
    pub(crate) marked_missing: u64,
    pub(crate) warnings: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DeleteMissingInventoryItemsOutput {
    pub(crate) deleted: u64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DeleteInventoryItemFileOutput {
    pub(crate) file_deleted: bool,
    pub(crate) file_missing: bool,
}

/// The file to stream and the name to offer it under.
#[derive(Debug)]
pub(crate) struct DownloadInventoryItemOutput {
    pub(crate) path: PathBuf,
    pub(crate) file_name: String,
}
