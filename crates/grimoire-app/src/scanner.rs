use chrono::{DateTime, Utc};
use grimoire_domain::inventory::{InventoryItem, InventoryKind, OrganizationStatus, PlayStatus};
use std::path::{Path, PathBuf};
use tokio::task;
use uuid::Uuid;
use walkdir::WalkDir;

#[derive(Debug, Clone)]
pub struct ScanOptions {
    pub source_id: String,
    pub root: PathBuf,
}

#[derive(Debug, Clone)]
pub struct ScanResult {
    pub items: Vec<InventoryItem>,
    pub warnings: Vec<String>,
}

pub async fn scan_library(options: ScanOptions) -> anyhow::Result<ScanResult> {
    task::spawn_blocking(move || scan_library_blocking(options)).await?
}

fn scan_library_blocking(options: ScanOptions) -> anyhow::Result<ScanResult> {
    let mut items = Vec::new();
    let mut warnings = Vec::new();

    for entry in WalkDir::new(&options.root).min_depth(1) {
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

        let metadata = entry.metadata()?;
        let modified_at: DateTime<Utc> = metadata.modified()?.into();
        let file_name = path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or_default()
            .to_string();
        let legacy_location = legacy_location(&options.root, path);

        items.push(InventoryItem {
            id: Uuid::new_v4(),
            source_id: options.source_id.clone(),
            path: path.to_path_buf(),
            file_name,
            extension,
            kind: InventoryKind::File,
            file_size: metadata.len(),
            modified_at,
            content_hash: None,
            legacy_location,
            primary_category: None,
            genre_facets: Vec::new(),
            game_work_id: None,
            version: None,
            language: None,
            patch_location: None,
            save_location: None,
            extracted: false,
            downloaded: false,
            organization_status: OrganizationStatus::Pending,
            play_status: PlayStatus::NotPlayed,
            rating: None,
            personal_tags: Vec::new(),
            notes: None,
        });
    }

    Ok(ScanResult { items, warnings })
}

fn legacy_location(root: &Path, path: &Path) -> Option<String> {
    path.strip_prefix(root)
        .ok()
        .and_then(|relative| relative.components().next())
        .and_then(|component| component.as_os_str().to_str())
        .map(ToOwned::to_owned)
}
