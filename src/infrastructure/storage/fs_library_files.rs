//! Moves never overwrite: POSIX `rename` silently replaces an existing target, so a move is a
//! hard link (which fails atomically when the target exists) followed by removing the source.
//! Where hard links are unsupported the target is claimed with an exclusive create and renamed
//! over.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use async_trait::async_trait;

use crate::domain::library_files::{
    FileMoveError, FileRemoval, LibraryFiles, LibraryFilesError, MoveProbe,
};
use crate::domain::library_path::LibraryPath;

pub(crate) struct FsLibraryFiles {
    root: PathBuf,
}

impl FsLibraryFiles {
    pub(crate) fn new(root: PathBuf) -> Self {
        Self { root }
    }

    fn absolute(&self, path: &LibraryPath) -> PathBuf {
        self.root.join(path.as_str())
    }
}

fn unavailable(e: impl std::error::Error + Send + Sync + 'static) -> LibraryFilesError {
    LibraryFilesError::Unavailable(Box::new(e))
}

#[async_trait]
impl LibraryFiles for FsLibraryFiles {
    async fn exist(&self, paths: &[LibraryPath]) -> Result<Vec<bool>, LibraryFilesError> {
        let absolute: Vec<PathBuf> = paths.iter().map(|p| self.absolute(p)).collect();
        // Filesystem checks can be slow over NFS; keep them off the async workers.
        tokio::task::spawn_blocking(move || absolute.iter().map(|p| exists(p)).collect())
            .await
            .map_err(unavailable)
    }

    async fn move_file(&self, from: &LibraryPath, to: &LibraryPath) -> Result<(), FileMoveError> {
        let (from, to) = (self.absolute(from), self.absolute(to));
        tokio::task::spawn_blocking(move || move_no_overwrite(&from, &to))
            .await
            .unwrap_or_else(|e| Err(FileMoveError::Unavailable(Box::new(e))))
    }

    async fn remove_file(&self, path: &LibraryPath) -> Result<FileRemoval, LibraryFilesError> {
        match tokio::fs::remove_file(self.absolute(path)).await {
            Ok(()) => Ok(FileRemoval::Removed),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(FileRemoval::AlreadyGone),
            Err(e) => Err(unavailable(e)),
        }
    }

    async fn probe_move(
        &self,
        from: &LibraryPath,
        to: &LibraryPath,
    ) -> Result<MoveProbe, LibraryFilesError> {
        let (from, to) = (self.absolute(from), self.absolute(to));
        tokio::task::spawn_blocking(move || {
            MoveProbe::new(exists(&from), exists(&to), same_file(&from, &to))
        })
        .await
        .map_err(unavailable)
    }

    async fn remove_link(&self, path: &LibraryPath) {
        let _ = tokio::fs::remove_file(self.absolute(path)).await;
    }

    fn locate(&self, path: &LibraryPath) -> PathBuf {
        self.absolute(path)
    }
}

fn exists(path: &Path) -> bool {
    fs::symlink_metadata(path).is_ok()
}

/// True when both paths are the same file (hard links of one inode).
fn same_file(a: &Path, b: &Path) -> bool {
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

/// Move `src` to `dst`, creating `dst`'s parent folders. Fails with `TargetExists` instead of
/// replacing anything.
fn move_no_overwrite(src: &Path, dst: &Path) -> Result<(), FileMoveError> {
    let io = |e: io::Error| FileMoveError::Unavailable(Box::new(e));
    if !exists(src) {
        return Err(FileMoveError::SourceMissing);
    }
    if exists(dst) {
        return Err(FileMoveError::TargetExists);
    }
    if let Some(parent) = dst.parent() {
        fs::create_dir_all(parent).map_err(io)?;
    }
    match fs::hard_link(src, dst) {
        Ok(()) => match fs::remove_file(src) {
            Ok(()) => Ok(()),
            // Over NFS the unlink can succeed on the server while the client still reports an
            // error. Only roll back while the source is really still there, so the new link is
            // never the last copy.
            Err(e) if exists(src) => {
                if same_file(src, dst) {
                    let _ = fs::remove_file(dst);
                }
                Err(io(e))
            }
            Err(_) => Ok(()),
        },
        Err(e) if e.kind() == io::ErrorKind::AlreadyExists => Err(FileMoveError::TargetExists),
        Err(e) if links_unsupported(&e) => rename_exclusive(src, dst),
        Err(e) => Err(io(e)),
    }
}

/// Errors meaning "this filesystem or share does not do hard links".
fn links_unsupported(e: &io::Error) -> bool {
    matches!(
        e.kind(),
        io::ErrorKind::PermissionDenied | io::ErrorKind::Unsupported | io::ErrorKind::TooManyLinks
    )
}

/// Rename without hard links, still refusing to overwrite: claim the target with an exclusive
/// create first (decided by the NFS server, not by the client's cache), then rename over that
/// empty placeholder.
fn rename_exclusive(src: &Path, dst: &Path) -> Result<(), FileMoveError> {
    match fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(dst)
    {
        Ok(_) => {}
        Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {
            return Err(FileMoveError::TargetExists);
        }
        Err(e) => return Err(FileMoveError::Unavailable(Box::new(e))),
    }
    if let Err(e) = fs::rename(src, dst) {
        // Remove the placeholder, but only while it is still our empty file.
        if fs::symlink_metadata(dst).is_ok_and(|m| m.is_file() && m.len() == 0) {
            let _ = fs::remove_file(dst);
        }
        return Err(FileMoveError::Unavailable(Box::new(e)));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::storage::temp_dir::TempDir;

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
            Err(FileMoveError::TargetExists)
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
            Err(FileMoveError::SourceMissing)
        ));
    }

    #[test]
    fn exclusive_rename_refuses_an_existing_target() {
        let t = TempDir::new();
        let src = t.file("a/game.zip", "new");
        let dst = t.file("b/game.zip", "old");
        assert!(matches!(
            rename_exclusive(&src, &dst),
            Err(FileMoveError::TargetExists)
        ));
        assert_eq!(fs::read_to_string(&dst).unwrap(), "old");
        let free = t.0.join("b/other.zip");
        rename_exclusive(&src, &free).unwrap();
        assert_eq!(fs::read_to_string(&free).unwrap(), "new");
        assert!(!src.exists());
    }

    #[tokio::test]
    async fn probes_both_ends_of_a_half_done_move() {
        let t = TempDir::new();
        let src = t.file("a/game.zip", "x");
        fs::create_dir_all(t.0.join("b")).unwrap();
        fs::hard_link(&src, t.0.join("b/game.zip")).unwrap();
        let files = FsLibraryFiles::new(t.0.clone());
        let probe = files
            .probe_move(
                &LibraryPath::parse("a/game.zip").unwrap(),
                &LibraryPath::parse("b/game.zip").unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(probe, MoveProbe::new(true, true, true));
    }
}
