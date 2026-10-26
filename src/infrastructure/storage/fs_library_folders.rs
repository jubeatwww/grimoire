//! Pruning removes folders that hold nothing but NAS / OS metadata, walking up from where a file
//! left, and never touches protected folders. A zip or rar is never metadata, whatever its name
//! or folder.

use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use async_trait::async_trait;
use walkdir::WalkDir;

use super::fs_library_scanner::is_skipped;
use crate::domain::library_folders::{
    LibraryFolders, LibraryFoldersError, OtherFileEntry, ProtectedFolders, PruneStarts,
    PrunedFolders,
};
use crate::domain::library_path::{
    LibraryPath, is_archive_name, is_metadata_dir_name, is_metadata_file_name,
};

pub(crate) struct FsLibraryFolders {
    root: PathBuf,
    /// Lives inside the library on the NAS; never pruned and left out of the report.
    asset_cache_root: PathBuf,
}

impl FsLibraryFolders {
    pub(crate) fn new(root: PathBuf, asset_cache_root: PathBuf) -> Self {
        Self {
            root,
            asset_cache_root,
        }
    }

    fn guard(&self, protected: &ProtectedFolders) -> PruneGuard {
        let mut folders: Vec<PathBuf> = protected
            .folders()
            .iter()
            .map(|f| self.root.join(f.as_str()))
            .collect();
        folders.push(self.asset_cache_root.clone());
        PruneGuard::new(self.root.clone(), folders)
    }

    fn relative(&self, abs: &Path) -> String {
        relative_to(&self.root, abs)
    }
}

fn relative_to(root: &Path, abs: &Path) -> String {
    abs.strip_prefix(root)
        .unwrap_or(abs)
        .to_string_lossy()
        .replace('\\', "/")
}

fn unavailable(e: impl std::error::Error + Send + Sync + 'static) -> LibraryFoldersError {
    LibraryFoldersError::Unavailable(Box::new(e))
}

#[async_trait]
impl LibraryFolders for FsLibraryFolders {
    async fn prune(&self, starts: PruneStarts, protected: &ProtectedFolders) -> PrunedFolders {
        let guard = self.guard(protected);
        let root = self.root.clone();
        let starts: Vec<PathBuf> = starts
            .folders()
            .iter()
            .map(|s| root.join(s.as_str()))
            .collect();
        let (removed, errors) = tokio::task::spawn_blocking(move || {
            let mut removed = Vec::new();
            let mut errors = Vec::new();
            for start in starts {
                let outcome = guard.prune_upwards(&start);
                removed.extend(outcome.removed);
                if let Some(e) = outcome.error {
                    errors.push(format!("could not prune {}: {e}", start.display()));
                }
            }
            (removed, errors)
        })
        .await
        .unwrap_or_else(|e| (Vec::new(), vec![format!("prune task failed: {e}")]));
        let removed = removed.iter().map(|d| self.relative(d)).collect();
        PrunedFolders::new(removed, errors)
    }

    async fn preview_prune(
        &self,
        leaving: Vec<LibraryPath>,
        arriving: Vec<LibraryPath>,
        protected: &ProtectedFolders,
    ) -> Result<Vec<String>, LibraryFoldersError> {
        let guard = self.guard(protected);
        let absolute = |paths: Vec<LibraryPath>| -> Vec<PathBuf> {
            paths.iter().map(|p| self.root.join(p.as_str())).collect()
        };
        let (out, into) = (absolute(leaving), absolute(arriving));
        let dirs = tokio::task::spawn_blocking(move || guard.preview(&out, &into))
            .await
            .map_err(unavailable)?;
        Ok(dirs.iter().map(|d| self.relative(d)).collect())
    }

    async fn folder_exists(&self, folder: &LibraryPath) -> bool {
        tokio::fs::metadata(self.root.join(folder.as_str()))
            .await
            .is_ok_and(|m| m.is_dir())
    }

    async fn create_folder(&self, folder: &LibraryPath) -> Result<(), LibraryFoldersError> {
        tokio::fs::create_dir_all(self.root.join(folder.as_str()))
            .await
            .map_err(unavailable)
    }

    async fn report_other_files(&self) -> Result<Vec<OtherFileEntry>, LibraryFoldersError> {
        let root = self.root.clone();
        let exclude = vec![self.asset_cache_root.clone()];
        tokio::task::spawn_blocking(move || report_other_files(&root, &exclude))
            .await
            .map_err(unavailable)?
            .map_err(unavailable)
    }
}

