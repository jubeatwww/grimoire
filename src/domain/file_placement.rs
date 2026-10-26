//! Where a classified file belongs on the NAS.
//!
//! ```text
//! <category folder>/<circle>/<original file name>
//! <category folder>/<circle>/<work title>/<original file name>   (2+ files)
//! ```
//!
//! File names never change. A file already inside its work folder stays there even when the
//! work drops back to one file (no auto-flatten).

mod folder_segment;
mod placement_source;
mod work_placement;

use crate::domain::inventory_item::InventoryItemId;
use crate::domain::library_folders::ProtectedFolders;
use crate::domain::library_path::LibraryPath;
use crate::domain::vocabulary::Vocabulary;

pub(crate) use folder_segment::sanitize_segment;
pub(crate) use placement_source::{PlacementSource, PlacementSourceError};
pub(crate) use work_placement::WorkPlacement;

/// Inbox for new downloads at the library root. Never pruned.
pub(crate) const INBOX_FOLDER: &str = "00 待整理";
/// Synology's recycle bin. Never pruned.
pub(crate) const RECYCLE_FOLDER: &str = "#recycle";
/// Circle folder for works without a circle or developer.
pub(crate) const UNKNOWN_CIRCLE: &str = "未知社團";
/// Folder name byte cap. btrfs/ext4 allow 255; leave room for safety.
pub(crate) const MAX_SEGMENT_BYTES: usize = 200;

/// A present file of a work with a confirmed category.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FilePlacement {
    item_id: InventoryItemId,
    path: LibraryPath,
    file_name: String,
    work: WorkPlacement,
}

impl FilePlacement {
    pub(crate) fn new(
        item_id: InventoryItemId,
        path: LibraryPath,
        file_name: String,
        work: WorkPlacement,
    ) -> Self {
        Self {
            item_id,
            path,
            file_name,
            work,
        }
    }

    pub(crate) fn item_id(&self) -> InventoryItemId {
        self.item_id
    }

    pub(crate) fn path(&self) -> &LibraryPath {
        &self.path
    }

    pub(crate) fn work(&self) -> &WorkPlacement {
        &self.work
    }

    fn circle_segment(&self) -> String {
        let safe = self.work.circle().map(sanitize_segment).unwrap_or_default();
        if safe.is_empty() {
            UNKNOWN_CIRCLE.to_string()
        } else {
            safe
        }
    }

    fn title_segment(&self) -> String {
        let safe = sanitize_segment(self.work.title());
        if safe.is_empty() {
            "Untitled".to_string()
        } else {
            safe
        }
    }

    /// `<category>/<circle>/<file>`
    pub(crate) fn flat_target(&self) -> String {
        format!(
            "{}/{}/{}",
            self.work.category_folder(),
            self.circle_segment(),
            self.file_name
        )
    }

    /// `<category>/<circle>/<title>/<file>`
    pub(crate) fn work_folder_target(&self) -> String {
        format!(
            "{}/{}/{}/{}",
            self.work.category_folder(),
            self.circle_segment(),
            self.title_segment(),
            self.file_name
        )
    }

    /// The path the file should move to, or `None` when it already sits in an accepted place.
    pub(crate) fn target(&self) -> Option<String> {
        let current = self.path.as_str();
        let in_work = self.work_folder_target();
        if self.work.file_count() >= 2 {
            return (current != in_work).then_some(in_work);
        }
        let flat = self.flat_target();
        if current == flat || current == in_work {
            return None;
        }
        Some(flat)
    }
}

/// The inbox as a library path.
pub(crate) fn inbox() -> LibraryPath {
    LibraryPath::parse(INBOX_FOLDER).expect("the inbox folder name is a plain folder name")
}

