//! Paths below the library root, and which entry names count as NAS / OS metadata.

mod entry_name;
mod error;

use std::path::{Component, Path};

use serde::Serialize;

pub(crate) use entry_name::{
    is_archive_name, is_metadata_dir_name, is_metadata_file_name, is_reserved_folder_name,
};
pub(crate) use error::LibraryPathError;

/// A file or folder below the library root: relative and never escaping the root. Paths the
/// scanner and the move planner build are `/`-separated.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(transparent)]
pub(crate) struct LibraryPath(String);

impl LibraryPath {
    pub(crate) fn parse(raw: &str) -> Result<Self, LibraryPathError> {
        if raw.is_empty() {
            return Err(LibraryPathError::Empty);
        }
        let path = Path::new(raw);
        if path.is_absolute() {
            return Err(LibraryPathError::Absolute);
        }
        let escapes = path.components().any(|c| {
            matches!(
                c,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        });
        if escapes {
            return Err(LibraryPathError::Traversal);
        }
        Ok(Self(raw.to_owned()))
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }

    /// The folder holding this entry; `None` when that is the library root itself.
    pub(crate) fn parent(&self) -> Option<LibraryPath> {
        self.0
            .rsplit_once('/')
            .map(|(dir, _)| dir)
            .filter(|dir| !dir.is_empty())
            .map(|dir| Self(dir.to_owned()))
    }

    /// The folder part of the path as text, `None` for an entry at the root.
    pub(crate) fn folder(&self) -> Option<&str> {
        self.0.rsplit_once('/').map(|(dir, _)| dir)
    }
}

impl std::fmt::Display for LibraryPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_a_child_of_the_root() {
        assert_eq!(
            LibraryPath::parse("ADV/game.zip").unwrap().as_str(),
            "ADV/game.zip"
        );
    }

    #[test]
    fn rejects_traversal_absolute_and_empty_paths() {
        assert_eq!(
            LibraryPath::parse("../secret.zip"),
            Err(LibraryPathError::Traversal)
        );
        assert_eq!(
            LibraryPath::parse("/etc/passwd"),
            Err(LibraryPathError::Absolute)
        );
        assert_eq!(LibraryPath::parse(""), Err(LibraryPathError::Empty));
    }

    #[test]
    fn parent_stops_at_the_root() {
        let path = LibraryPath::parse("RPG/Circle/game.zip").unwrap();
        assert_eq!(path.parent().unwrap().as_str(), "RPG/Circle");
        assert_eq!(LibraryPath::parse("game.zip").unwrap().parent(), None);
    }
}
