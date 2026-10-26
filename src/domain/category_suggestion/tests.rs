use super::*;
use crate::domain::keyword_rule::{KeywordRule, KeywordRuleId};
use crate::domain::metadata_source::MetadataProvider;
use crate::domain::vocabulary::tests::sample;
use crate::domain::work_type_rule::WorkTypeRule;

fn provider(name: &str) -> MetadataProvider {
    MetadataProvider::parse(name).unwrap()
}

fn type_rule(source: &str, work_type: &str, code: Option<&str>, confident: bool) -> WorkTypeRule {
    WorkTypeRule::from_persisted(
        provider(source),
        work_type.to_string(),
        code.map(str::to_string),
        confident,
    )
}

fn keyword(source: Option<&str>, keyword: &str, code: &str, contests_type: bool) -> KeywordRule {
    KeywordRule::from_persisted(
        KeywordRuleId::from_raw(uuid::Uuid::from_u128(1)),
        source.map(provider),
        keyword.to_string(),
        code.to_string(),
        contests_type,
    )
}

fn rules() -> ClassificationRules {
    ClassificationRules::new(
        vec![
            type_rule("dlsite", "ACN", Some("ACT"), true),
            type_rule("dlsite", "RPG", Some("RPG"), true),
            type_rule("dlsite", "ADV", Some("ADV"), true),
            type_rule("dlsite", "SLN", None, false),
            type_rule("dlsite", "MNG", Some("ETC"), true),
            type_rule("vndb", "ADV", Some("AVG"), true),
            type_rule("steam", "STEAM", None, false),
        ],
        vec![
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
    )
}

fn dlsite(work_type: &str, tags: &[&str]) -> SourceFacts {
    SourceFacts::new(
        MetadataProvider::Dlsite,
        Some(work_type.to_string()),
        tags.iter().map(|t| t.to_string()).collect(),
    )
}

fn facts(sources: Vec<SourceFacts>) -> WorkFacts {
    WorkFacts::new(sources, Vec::new(), Vec::new())
}

#[test]
fn confident_type_rule_gives_a_confident_suggestion() {
    let s = CategorySuggestion::derive(
        &sample(),
        &rules(),
        &facts(vec![dlsite("RPG", &["ファンタジー"])]),
    );
    assert_eq!(s.category(), Some("RPG"));
    assert_eq!(s.codes(), vec!["RPG"]);
    assert!(s.confident());
}

#[test]
fn secondary_keywords_add_codes_without_hurting_confidence() {
    let s = CategorySuggestion::derive(&sample(), &rules(), &facts(vec![dlsite("RPG", &["調教"])]));
    assert_eq!(s.category(), Some("RPG"));
    assert_eq!(s.codes(), vec!["RPG", "EDU"]);
    assert!(s.confident());
}

#[test]
fn same_category_keyword_refines_the_type_code() {
    let s = CategorySuggestion::derive(&sample(), &rules(), &facts(vec![dlsite("ACN", &["格闘"])]));
    assert_eq!(s.category(), Some("ACT"));
    assert_eq!(s.codes(), vec!["FTG"]);
    assert!(s.confident());
}

#[test]
fn contesting_keyword_keeps_the_type_category_but_is_not_confident() {
    let s = CategorySuggestion::derive(
        &sample(),
        &rules(),
        &facts(vec![dlsite("RPG", &["アクションRPG"])]),
    );
    assert_eq!(s.category(), Some("RPG"));
    assert_eq!(s.codes(), vec!["RPG", "ARPG"]);
    assert!(!s.confident());
}

#[test]
fn deferred_type_is_decided_by_tags_and_never_confident() {
    let s = CategorySuggestion::derive(
        &sample(),
        &rules(),
        &facts(vec![dlsite("SLN", &["経営", "調教"])]),
    );
    assert_eq!(s.category(), Some("SIM"));
    // Codes follow rule order; both belong to 經營養成.
    assert_eq!(s.codes(), vec!["EDU", "SIM"]);
    assert!(!s.confident());
}

#[test]
fn prefixed_keyword_codes_resolve_to_their_category() {
    let s = CategorySuggestion::derive(
        &sample(),
        &rules(),
        &facts(vec![dlsite("SLN", &["タッチ"])]),
    );
    assert_eq!(s.category(), Some("HAG"));
    assert_eq!(s.codes(), vec!["T-HAG"]);
}

#[test]
fn deferred_type_without_matching_tags_has_no_category() {
    let s = CategorySuggestion::derive(
        &sample(),
        &rules(),
        &facts(vec![dlsite("SLN", &["ファンタジー"])]),
    );
    assert_eq!(s.category(), None);
    assert!(!s.confident());
    assert!(s.codes().is_empty());
}

#[test]
fn vndb_no_choices_refines_avg_to_nvl() {
    let vndb = SourceFacts::new(
        MetadataProvider::Vndb,
        Some("ADV".to_string()),
        vec!["No Choices".to_string(), "Romance".to_string()],
    );
    let s = CategorySuggestion::derive(&sample(), &rules(), &facts(vec![vndb]));
    assert_eq!(s.category(), Some("AVG"));
    assert_eq!(s.codes(), vec!["NVL"]);
    assert!(s.confident());
}

#[test]
fn source_limited_keywords_ignore_other_sources() {
    let s = CategorySuggestion::derive(
        &sample(),
        &rules(),
        &facts(vec![dlsite("SLN", &["No Choices"])]),
    );
    assert_eq!(s.category(), None);
}

#[test]
fn steam_is_decided_by_genres_with_ties_to_the_lower_folder() {
    let steam = SourceFacts::new(
        MetadataProvider::Steam,
        Some("STEAM".to_string()),
        vec!["動作".to_string(), "冒險".to_string()],
    );
    let s = CategorySuggestion::derive(&sample(), &rules(), &facts(vec![steam]));
    // ADV (01 圖文冒險) beats ACT (02 動作格鬥) on a 1:1 tie.
    assert_eq!(s.category(), Some("AVG"));
    assert!(!s.confident());
}

#[test]
fn non_game_types_go_to_etc() {
    let s = CategorySuggestion::derive(&sample(), &rules(), &facts(vec![dlsite("MNG", &[])]));
    assert_eq!(s.category(), Some("ETC"));
    assert!(s.confident());
}

#[test]
fn conflicting_legacy_folder_removes_confidence() {
    let f = WorkFacts::new(vec![dlsite("ACN", &[])], vec!["RPG".to_string()], vec![]);
    let s = CategorySuggestion::derive(&sample(), &rules(), &f);
    assert_eq!(s.category(), Some("ACT"));
    assert!(!s.confident());
}

#[test]
fn agreeing_hints_keep_confidence() {
    let f = WorkFacts::new(
        vec![dlsite("RPG", &[])],
        vec!["RPG".to_string(), "未分類".to_string()],
        vec!["RPG".to_string(), "Simulation".to_string()],
    );
    let s = CategorySuggestion::derive(&sample(), &rules(), &f);
    assert!(s.confident());
}

#[test]
fn sim_slg_folder_accepts_either_category() {
    let f = WorkFacts::new(
        vec![dlsite("SLN", &["戦略"])],
        vec!["SIM+SLG".to_string()],
        vec![],
    );
    let s = CategorySuggestion::derive(&sample(), &rules(), &f);
    assert_eq!(s.category(), Some("SLG"));
    assert!(s.reasons().iter().all(|r| !r.contains("points to")));
}

#[test]
fn hints_alone_suggest_without_confidence() {
    let f = WorkFacts::new(
        vec![],
        vec!["ADV".to_string()],
        vec!["Visual Novel".to_string()],
    );
    let s = CategorySuggestion::derive(&sample(), &rules(), &f);
    assert_eq!(s.category(), Some("AVG"));
    assert_eq!(s.codes(), vec!["AVG"]);
    assert!(!s.confident());
}

#[test]
fn ambiguous_hints_leave_the_category_open() {
    let f = WorkFacts::new(vec![], vec!["SIM+SLG".to_string()], vec![]);
    let s = CategorySuggestion::derive(&sample(), &rules(), &f);
    assert_eq!(s.category(), None);
    assert!(s.reasons().iter().any(|r| r.contains("hints allow")));
}

#[test]
fn new_category_folder_counts_as_a_hint() {
    let f = WorkFacts::new(vec![], vec!["04 角色扮演(RPG)".to_string()], vec![]);
    let s = CategorySuggestion::derive(&sample(), &rules(), &f);
    assert_eq!(s.category(), Some("RPG"));
}

#[test]
fn disagreeing_sources_are_not_confident() {
    let vndb = SourceFacts::new(MetadataProvider::Vndb, Some("ADV".to_string()), vec![]);
    let s = CategorySuggestion::derive(&sample(), &rules(), &facts(vec![dlsite("RPG", &[]), vndb]));
    assert_eq!(s.category(), Some("RPG"));
    assert!(!s.confident());
}

#[test]
fn unknown_work_type_is_reported() {
    let s = CategorySuggestion::derive(&sample(), &rules(), &facts(vec![dlsite("XYZ", &[])]));
    assert_eq!(s.category(), None);
    assert!(
        s.reasons()
            .iter()
            .any(|r| r.contains("no rule for dlsite type XYZ"))
    );
}
