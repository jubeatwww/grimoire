//! Primary-category suggestions for works that have not been classified yet.
//!
//! Pure function over a work's facts and the editable rules; see
//! `docs/library-reclassification.md` ("Suggestions") for the agreed policy:
//!
//! 1. A source's work type rule proposes a code and its category.
//! 2. Keyword rules over that source's tags add codes. A keyword code in the
//!    work type's own category replaces the type code (FTG refines ACT). A
//!    keyword flagged `contests_type` landing in another category makes the
//!    suggestion not confident (アクションRPG on a DLsite RPG).
//! 3. Without a work type category, keyword codes decide, then hints. Such
//!    suggestions are never confident.
//! 4. Confident needs a confident type rule, agreeing sources, no contesting
//!    keyword, and every hint containing the suggested category.

use crate::vocabulary::Vocabulary;
use serde::Serialize;

#[derive(Debug, Clone, Default)]
pub struct SourceFacts {
    pub source: String,
    pub work_type: Option<String>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct WorkFacts {
    pub sources: Vec<SourceFacts>,
    /// Top-level folder of each of the work's files.
    pub legacy_locations: Vec<String>,
    /// Old per-file category values ("Visual Novel", "Action", …).
    pub old_primaries: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkTypeRule {
    pub source: String,
    pub work_type: String,
    pub code: Option<String>,
    pub confident: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KeywordRule {
    pub id: String,
    pub source: Option<String>,
    pub keyword: String,
    pub code: String,
    pub contests_type: bool,
}

#[derive(Debug, Clone, Default)]
pub struct Rules {
    pub work_types: Vec<WorkTypeRule>,
    pub keywords: Vec<KeywordRule>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Suggestion {
    pub category: Option<String>,
    pub codes: Vec<String>,
    pub confident: bool,
    pub reasons: Vec<String>,
}

/// Categories a legacy top-level folder points at. A folder that already is a
/// new category folder points at that category.
pub fn legacy_folder_hint(vocab: &Vocabulary, folder: &str) -> Vec<String> {
    let ids: &[&str] = match folder {
        "RPG" => &["RPG"],
        "ADV" => &["AVG"],
        "ACT" => &["ACT"],
        "SIM+SLG" => &["SLG", "SIM"],
        other => {
            return vocab
                .categories()
                .iter()
                .filter(|c| c.folder_name == other)
                .map(|c| c.id.clone())
                .collect();
        }
    };
    ids.iter().map(|s| s.to_string()).collect()
}

/// Categories an old (pre-reclassification) category value points at.
pub fn old_primary_hint(value: &str) -> Vec<String> {
    let ids: &[&str] = match value {
        "Visual Novel" => &["AVG"],
        "Action" => &["ACT"],
        "RPG" => &["RPG"],
        "Strategy" => &["SLG"],
        _ => &[],
    };
    ids.iter().map(|s| s.to_string()).collect()
}

struct KeywordHit<'a> {
    rule: &'a KeywordRule,
    code: String,
    category: String,
    tag: &'a str,
}

fn keyword_hits<'a>(
    vocab: &Vocabulary,
    rules: &'a Rules,
    source: &'a SourceFacts,
) -> Vec<KeywordHit<'a>> {
    let mut hits: Vec<KeywordHit<'a>> = Vec::new();
    for rule in &rules.keywords {
        if rule.source.as_deref().is_some_and(|s| s != source.source) {
            continue;
        }
        let needle = rule.keyword.to_lowercase();
        if needle.is_empty() {
            continue;
        }
        let Some(tag) = source
            .tags
            .iter()
            .find(|t| t.to_lowercase().contains(&needle))
        else {
            continue;
        };
        let Some(resolved) = vocab.resolve(&rule.code) else {
            continue;
        };
        let code = resolved.canonical();
        if hits.iter().any(|h| h.code == code) {
            continue;
        }
        hits.push(KeywordHit {
            rule,
            code,
            category: resolved.base.category_id.clone(),
            tag,
        });
    }
    hits
}

fn push_unique(list: &mut Vec<String>, value: String) {
    if !list.contains(&value) {
        list.push(value);
    }
}

fn category_name(vocab: &Vocabulary, id: &str) -> String {
    vocab
        .category(id)
        .map(|c| c.folder_name.clone())
        .unwrap_or_else(|| id.to_string())
}

pub fn suggest(vocab: &Vocabulary, rules: &Rules, facts: &WorkFacts) -> Suggestion {
    let mut codes: Vec<String> = Vec::new();
    let mut reasons: Vec<String> = Vec::new();
    let mut contested = false;
    // (category, rule is confident) per source that had a usable type rule.
    let mut type_votes: Vec<(String, bool)> = Vec::new();
    // Keyword categories from sources whose type rule deferred to tags.
    let mut tag_votes: Vec<String> = Vec::new();

    for source in &facts.sources {
        let hits = keyword_hits(vocab, rules, source);
        let rule = source.work_type.as_deref().and_then(|wt| {
            rules
                .work_types
                .iter()
                .find(|r| r.source == source.source && r.work_type.eq_ignore_ascii_case(wt))
        });
        let typed = rule.and_then(|r| {
            let code = r.code.as_deref()?;
            let resolved = vocab.resolve(code)?;
            Some((r, resolved.canonical(), resolved.base.category_id.clone()))
        });

        match typed {
            Some((rule, type_code, category)) => {
                let refinements: Vec<&KeywordHit> =
                    hits.iter().filter(|h| h.category == category).collect();
                if refinements.is_empty() {
                    push_unique(&mut codes, type_code.clone());
                    reasons.push(format!(
                        "{} {} → {}",
                        source.source, rule.work_type, type_code
                    ));
                } else {
                    for hit in &refinements {
                        push_unique(&mut codes, hit.code.clone());
                        reasons.push(format!(
                            "{} {} refined by tag {} → {}",
                            source.source, rule.work_type, hit.tag, hit.code
                        ));
                    }
                }
                for hit in hits.iter().filter(|h| h.category != category) {
                    push_unique(&mut codes, hit.code.clone());
                    if hit.rule.contests_type {
                        contested = true;
                        reasons.push(format!(
                            "tag {} suggests {} instead of {}",
                            hit.tag,
                            category_name(vocab, &hit.category),
                            category_name(vocab, &category)
                        ));
                    } else {
                        reasons.push(format!("tag {} adds {}", hit.tag, hit.code));
                    }
                }
                type_votes.push((category, rule.confident));
            }
            None => {
                match (source.work_type.as_deref(), rule) {
                    (Some(wt), None) => {
                        reasons.push(format!("no rule for {} type {}", source.source, wt))
                    }
                    (Some(wt), Some(_)) => {
                        reasons.push(format!("{} {} → decided by tags", source.source, wt))
                    }
                    (None, _) => {}
                }
                for hit in &hits {
                    push_unique(&mut codes, hit.code.clone());
                    tag_votes.push(hit.category.clone());
                    reasons.push(format!("tag {} → {}", hit.tag, hit.code));
                }
            }
        }
    }

    let mut type_categories: Vec<String> = Vec::new();
    for (category, _) in &type_votes {
        push_unique(&mut type_categories, category.clone());
    }

    let mut from_types = false;
    let mut category: Option<String> = match type_categories.as_slice() {
        [] => None,
        [only] => {
            from_types = true;
            Some(only.clone())
        }
        [first, ..] => {
            from_types = true;
            contested = true;
            reasons.push(format!(
                "sources disagree: {}",
                type_categories
                    .iter()
                    .map(|c| category_name(vocab, c))
                    .collect::<Vec<_>>()
                    .join(" / ")
            ));
            Some(first.clone())
        }
    };

    if category.is_none() {
        category = most_voted(vocab, &tag_votes);
        if let Some(c) = &category {
            let distinct = {
                let mut d: Vec<&String> = tag_votes.iter().collect();
                d.sort();
                d.dedup();
                d.len()
            };
            if distinct > 1 {
                reasons.push(format!(
                    "tags point to several categories; picked {}",
                    category_name(vocab, c)
                ));
            }
        }
    }

    // Hints: non-empty hint sets per file and per old category value.
    let mut hint_sets: Vec<(String, Vec<String>)> = Vec::new();
    for folder in &facts.legacy_locations {
        let set = legacy_folder_hint(vocab, folder);
        if !set.is_empty() {
            hint_sets.push((format!("legacy folder {folder}"), set));
        }
    }
    for old in &facts.old_primaries {
        let set = old_primary_hint(old);
        if !set.is_empty() {
            hint_sets.push((format!("old category {old}"), set));
        }
    }

    match &category {
        Some(c) => {
            for (what, set) in &hint_sets {
                if !set.contains(c) {
                    contested = true;
                    reasons.push(format!(
                        "{what} points to {}",
                        set.iter()
                            .map(|s| category_name(vocab, s))
                            .collect::<Vec<_>>()
                            .join(" or ")
                    ));
                }
            }
        }
        None => {
            // Only hints left: take the categories every hint agrees on.
            let mut common: Option<Vec<String>> = None;
            for (_, set) in &hint_sets {
                common = Some(match common {
                    None => set.clone(),
                    Some(prev) => prev.into_iter().filter(|c| set.contains(c)).collect(),
                });
            }
            match common.as_deref() {
                Some([only]) => {
                    category = Some(only.clone());
                    reasons.push(format!(
                        "only hints available: {}",
                        hint_sets
                            .iter()
                            .map(|(w, _)| w.as_str())
                            .collect::<Vec<_>>()
                            .join(", ")
                    ));
                }
                Some(several) if several.len() > 1 => reasons.push(format!(
                    "hints allow {}",
                    several
                        .iter()
                        .map(|s| category_name(vocab, s))
                        .collect::<Vec<_>>()
                        .join(" or ")
                )),
                Some(_) => reasons.push("hints disagree".to_string()),
                None => {}
            }
        }
    }

    if category.is_none() && reasons.is_empty() {
        reasons.push("no source data or hints".to_string());
    }

    let confident = category.is_some()
        && from_types
        && type_votes.iter().all(|(_, confident)| *confident)
        && !contested;

    // Final code list: known codes only, plus the category's own code.
    let known: Vec<&str> = codes
        .iter()
        .map(String::as_str)
        .filter(|c| vocab.resolve(c).is_some())
        .collect();
    let codes = vocab
        .prepare_classification(category.as_deref(), known)
        .unwrap_or_default();

    Suggestion {
        category,
        codes,
        confident,
        reasons,
    }
}

/// The category with the most keyword votes; ties go to the lower folder number.
fn most_voted(vocab: &Vocabulary, votes: &[String]) -> Option<String> {
    let mut best: Option<(&String, usize, i32)> = None;
    for candidate in votes {
        let count = votes.iter().filter(|v| *v == candidate).count();
        let order = vocab
            .category(candidate)
            .map(|c| c.sort_order)
            .unwrap_or(i32::MAX);
        let better = match best {
            None => true,
            Some((_, best_count, best_order)) => {
                count > best_count || (count == best_count && order < best_order)
            }
        };
        if better {
            best = Some((candidate, count, order));
        }
    }
    best.map(|(c, _, _)| c.clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vocabulary::tests::sample;

    fn type_rule(source: &str, work_type: &str, code: Option<&str>, confident: bool) -> WorkTypeRule {
        WorkTypeRule {
            source: source.to_string(),
            work_type: work_type.to_string(),
            code: code.map(str::to_string),
            confident,
        }
    }

    fn keyword(source: Option<&str>, keyword: &str, code: &str, contests_type: bool) -> KeywordRule {
        KeywordRule {
            id: keyword.to_string(),
            source: source.map(str::to_string),
            keyword: keyword.to_string(),
            code: code.to_string(),
            contests_type,
        }
    }

    fn rules() -> Rules {
        Rules {
            work_types: vec![
                type_rule("dlsite", "ACN", Some("ACT"), true),
                type_rule("dlsite", "RPG", Some("RPG"), true),
                type_rule("dlsite", "ADV", Some("ADV"), true),
                type_rule("dlsite", "SLN", None, false),
                type_rule("dlsite", "MNG", Some("ETC"), true),
                type_rule("vndb", "ADV", Some("AVG"), true),
                type_rule("steam", "STEAM", None, false),
            ],
            keywords: vec![
                keyword(None, "格闘", "FTG", false),
                keyword(None, "調教", "EDU", false),
                keyword(None, "戦略", "SLG", false),
                keyword(None, "経営", "SIM", false),
                keyword(None, "タッチ", "T-HAG", false),
                keyword(None, "アクションRPG", "ARPG", true),
                keyword(Some("vndb"), "No Choices", "NVL", false),
                keyword(Some("steam"), "動作", "ACT", false),
                keyword(Some("steam"), "冒險", "ADV", false),
            ],
        }
    }

    fn dlsite(work_type: &str, tags: &[&str]) -> SourceFacts {
        SourceFacts {
            source: "dlsite".to_string(),
            work_type: Some(work_type.to_string()),
            tags: tags.iter().map(|t| t.to_string()).collect(),
        }
    }

    fn facts(sources: Vec<SourceFacts>) -> WorkFacts {
        WorkFacts {
            sources,
            ..WorkFacts::default()
        }
    }

    #[test]
    fn confident_type_rule_gives_a_confident_suggestion() {
        let s = suggest(&sample(), &rules(), &facts(vec![dlsite("RPG", &["ファンタジー"])]));
        assert_eq!(s.category.as_deref(), Some("RPG"));
        assert_eq!(s.codes, vec!["RPG"]);
        assert!(s.confident);
    }

    #[test]
    fn secondary_keywords_add_codes_without_hurting_confidence() {
        let s = suggest(&sample(), &rules(), &facts(vec![dlsite("RPG", &["調教"])]));
        assert_eq!(s.category.as_deref(), Some("RPG"));
        assert_eq!(s.codes, vec!["RPG", "EDU"]);
        assert!(s.confident);
    }

    #[test]
    fn same_category_keyword_refines_the_type_code() {
        let s = suggest(&sample(), &rules(), &facts(vec![dlsite("ACN", &["格闘"])]));
        assert_eq!(s.category.as_deref(), Some("ACT"));
        assert_eq!(s.codes, vec!["FTG"]);
        assert!(s.confident);
    }

    #[test]
    fn contesting_keyword_keeps_the_type_category_but_is_not_confident() {
        let s = suggest(&sample(), &rules(), &facts(vec![dlsite("RPG", &["アクションRPG"])]));
        assert_eq!(s.category.as_deref(), Some("RPG"));
        assert_eq!(s.codes, vec!["RPG", "ARPG"]);
        assert!(!s.confident);
    }

    #[test]
    fn deferred_type_is_decided_by_tags_and_never_confident() {
        let s = suggest(&sample(), &rules(), &facts(vec![dlsite("SLN", &["経営", "調教"])]));
        assert_eq!(s.category.as_deref(), Some("SIM"));
        // Codes follow rule order; both belong to 經營養成.
        assert_eq!(s.codes, vec!["EDU", "SIM"]);
        assert!(!s.confident);
    }

    #[test]
    fn prefixed_keyword_codes_resolve_to_their_category() {
        let s = suggest(&sample(), &rules(), &facts(vec![dlsite("SLN", &["タッチ"])]));
        assert_eq!(s.category.as_deref(), Some("HAG"));
        assert_eq!(s.codes, vec!["T-HAG"]);
    }

    #[test]
    fn deferred_type_without_matching_tags_has_no_category() {
        let s = suggest(&sample(), &rules(), &facts(vec![dlsite("SLN", &["ファンタジー"])]));
        assert_eq!(s.category, None);
        assert!(!s.confident);
        assert!(s.codes.is_empty());
    }

    #[test]
    fn vndb_no_choices_refines_avg_to_nvl() {
        let vndb = SourceFacts {
            source: "vndb".to_string(),
            work_type: Some("ADV".to_string()),
            tags: vec!["No Choices".to_string(), "Romance".to_string()],
        };
        let s = suggest(&sample(), &rules(), &facts(vec![vndb]));
        assert_eq!(s.category.as_deref(), Some("AVG"));
        assert_eq!(s.codes, vec!["NVL"]);
        assert!(s.confident);
    }

    #[test]
    fn source_limited_keywords_ignore_other_sources() {
        let s = suggest(&sample(), &rules(), &facts(vec![dlsite("SLN", &["No Choices"])]));
        assert_eq!(s.category, None);
    }

    #[test]
    fn steam_is_decided_by_genres_with_ties_to_the_lower_folder() {
        let steam = SourceFacts {
            source: "steam".to_string(),
            work_type: Some("STEAM".to_string()),
            tags: vec!["動作".to_string(), "冒險".to_string()],
        };
        let s = suggest(&sample(), &rules(), &facts(vec![steam]));
        // ADV (01 圖文冒險) beats ACT (02 動作格鬥) on a 1:1 tie.
        assert_eq!(s.category.as_deref(), Some("AVG"));
        assert!(!s.confident);
    }

    #[test]
    fn non_game_types_go_to_etc() {
        let s = suggest(&sample(), &rules(), &facts(vec![dlsite("MNG", &[])]));
        assert_eq!(s.category.as_deref(), Some("ETC"));
        assert!(s.confident);
    }

    #[test]
    fn conflicting_legacy_folder_removes_confidence() {
        let mut f = facts(vec![dlsite("ACN", &[])]);
        f.legacy_locations = vec!["RPG".to_string()];
        let s = suggest(&sample(), &rules(), &f);
        assert_eq!(s.category.as_deref(), Some("ACT"));
        assert!(!s.confident);
    }

    #[test]
    fn agreeing_hints_keep_confidence() {
        let mut f = facts(vec![dlsite("RPG", &[])]);
        f.legacy_locations = vec!["RPG".to_string(), "未分類".to_string()];
        f.old_primaries = vec!["RPG".to_string(), "Simulation".to_string()];
        let s = suggest(&sample(), &rules(), &f);
        assert!(s.confident);
    }

    #[test]
    fn sim_slg_folder_accepts_either_category() {
        let mut f = facts(vec![dlsite("SLN", &["戦略"])]);
        f.legacy_locations = vec!["SIM+SLG".to_string()];
        let s = suggest(&sample(), &rules(), &f);
        assert_eq!(s.category.as_deref(), Some("SLG"));
        assert!(s.reasons.iter().all(|r| !r.contains("points to")));
    }

    #[test]
    fn hints_alone_suggest_without_confidence() {
        let f = WorkFacts {
            sources: vec![],
            legacy_locations: vec!["ADV".to_string()],
            old_primaries: vec!["Visual Novel".to_string()],
        };
        let s = suggest(&sample(), &rules(), &f);
        assert_eq!(s.category.as_deref(), Some("AVG"));
        assert_eq!(s.codes, vec!["AVG"]);
        assert!(!s.confident);
    }

    #[test]
    fn ambiguous_hints_leave_the_category_open() {
        let f = WorkFacts {
            sources: vec![],
            legacy_locations: vec!["SIM+SLG".to_string()],
            old_primaries: vec![],
        };
        let s = suggest(&sample(), &rules(), &f);
        assert_eq!(s.category, None);
        assert!(s.reasons.iter().any(|r| r.contains("hints allow")));
    }

    #[test]
    fn new_category_folder_counts_as_a_hint() {
        let f = WorkFacts {
            sources: vec![],
            legacy_locations: vec!["04 角色扮演(RPG)".to_string()],
            old_primaries: vec![],
        };
        let s = suggest(&sample(), &rules(), &f);
        assert_eq!(s.category.as_deref(), Some("RPG"));
    }

    #[test]
    fn disagreeing_sources_are_not_confident() {
        let vndb = SourceFacts {
            source: "vndb".to_string(),
            work_type: Some("ADV".to_string()),
            tags: vec![],
        };
        let s = suggest(&sample(), &rules(), &facts(vec![dlsite("RPG", &[]), vndb]));
        assert_eq!(s.category.as_deref(), Some("RPG"));
        assert!(!s.confident);
    }

    #[test]
    fn unknown_work_type_is_reported() {
        let s = suggest(&sample(), &rules(), &facts(vec![dlsite("XYZ", &[])]));
        assert_eq!(s.category, None);
        assert!(s.reasons.iter().any(|r| r.contains("no rule for dlsite type XYZ")));
    }
}