/// Folders pruning must leave alone: every category folder, the inbox and the recycle bin.
pub(crate) fn protected_folders(vocabulary: &Vocabulary) -> ProtectedFolders {
    let folders = vocabulary
        .categories()
        .iter()
        .map(|c| c.folder_name())
        .chain([INBOX_FOLDER, RECYCLE_FOLDER])
        .filter_map(|name| LibraryPath::parse(name).ok())
        .collect();
    ProtectedFolders::new(folders)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::game_work::GameWorkId;

    fn placement(current_path: &str, work_file_count: usize) -> FilePlacement {
        FilePlacement::new(
            InventoryItemId::from_raw(uuid::Uuid::from_u128(1)),
            LibraryPath::parse(current_path).unwrap(),
            "game_v1.0.zip".into(),
            WorkPlacement::new(
                GameWorkId::from_raw(uuid::Uuid::from_u128(2)),
                "Game: Title?".into(),
                Some("LunaSoft".into()),
                "RPG".into(),
                "04 角色扮演(RPG)".into(),
                work_file_count,
            ),
        )
    }

    fn with_circle(p: FilePlacement, circle: Option<&str>) -> FilePlacement {
        let w = p.work().clone();
        FilePlacement::new(
            p.item_id(),
            p.path().clone(),
            "game_v1.0.zip".into(),
            WorkPlacement::new(
                w.work_id(),
                w.title().into(),
                circle.map(str::to_owned),
                w.category_id().into(),
                w.category_folder().into(),
                w.file_count(),
            ),
        )
    }

    #[test]
    fn single_file_goes_flat_under_its_circle() {
        let p = placement("LunaSoft/game_v1.0.zip", 1);
        assert_eq!(
            p.target().as_deref(),
            Some("04 角色扮演(RPG)/LunaSoft/game_v1.0.zip")
        );
    }

    #[test]
    fn multiple_files_get_a_sanitized_work_folder() {
        let p = placement("RPG/game_v1.0.zip", 2);
        assert_eq!(
            p.target().as_deref(),
            Some("04 角色扮演(RPG)/LunaSoft/Game_ Title_/game_v1.0.zip")
        );
    }

    #[test]
    fn flat_file_moves_into_the_work_folder_when_a_second_file_arrives() {
        let p = placement("04 角色扮演(RPG)/LunaSoft/game_v1.0.zip", 2);
        assert_eq!(
            p.target().as_deref(),
            Some("04 角色扮演(RPG)/LunaSoft/Game_ Title_/game_v1.0.zip")
        );
    }

    #[test]
    fn files_in_place_need_no_move() {
        assert_eq!(
            placement("04 角色扮演(RPG)/LunaSoft/game_v1.0.zip", 1).target(),
            None
        );
        assert_eq!(
            placement("04 角色扮演(RPG)/LunaSoft/Game_ Title_/game_v1.0.zip", 2).target(),
            None
        );
    }

    #[test]
    fn work_folder_is_not_flattened_when_one_file_remains() {
        let p = placement("04 角色扮演(RPG)/LunaSoft/Game_ Title_/game_v1.0.zip", 1);
        assert_eq!(p.target(), None);
    }

    #[test]
    fn category_change_moves_out_of_the_old_category() {
        let p = placement("05 策略模擬(SLG)/LunaSoft/game_v1.0.zip", 1);
        assert_eq!(
            p.target().as_deref(),
            Some("04 角色扮演(RPG)/LunaSoft/game_v1.0.zip")
        );
    }

    #[test]
    fn missing_or_blank_circle_uses_the_unknown_folder() {
        let p = with_circle(placement("未分類/game_v1.0.zip", 1), None);
        assert_eq!(
            p.target().as_deref(),
            Some("04 角色扮演(RPG)/未知社團/game_v1.0.zip")
        );
        let p = with_circle(placement("未分類/game_v1.0.zip", 1), Some(" ... "));
        assert_eq!(
            p.target().as_deref(),
            Some("04 角色扮演(RPG)/未知社團/game_v1.0.zip")
        );
    }

    #[test]
    fn protects_category_folders_the_inbox_and_the_recycle_bin() {
        let protected = protected_folders(&crate::domain::vocabulary::tests::sample());
        let names: Vec<&str> = protected
            .folders()
            .iter()
            .map(LibraryPath::as_str)
            .collect();
        assert!(names.contains(&"04 角色扮演(RPG)"));
        assert!(names.contains(&INBOX_FOLDER));
        assert!(names.contains(&RECYCLE_FOLDER));
    }
}
