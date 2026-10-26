use crate::domain::vocabulary::Vocabulary;

/// Categories a legacy top-level folder points at. A folder that already is a new category
/// folder points at that category.
pub(super) fn legacy_folder_hint(vocab: &Vocabulary, folder: &str) -> Vec<String> {
    let ids: &[&str] = match folder {
        "RPG" => &["RPG"],
        "ADV" => &["AVG"],
        "ACT" => &["ACT"],
        "SIM+SLG" => &["SLG", "SIM"],
        other => {
            return vocab
                .categories()
                .iter()
                .filter(|c| c.folder_name() == other)
                .map(|c| c.id().to_owned())
                .collect();
        }
    };
    ids.iter().map(|s| s.to_string()).collect()
}

/// Categories an old (pre-reclassification) category value points at.
pub(super) fn old_primary_hint(value: &str) -> Vec<String> {
    let ids: &[&str] = match value {
        "Visual Novel" => &["AVG"],
        "Action" => &["ACT"],
        "RPG" => &["RPG"],
        "Strategy" => &["SLG"],
        _ => &[],
    };
    ids.iter().map(|s| s.to_string()).collect()
}
