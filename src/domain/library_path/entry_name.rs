/// Metadata folders: Synology thumbnails and macOS resource forks.
const METADATA_DIRS: &[&str] = &["@eaDir", ".AppleDouble"];
/// Metadata files from macOS and Windows. `._*` AppleDouble files too.
const METADATA_FILES: &[&str] = &[".DS_Store", "Thumbs.db", "desktop.ini"];

pub(crate) fn is_archive_name(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    lower.ends_with(".zip") || lower.ends_with(".rar")
}

/// A file name that is NAS / OS metadata. Archives never are.
pub(crate) fn is_metadata_file_name(name: &str) -> bool {
    !is_archive_name(name)
        && (METADATA_FILES.iter().any(|n| name.eq_ignore_ascii_case(n)) || name.starts_with("._"))
}

pub(crate) fn is_metadata_dir_name(name: &str) -> bool {
    METADATA_DIRS.iter().any(|n| name.eq_ignore_ascii_case(n))
}

/// Names a folder created by grimoire must not take, so pruning can never mistake a real folder
/// for metadata. Hidden names are included.
pub(crate) fn is_reserved_folder_name(name: &str) -> bool {
    name.starts_with('.')
        || is_metadata_dir_name(name)
        || METADATA_FILES.iter().any(|n| name.eq_ignore_ascii_case(n))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn archives_are_never_metadata_whatever_their_name() {
        assert!(!is_metadata_file_name("._game.zip"));
        assert!(!is_metadata_file_name("Thumbs.db.rar"));
        assert!(is_metadata_file_name("._cover.jpg"));
        assert!(is_metadata_file_name("thumbs.db"));
    }

    #[test]
    fn hidden_and_metadata_names_are_reserved() {
        assert!(is_reserved_folder_name(".hack"));
        assert!(is_reserved_folder_name("@eaDir"));
        assert!(is_reserved_folder_name("Desktop.ini"));
        assert!(!is_reserved_folder_name("LunaSoft"));
    }
}
