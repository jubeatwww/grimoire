use super::MAX_SEGMENT_BYTES;
use crate::domain::library_path::is_reserved_folder_name;

/// Make a string safe as one folder name on the NAS, over SMB and on Windows clients: path and
/// reserved characters become `_`, control characters are dropped, surrounding spaces and
/// trailing dots are trimmed, and the result is capped at `MAX_SEGMENT_BYTES` on a character
/// boundary.
pub(crate) fn sanitize_segment(value: &str) -> String {
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

    #[test]
    fn trims_trailing_dots_and_control_characters() {
        assert_eq!(sanitize_segment("  Circle.\u{0007}.. "), "Circle");
        assert_eq!(sanitize_segment("a/b\\c"), "a_b_c");
    }

    #[test]
    fn caps_length_on_a_character_boundary() {
        let long = "あ".repeat(100); // 300 bytes
        let safe = sanitize_segment(&long);
        assert!(safe.len() <= MAX_SEGMENT_BYTES);
        assert_eq!(safe.chars().count(), (MAX_SEGMENT_BYTES - 1) / 3);
    }

    #[test]
    fn never_returns_metadata_or_hidden_names() {
        assert_eq!(sanitize_segment("._Foo"), "_._Foo");
        assert_eq!(sanitize_segment("@eaDir"), "_@eaDir");
        assert_eq!(sanitize_segment("Thumbs.db"), "_Thumbs.db");
        assert_eq!(sanitize_segment(".hack//G.U."), "_.hack__G.U");
    }
}
