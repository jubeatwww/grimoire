//! The only code that moves or deletes things under the library root.
//!
//! Moves never overwrite: POSIX `rename` silently replaces an existing target,
//! so a move is a hard link (which fails atomically when the target exists)
//! followed by removing the source. Where hard links are unsupported the
//! writer checks the target is free and renames.
//!
//! Pruning removes folders that hold nothing but NAS / OS metadata, walking
//! up from where a file left, and never touches protected folders. A zip or
//! rar is never metadata, whatever its name or folder.

use std::{
    collections::HashSet,
    fs, io,
    path::{Path, PathBuf},
};

/// Metadata folders: Synology thumbnails and macOS resource forks.
const METADATA_DIRS: &[&str] = &["@eaDir", ".AppleDouble"];
/// Metadata files from macOS and Windows. `._*` AppleDouble files too.
const METADATA_FILES: &[&str] = &[".DS_Store", "Thumbs.db", "desktop.ini"];

pub fn is_archive_name(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    lower.ends_with(".zip") || lower.ends_with(".rar")
}

/// A file name that is NAS / OS metadata. Archives never are.
pub fn is_metadata_file_name(name: &str) -> bool {
    !is_archive_name(name)
        && (METADATA_FILES.iter().any(|n| name.eq_ignore_ascii_case(n)) || name.starts_with("._"))
}

fn is_metadata_dir_name(name: &str) -> bool {
    METADATA_DIRS.iter().any(|n| name.eq_ignore_ascii_case(n))
}

/// Names a folder created by grimoire must not take, so pruning can never
/// mistake a real folder for metadata. Hidden names are included.
pub fn is_reserved_folder_name(name: &str) -> bool {
    name.starts_with('.')
        || is_metadata_dir_name(name)
        || METADATA_FILES.iter().any(|n| name.eq_ignore_ascii_case(n))
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

/// Whether pruning may delete this entry: a metadata file, or a metadata
/// folder with no archive anywhere inside.
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

/// True when both paths are the same file (hard links of one inode).
pub fn same_file(a: &Path, b: &Path) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        match (fs::symlink_metadata(a), fs::symlink_metadata(b)) {
            (Ok(ma), Ok(mb)) => ma.dev() == mb.dev() && ma.ino() == mb.ino(),
            _ => false,
        }
    }
    #[cfg(not(unix))]
    {
        let _ = (a, b);
        false
    }
}

#[derive(Debug)]
pub enum MoveError {
    SourceMissing,
    TargetExists,
    Io(io::Error),
}

impl std::fmt::Display for MoveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SourceMissing => write!(f, "source file is missing"),
            Self::TargetExists => write!(f, "target already exists"),
            Self::Io(e) => write!(f, "{e}"),
        }
    }
}

fn exists(path: &Path) -> bool {
    fs::symlink_metadata(path).is_ok()
}

/// Move `src` to `dst`, creating `dst`'s parent folders. Fails with
/// `TargetExists` instead of replacing anything.
pub fn move_no_overwrite(src: &Path, dst: &Path) -> Result<(), MoveError> {
    if !exists(src) {
        return Err(MoveError::SourceMissing);
    }
    if exists(dst) {
        return Err(MoveError::TargetExists);
    }
    if let Some(parent) = dst.parent() {
        fs::create_dir_all(parent).map_err(MoveError::Io)?;
    }
    match fs::hard_link(src, dst) {
        Ok(()) => match fs::remove_file(src) {
            Ok(()) => Ok(()),
            // Over NFS the unlink can succeed on the server while the client
            // still reports an error. Only roll back while the source is
            // really still there, so the new link is never the last copy.
            Err(e) if exists(src) => {
                if same_file(src, dst) {
                    let _ = fs::remove_file(dst);
                }
                Err(MoveError::Io(e))
            }
            Err(_) => Ok(()),
        },
        Err(e) if e.kind() == io::ErrorKind::AlreadyExists => Err(MoveError::TargetExists),
        Err(e) if links_unsupported(&e) => rename_exclusive(src, dst),
        Err(e) => Err(MoveError::Io(e)),
    }
}

