//! Where a classified file belongs on the NAS.
//!
//! ```text
//! <category folder>/<circle>/<original file name>
//! <category folder>/<circle>/<work title>/<original file name>   (2+ files)
//! ```
//!
//! File names never change. A file already inside its work folder stays there
//! even when the work drops back to one file (no auto-flatten).

use crate::library_writer::is_reserved_folder_name;

/// Inbox for new downloads at the library root. Never pruned.
pub const INBOX_FOLDER: &str = "00 待整理";
/// Circle folder for works without a circle or developer.
pub const UNKNOWN_CIRCLE: &str = "未知社團";
/// Folder name byte cap. btrfs/ext4 allow 255; leave room for safety.
pub const MAX_SEGMENT_BYTES: usize = 200;

#[derive(Debug, Clone, Copy)]
pub struct FilePlacement<'a> {
    pub category_folder: &'a str,
    pub circle: Option<&'a str>,
    pub work_title: &'a str,
    pub file_name: &'a str,
    /// Current path relative to the library root, `/`-separated.
    pub current_path: &'a str,
    /// Present, non-excluded files linked to the same work.
    pub work_file_count: usize,
}

fn circle_segment(circle: Option<&str>) -> String {
    let safe = circle.map(sanitize_segment).unwrap_or_default();
    if safe.is_empty() {
        UNKNOWN_CIRCLE.to_string()
    } else {
        safe
    }
}

fn title_segment(title: &str) -> String {
    let safe = sanitize_segment(title);
    if safe.is_empty() {
        "Untitled".to_string()
    } else {
        safe
    }
}

/// `<category>/<circle>/<file>`
pub fn flat_target(p: &FilePlacement) -> String {
    format!(
        "{}/{}/{}",
        p.category_folder,
        circle_segment(p.circle),
        p.file_name
    )
}

/// `<category>/<circle>/<title>/<file>`
pub fn work_folder_target(p: &FilePlacement) -> String {
    format!(
        "{}/{}/{}/{}",
        p.category_folder,
        circle_segment(p.circle),
        title_segment(p.work_title),
        p.file_name
    )
}

/// The path the file should move to, or `None` when it already sits in an
/// accepted place.
pub fn plan_target(p: &FilePlacement) -> Option<String> {
    let in_work = work_folder_target(p);
    if p.work_file_count >= 2 {
        return (p.current_path != in_work).then_some(in_work);
    }
    let flat = flat_target(p);
    if p.current_path == flat || p.current_path == in_work {
        return None;
    }
    Some(flat)
}

/// Make a string safe as one folder name on the NAS, over SMB and on Windows
/// clients: path and reserved characters become `_`, control characters are
/// dropped, surrounding spaces and trailing dots are trimmed, and the result
/// is capped at `MAX_SEGMENT_BYTES` on a character boundary.
pub fn sanitize_segment(value: &str) -> String {
    let replaced: String = value
        .chars()
        .filter(|ch| !ch.is_control())
        .map(|ch| match ch {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            _ => ch,
        })
        .collect();
    let mut out = String::new();
    for ch in replaced.trim().chars() {
        // Leave room for the "_" a reserved name may get below.
        if out.len() + ch.len_utf8() > MAX_SEGMENT_BYTES - 1 {
            break;
        }
        out.push(ch);
    }
    let out = out.trim_end_matches([' ', '.']).trim().to_string();
    // Never produce a name pruning would treat as metadata, or a hidden one.
    if !out.is_empty() && is_reserved_folder_name(&out) {
        format!("_{out}")
    } else {
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn placement<'a>(current_path: &'a str, work_file_count: usize) -> FilePlacement<'a> {
        FilePlacement {
            category_folder: "04 角色扮演(RPG)",
            circle: Some("LunaSoft"),
            work_title: "Game: Title?",
            file_name: "game_v1.0.zip",
            current_path,
            work_file_count,
        }
    }

    #[test]
    fn single_file_goes_flat_under_its_circle() {
        let p = placement("LunaSoft/game_v1.0.zip", 1);
        assert_eq!(
            plan_target(&p).as_deref(),
            Some("04 角色扮演(RPG)/LunaSoft/game_v1.0.zip")
        );
    }

    #[test]
    fn multiple_files_get_a_sanitized_work_folder() {
        let p = placement("RPG/game_v1.0.zip", 2);
        assert_eq!(
            plan_target(&p).as_deref(),
            Some("04 角色扮演(RPG)/LunaSoft/Game_ Title_/game_v1.0.zip")
        );
    }

    #[test]
    fn flat_file_moves_into_the_work_folder_when_a_second_file_arrives() {
        let p = placement("04 角色扮演(RPG)/LunaSoft/game_v1.0.zip", 2);
        assert_eq!(
            plan_target(&p).as_deref(),
            Some("04 角色扮演(RPG)/LunaSoft/Game_ Title_/game_v1.0.zip")
        );
    }

    #[test]
    fn files_in_place_need_no_move() {
        assert_eq!(plan_target(&placement("04 角色扮演(RPG)/LunaSoft/game_v1.0.zip", 1)), None);
        assert_eq!(
            plan_target(&placement("04 角色扮演(RPG)/LunaSoft/Game_ Title_/game_v1.0.zip", 2)),
            None
        );
    }

    #[test]
    fn work_folder_is_not_flattened_when_one_file_remains() {
        let p = placement("04 角色扮演(RPG)/LunaSoft/Game_ Title_/game_v1.0.zip", 1);
        assert_eq!(plan_target(&p), None);
    }

    #[test]
    fn category_change_moves_out_of_the_old_category() {
        let p = placement("05 策略模擬(SLG)/LunaSoft/game_v1.0.zip", 1);
        assert_eq!(
            plan_target(&p).as_deref(),
            Some("04 角色扮演(RPG)/LunaSoft/game_v1.0.zip")
        );
    }

    #[test]
    fn missing_or_blank_circle_uses_the_unknown_folder() {
        let mut p = placement("未分類/game_v1.0.zip", 1);
        p.circle = None;
        assert_eq!(
            plan_target(&p).as_deref(),
            Some("04 角色扮演(RPG)/未知社團/game_v1.0.zip")
        );
        p.circle = Some(" ... ");
        assert_eq!(
            plan_target(&p).as_deref(),
            Some("04 角色扮演(RPG)/未知社團/game_v1.0.zip")
        );
    }

    #[test]
    fn sanitize_trims_trailing_dots_and_control_characters() {
        assert_eq!(sanitize_segment("  Circle.\u{0007}.. "), "Circle");
        assert_eq!(sanitize_segment("a/b\\c"), "a_b_c");
    }

    #[test]
    fn sanitize_caps_length_on_a_character_boundary() {
        let long = "あ".repeat(100); // 300 bytes
        let safe = sanitize_segment(&long);
        assert!(safe.len() <= MAX_SEGMENT_BYTES);
        assert_eq!(safe.chars().count(), (MAX_SEGMENT_BYTES - 1) / 3);
    }

    #[test]
    fn sanitize_never_returns_metadata_or_hidden_names() {
        assert_eq!(sanitize_segment("._Foo"), "_._Foo");
        assert_eq!(sanitize_segment("@eaDir"), "_@eaDir");
        assert_eq!(sanitize_segment("Thumbs.db"), "_Thumbs.db");
        assert_eq!(sanitize_segment(".hack//G.U."), "_.hack__G.U");
    }
}
