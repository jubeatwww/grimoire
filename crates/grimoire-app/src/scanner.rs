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

/// Names we never recurse into. Synology drops deleted files into `#recycle`
/// and indexes thumbnails into `@eaDir`; OS metadata files (`.DS_Store`,
/// `Thumbs.db`) and the Windows recycle bin add noise as well. Skipping these
/// at the directory level keeps scan output focused on the actual library
/// and stops "I deleted a file" from showing up as a new ghost entry.
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

fn is_skipped(entry: &walkdir::DirEntry) -> bool {
    let name = entry.file_name().to_string_lossy();
    SKIPPED_DIR_NAMES.iter().any(|skip| name.eq_ignore_ascii_case(skip))
}

fn scan_library_blocking(options: ScanOptions) -> anyhow::Result<ScanResult> {
    let mut items = Vec::new();
    let mut warnings = Vec::new();

    for entry in WalkDir::new(&options.root)
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

        let metadata = entry.metadata()?;
        let modified_at: DateTime<Utc> = metadata.modified()?.into();
        let file_name = path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or_default()
            .to_string();
        let legacy_location = legacy_location(&options.root, path);
        let relative_path = path
            .strip_prefix(&options.root)
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/");

        items.push(InventoryItem {
            id: Uuid::new_v4(),
            source_id: options.source_id.clone(),
            path: PathBuf::from(&relative_path),
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

/// One line of the "other files" report: a file the scanner ignores, or a
/// whole directory whose subtree holds no zip or rar.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OtherEntry {
    /// Relative to the library root, `/`-separated.
    pub path: String,
    pub is_dir: bool,
    pub files: u64,
    pub bytes: u64,
}

fn is_archive(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("zip") || e.eq_ignore_ascii_case("rar"))
}

/// Everything under `root` that is not a zip or rar, for the read-only report.
/// `exclude` holds absolute folders to leave out entirely (the asset cache).
pub fn report_other_files(root: &Path, exclude: &[PathBuf]) -> anyhow::Result<Vec<OtherEntry>> {
    use std::collections::{BTreeMap, HashSet};

    let mut archive_dirs: HashSet<PathBuf> = HashSet::new();
    let mut others: Vec<(PathBuf, u64)> = Vec::new();
    for entry in WalkDir::new(root)
        .min_depth(1)
        .into_iter()
        .filter_entry(|e| !is_skipped(e) && !exclude.iter().any(|x| e.path() == x))
    {
        let Ok(entry) = entry else { continue };
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        let name = entry.file_name().to_string_lossy();
        if crate::library_writer::is_metadata_file_name(&name) {
            continue;
        }
        if is_archive(path) {
            for dir in path.ancestors().skip(1) {
                if dir == root || !archive_dirs.insert(dir.to_path_buf()) {
                    break;
                }
            }
        } else {
            let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
            others.push((path.to_path_buf(), size));
        }
    }

    let rel = |p: &Path| {
        p.strip_prefix(root)
            .unwrap_or(p)
            .to_string_lossy()
            .replace('\\', "/")
    };
    let mut grouped: BTreeMap<String, OtherEntry> = BTreeMap::new();
    for (path, size) in others {
        // The top-most folder below the root without any archive in it.
        let ancestors: Vec<&Path> = path
            .ancestors()
            .skip(1)
            .take_while(|d| *d != root)
            .collect();
        let collapse = ancestors
            .iter()
            .rev()
            .find(|d| !archive_dirs.contains(**d))
            .copied();
        let (key, is_dir) = match collapse {
            Some(dir) => (rel(dir), true),
            None => (rel(&path), false),
        };
        let entry = grouped.entry(key.clone()).or_insert(OtherEntry {
            path: key,
            is_dir,
            files: 0,
            bytes: 0,
        });
        entry.files += 1;
        entry.bytes += size;
    }
    Ok(grouped.into_values().collect())
}

#[cfg(test)]
mod other_files_tests {
    use super::*;
    use std::fs;

    #[test]
    fn collapses_archive_free_folders_and_lists_loose_files() {
        let root = std::env::temp_dir().join(format!("grimoire-other-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let write = |rel: &str, bytes: usize| {
            let p = root.join(rel);
            fs::create_dir_all(p.parent().unwrap()).unwrap();
            fs::write(p, vec![0u8; bytes]).unwrap();
        };
        write("RPG/game.zip", 1);
        write("RPG/readme.txt", 3);
        write("RPG/Extracted Game/Game.exe", 10);
        write("RPG/Extracted Game/data/a.dat", 5);
        write("RPG/.DS_Store", 1);
        write("Tools/tool.7z", 7);
        write("cache/cover.jpg", 2);
        write("#recycle/old.iso", 9);

        let entries = report_other_files(&root, &[root.join("cache")]).unwrap();
        let _ = fs::remove_dir_all(&root);
        assert_eq!(
            entries,
            vec![
                OtherEntry { path: "RPG/Extracted Game".into(), is_dir: true, files: 2, bytes: 15 },
                OtherEntry { path: "RPG/readme.txt".into(), is_dir: false, files: 1, bytes: 3 },
                OtherEntry { path: "Tools".into(), is_dir: true, files: 1, bytes: 7 },
            ]
        );
    }
}