/// Errors meaning "this filesystem or share does not do hard links".
fn links_unsupported(e: &io::Error) -> bool {
    matches!(
        e.kind(),
        io::ErrorKind::PermissionDenied | io::ErrorKind::Unsupported | io::ErrorKind::TooManyLinks
    )
}

/// Rename without hard links, still refusing to overwrite: claim the target
/// with an exclusive create first (decided by the NFS server, not by the
/// client's cache), then rename over that empty placeholder.
pub(crate) fn rename_exclusive(src: &Path, dst: &Path) -> Result<(), MoveError> {
    match fs::OpenOptions::new().write(true).create_new(true).open(dst) {
        Ok(_) => {}
        Err(e) if e.kind() == io::ErrorKind::AlreadyExists => return Err(MoveError::TargetExists),
        Err(e) => return Err(MoveError::Io(e)),
    }
    if let Err(e) = fs::rename(src, dst) {
        // Remove the placeholder, but only while it is still our empty file.
        if fs::symlink_metadata(dst).is_ok_and(|m| m.is_file() && m.len() == 0) {
            let _ = fs::remove_file(dst);
        }
        return Err(MoveError::Io(e));
    }
    Ok(())
}

#[derive(Debug, Default)]
pub struct PruneOutcome {
    pub removed: Vec<PathBuf>,
    pub error: Option<io::Error>,
}

/// Folders pruning must never remove, besides anything outside `root`.
#[derive(Debug, Clone)]
pub struct PruneGuard {
    root: PathBuf,
    protected: HashSet<PathBuf>,
}