fn subtree_has_archive(dir: &Path) -> io::Result<bool> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            if subtree_has_archive(&entry.path())? {
                return Ok(true);
            }
        } else if is_archive_name(&entry.file_name().to_string_lossy()) {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Whether pruning may delete this entry: a metadata file, or a metadata folder with no archive
/// anywhere inside.
fn is_disposable(path: &Path, file_type: fs::FileType) -> io::Result<bool> {
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    if file_type.is_dir() {
        Ok(is_metadata_dir_name(&name) && !subtree_has_archive(path)?)
    } else {
        Ok(is_metadata_file_name(&name))
    }
}

#[derive(Debug, Default)]
struct PruneOutcome {
    removed: Vec<PathBuf>,
    error: Option<io::Error>,
}

/// Folders pruning must never remove, besides anything outside `root`.
#[derive(Debug, Clone)]
struct PruneGuard {
    root: PathBuf,
    protected: HashSet<PathBuf>,
}

impl PruneGuard {
    fn new(root: PathBuf, protected: impl IntoIterator<Item = PathBuf>) -> Self {
        let mut set: HashSet<PathBuf> = protected.into_iter().collect();
        set.insert(root.clone());
        Self {
            root,
            protected: set,
        }
    }

    fn may_remove(&self, dir: &Path) -> bool {
        dir.starts_with(&self.root) && dir != self.root && !self.protected.contains(dir)
    }

    /// Remove `start` and its parents while they hold only disposable metadata. Returns the
    /// removed folders, deepest first, and the error that stopped the walk early, if any.
    fn prune_upwards(&self, start: &Path) -> PruneOutcome {
        let mut outcome = PruneOutcome::default();
        let mut dir = start.to_path_buf();
        while self.may_remove(&dir) {
            match self.prune_one(&dir) {
                Ok(true) => outcome.removed.push(dir.clone()),
                Ok(false) => break,
                Err(e) => {
                    outcome.error = Some(e);
                    break;
                }
            }
            match dir.parent() {
                Some(parent) => dir = parent.to_path_buf(),
                None => break,
            }
        }
        outcome
    }

    /// Remove `dir` when it holds only disposable entries. `Ok(false)` when it has content or
    /// is already gone.
    fn prune_one(&self, dir: &Path) -> io::Result<bool> {
        let entries = match fs::read_dir(dir) {
            Ok(entries) => entries.collect::<Result<Vec<_>, _>>()?,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(false),
            Err(e) => return Err(e),
        };
        for entry in &entries {
            if !is_disposable(&entry.path(), entry.file_type()?)? {
                return Ok(false);
            }
        }
        for entry in entries {
            let path = entry.path();
            if entry.file_type()?.is_dir() {
                fs::remove_dir_all(&path)?;
            } else {
                fs::remove_file(&path)?;
            }
        }
        fs::remove_dir(dir)?;
        Ok(true)
    }

    /// Folders `prune_upwards` would remove after `moved_out` files leave and `moved_in` files
    /// arrive. Used for the preview; reads, never writes.
    fn preview(&self, moved_out: &[PathBuf], moved_in: &[PathBuf]) -> Vec<PathBuf> {
        let leaving: HashSet<&Path> = moved_out.iter().map(PathBuf::as_path).collect();
        // A folder that will receive a file is not going to be empty.
        let receiving: HashSet<&Path> = moved_in
            .iter()
            .flat_map(|p| p.ancestors().skip(1))
            .collect();

        let mut candidates: Vec<PathBuf> = Vec::new();
        for file in moved_out {
            for dir in file.ancestors().skip(1) {
                if !self.may_remove(dir) {
                    break;
                }
                if !candidates.iter().any(|c| c == dir) {
                    candidates.push(dir.to_path_buf());
                }
            }
        }
        // Deepest first so a parent can count an emptied child as removable.
        candidates.sort_by_key(|d| std::cmp::Reverse(d.components().count()));

        let mut removable: HashSet<PathBuf> = HashSet::new();
        for dir in &candidates {
            if receiving.contains(dir.as_path()) {
                continue;
            }
            let Ok(entries) = fs::read_dir(dir) else {
                continue;
            };
            let empty_after = entries.filter_map(Result::ok).all(|e| {
                let path = e.path();
                leaving.contains(path.as_path())
                    || removable.contains(&path)
                    || e.file_type()
                        .and_then(|ft| is_disposable(&path, ft))
                        .unwrap_or(false)
            });
            if empty_after {
                removable.insert(dir.clone());
            }
        }
        let mut out: Vec<PathBuf> = removable.into_iter().collect();
        out.sort();
        out
    }
}

/// Everything under `root` that is not a zip or rar, for the read-only report. `exclude` holds
/// absolute folders to leave out entirely (the asset cache).
fn report_other_files(
    root: &Path,
    exclude: &[PathBuf],
) -> Result<Vec<OtherFileEntry>, walkdir::Error> {
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
        if is_metadata_file_name(&name) {
            continue;
        }
        if is_archive_name(&name) {
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

    let mut grouped: BTreeMap<String, (bool, u64, u64)> = BTreeMap::new();
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
            Some(dir) => (relative_to(root, dir), true),
            None => (relative_to(root, &path), false),
        };
        let entry = grouped.entry(key).or_insert((is_dir, 0, 0));
        entry.1 += 1;
        entry.2 += size;
    }
    Ok(grouped
        .into_iter()
        .map(|(path, (is_dir, files, bytes))| OtherFileEntry::new(path, is_dir, files, bytes))
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::storage::temp_dir::TempDir;

    #[test]
    fn prunes_folders_holding_only_metadata() {
        let t = TempDir::new();
        t.file("LunaSoft/sub/@eaDir/game.zip@SynoEAStream", "x");
        t.file("LunaSoft/sub/.DS_Store", "x");
        t.file("LunaSoft/Thumbs.db", "x");
        let guard = PruneGuard::new(t.0.clone(), []);
        let removed = guard.prune_upwards(&t.0.join("LunaSoft/sub")).removed;
        assert_eq!(
            removed,
            vec![t.0.join("LunaSoft/sub"), t.0.join("LunaSoft")]
        );
        assert!(!t.0.join("LunaSoft").exists());
        assert!(t.0.exists());
    }

    #[test]
    fn stops_at_folders_with_content() {
        let t = TempDir::new();
        t.file("RPG/other.zip", "x");
        fs::create_dir_all(t.0.join("RPG/empty")).unwrap();
        let guard = PruneGuard::new(t.0.clone(), []);
        let removed = guard.prune_upwards(&t.0.join("RPG/empty")).removed;
        assert_eq!(removed, vec![t.0.join("RPG/empty")]);
        assert!(t.0.join("RPG/other.zip").exists());
    }

    #[test]
    fn never_prunes_protected_folders() {
        let t = TempDir::new();
        fs::create_dir_all(t.0.join("04 角色扮演(RPG)/Circle")).unwrap();
        let guard = PruneGuard::new(t.0.clone(), [t.0.join("04 角色扮演(RPG)")]);
        let removed = guard
            .prune_upwards(&t.0.join("04 角色扮演(RPG)/Circle"))
            .removed;
        assert_eq!(removed, vec![t.0.join("04 角色扮演(RPG)/Circle")]);
        assert!(t.0.join("04 角色扮演(RPG)").exists());
    }

    #[test]
    fn archives_are_never_metadata_whatever_their_name_or_folder() {
        let t = TempDir::new();
        t.file("Circle/._Foo/game.zip", "x");
        t.file("Circle2/@eaDir/stray.rar", "x");
        t.file("Circle3/._resource.zip", "x");
        t.file("Circle3/.DS_Store", "x");
        let guard = PruneGuard::new(t.0.clone(), []);
        for dir in ["Circle", "Circle2", "Circle3"] {
            let outcome = guard.prune_upwards(&t.0.join(dir));
            assert!(outcome.removed.is_empty(), "{dir} was pruned");
            assert!(outcome.error.is_none());
        }
        assert!(t.0.join("Circle/._Foo/game.zip").exists());
        assert!(t.0.join("Circle2/@eaDir/stray.rar").exists());
        assert!(t.0.join("Circle3/._resource.zip").exists());
    }

    #[test]
    fn preview_predicts_pruning_without_touching_disk() {
        let t = TempDir::new();
        let a = t.file("LunaSoft/a.zip", "x");
        let b = t.file("LunaSoft/sub/b.zip", "x");
        t.file("LunaSoft/sub/@eaDir/b.zip@SynoEAStream", "x");
        let keep = t.file("RPG/stay.zip", "x");
        let c = t.file("RPG/c.zip", "x");
        let guard = PruneGuard::new(t.0.clone(), []);
        let dirs = guard.preview(&[a, b, c], &[t.0.join("04/Luna/a.zip")]);
        assert_eq!(dirs, vec![t.0.join("LunaSoft"), t.0.join("LunaSoft/sub")]);
        assert!(keep.exists());
        assert!(t.0.join("LunaSoft/sub/b.zip").exists());
    }

    #[test]
    fn preview_keeps_folders_that_receive_files() {
        let t = TempDir::new();
        let a = t.file("04/Circle/a.zip", "x");
        let guard = PruneGuard::new(t.0.clone(), [t.0.join("04")]);
        let dirs = guard.preview(&[a], &[t.0.join("04/Circle/Work/a.zip")]);
        assert!(dirs.is_empty());
    }

    #[test]
    fn collapses_archive_free_folders_and_lists_loose_files() {
        let t = TempDir::new();
        let write = |rel: &str, bytes: usize| {
            let p = t.0.join(rel);
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

        let entries = report_other_files(&t.0, &[t.0.join("cache")]).unwrap();
        assert_eq!(
            entries,
            vec![
                OtherFileEntry::new("RPG/Extracted Game".into(), true, 2, 15),
                OtherFileEntry::new("RPG/readme.txt".into(), false, 1, 3),
                OtherFileEntry::new("Tools".into(), true, 1, 7),
            ]
        );
    }
}
