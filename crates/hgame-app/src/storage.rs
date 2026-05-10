use std::path::{Component, Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum StorageError {
    #[error("relative path cannot be empty")]
    EmptyPath,
    #[error("path traversal is not allowed")]
    Traversal,
    #[error("absolute client paths are not allowed")]
    AbsolutePath,
}

#[derive(Debug, Clone)]
pub struct StorageRoot {
    root: PathBuf,
}

impl StorageRoot {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn resolve_relative(&self, relative: impl AsRef<Path>) -> Result<PathBuf, StorageError> {
        let relative = relative.as_ref();
        if relative.as_os_str().is_empty() {
            return Err(StorageError::EmptyPath);
        }
        if relative.is_absolute() {
            return Err(StorageError::AbsolutePath);
        }
        for component in relative.components() {
            match component {
                Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                    return Err(StorageError::Traversal);
                }
                Component::CurDir | Component::Normal(_) => {}
            }
        }
        Ok(self.root.join(relative))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_child_inside_root() {
        let root = StorageRoot::new("/mnt/games");
        let resolved = root.resolve_relative("ADV/game.zip").unwrap();
        assert_eq!(resolved, PathBuf::from("/mnt/games/ADV/game.zip"));
    }

    #[test]
    fn rejects_parent_traversal() {
        let root = StorageRoot::new("/mnt/games");
        assert!(root.resolve_relative("../secret.zip").is_err());
    }

    #[test]
    fn rejects_absolute_path() {
        let root = StorageRoot::new("/mnt/games");
        assert!(root.resolve_relative("/etc/passwd").is_err());
    }
}