impl PruneGuard {
    pub fn new(root: impl Into<PathBuf>, protected: impl IntoIterator<Item = PathBuf>) -> Self {
        let root = root.into();
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

    /// Remove `start` and its parents while they hold only disposable
    /// metadata. Returns the removed folders, deepest first, and the error
    /// that stopped the walk early, if any.
    pub fn prune_upwards(&self, start: &Path) -> PruneOutcome {
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

    /// Remove `dir` when it holds only disposable entries. `Ok(false)` when it
    /// has content or is already gone.
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

    /// Folders `prune_upwards` would remove after `moved_out` files leave and
    /// `moved_in` files arrive. Used for the preview; reads, never writes.
    pub fn preview(&self, moved_out: &[PathBuf], moved_in: &[PathBuf]) -> Vec<PathBuf> {
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
                    || e
                        .file_type()
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct TempDir(PathBuf);

    impl TempDir {
        fn new() -> Self {
            // A counter, not the clock: macOS clocks tick in microseconds, so
            // tests running in parallel could share a directory.
            static NEXT: AtomicUsize = AtomicUsize::new(0);
            let dir = std::env::temp_dir().join(format!(
                "grimoire-writer-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            let _ = fs::remove_dir_all(&dir);
            fs::create_dir_all(&dir).unwrap();
            Self(dir)
        }

        fn file(&self, rel: &str, content: &str) -> PathBuf {
            let path = self.0.join(rel);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(&path, content).unwrap();
            path
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn moves_a_file_and_creates_parents() {
        let t = TempDir::new();
        let src = t.file("RPG/game.zip", "game");
        let dst = t.0.join("04 角色扮演(RPG)/Circle/game.zip");
        move_no_overwrite(&src, &dst).unwrap();
        assert!(!src.exists());
        assert_eq!(fs::read_to_string(&dst).unwrap(), "game");
    }

    #[test]
    fn never_overwrites_an_existing_target() {
        let t = TempDir::new();
        let src = t.file("RPG/game.zip", "new");
        let dst = t.file("04/Circle/game.zip", "old");
        assert!(matches!(
            move_no_overwrite(&src, &dst),
            Err(MoveError::TargetExists)
        ));
        assert_eq!(fs::read_to_string(&src).unwrap(), "new");
        assert_eq!(fs::read_to_string(&dst).unwrap(), "old");
    }

    #[test]
    fn reports_a_missing_source() {
        let t = TempDir::new();
        let dst = t.0.join("04/game.zip");
        assert!(matches!(
            move_no_overwrite(&t.0.join("nope.zip"), &dst),
            Err(MoveError::SourceMissing)
        ));
    }

    #[test]
    fn prunes_folders_holding_only_metadata() {
        let t = TempDir::new();
        t.file("LunaSoft/sub/@eaDir/game.zip@SynoEAStream", "x");
        t.file("LunaSoft/sub/.DS_Store", "x");
        t.file("LunaSoft/Thumbs.db", "x");
        let guard = PruneGuard::new(&t.0, []);
        let removed = guard.prune_upwards(&t.0.join("LunaSoft/sub")).removed;
        assert_eq!(removed, vec![t.0.join("LunaSoft/sub"), t.0.join("LunaSoft")]);
        assert!(!t.0.join("LunaSoft").exists());
        assert!(t.0.exists());
    }

    #[test]
    fn stops_at_folders_with_content() {
        let t = TempDir::new();
        t.file("RPG/other.zip", "x");
        fs::create_dir_all(t.0.join("RPG/empty")).unwrap();
        let guard = PruneGuard::new(&t.0, []);
        let removed = guard.prune_upwards(&t.0.join("RPG/empty")).removed;
        assert_eq!(removed, vec![t.0.join("RPG/empty")]);
        assert!(t.0.join("RPG/other.zip").exists());
    }

    #[test]
    fn never_prunes_protected_folders() {
        let t = TempDir::new();
        fs::create_dir_all(t.0.join("04 角色扮演(RPG)/Circle")).unwrap();
        let guard = PruneGuard::new(&t.0, [t.0.join("04 角色扮演(RPG)")]);
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
        let guard = PruneGuard::new(&t.0, []);
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
    fn exclusive_rename_refuses_an_existing_target() {
        let t = TempDir::new();
        let src = t.file("a/game.zip", "new");
        let dst = t.file("b/game.zip", "old");
        assert!(matches!(rename_exclusive(&src, &dst), Err(MoveError::TargetExists)));
        assert_eq!(fs::read_to_string(&dst).unwrap(), "old");
        let free = t.0.join("b/other.zip");
        rename_exclusive(&src, &free).unwrap();
        assert_eq!(fs::read_to_string(&free).unwrap(), "new");
        assert!(!src.exists());
    }

    #[test]
    fn reserved_folder_names_cover_metadata_and_hidden_names() {
        assert!(is_reserved_folder_name("._Foo"));
        assert!(is_reserved_folder_name("@eadir"));
        assert!(is_reserved_folder_name("Thumbs.db"));
        assert!(is_reserved_folder_name(".hack"));
        assert!(!is_reserved_folder_name("LunaSoft"));
    }

    #[test]
    fn preview_predicts_pruning_without_touching_disk() {
        let t = TempDir::new();
        let a = t.file("LunaSoft/a.zip", "x");
        let b = t.file("LunaSoft/sub/b.zip", "x");
        t.file("LunaSoft/sub/@eaDir/b.zip@SynoEAStream", "x");
        let keep = t.file("RPG/stay.zip", "x");
        let c = t.file("RPG/c.zip", "x");
        let guard = PruneGuard::new(&t.0, []);
        let dirs = guard.preview(&[a, b, c], &[t.0.join("04/Luna/a.zip")]);
        assert_eq!(dirs, vec![t.0.join("LunaSoft"), t.0.join("LunaSoft/sub")]);
        assert!(keep.exists());
        assert!(t.0.join("LunaSoft/sub/b.zip").exists());
    }

    #[test]
    fn preview_keeps_folders_that_receive_files() {
        let t = TempDir::new();
        let a = t.file("04/Circle/a.zip", "x");
        let guard = PruneGuard::new(&t.0, [t.0.join("04")]);
        let dirs = guard.preview(&[a], &[t.0.join("04/Circle/Work/a.zip")]);
        assert!(dirs.is_empty());
    }
}
