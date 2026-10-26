use super::*;

fn family_of<'a>(v: &'a Vocabulary, code: &str) -> Option<&'a str> {
    v.resolve(code).map(|r| r.base().family())
}

fn code(code: &str, category: &str, family: &str, accepts_prefix: bool) -> GenreCode {
    GenreCode::from_persisted(
        code.to_string(),
        code.to_string(),
        category.to_string(),
        family.to_string(),
        accepts_prefix,
        Vec::new(),
        0,
    )
}

fn category(id: &str, folder: &str) -> Category {
    // "04 角色扮演(RPG)" sorts as 4, like the seeded data.
    Category::new(
        id.to_string(),
        id.to_string(),
        folder.to_string(),
        folder[..2].parse().unwrap(),
    )
}

/// A trimmed-down copy of the seeded vocabulary, enough for unit tests across modules.
pub(crate) fn sample() -> Vocabulary {
    let erg = GenreCode::from_persisted(
        "ERG".into(),
        "ERG".into(),
        "PZL".into(),
        "ERG".into(),
        false,
        vec!["H-ERG".into()],
        0,
    );
    Vocabulary::new(
        vec![
            category("AVG", "01 圖文冒險(AVG)"),
            category("ACT", "02 動作格鬥(ACT)"),
            category("STG", "03 射擊競技(STG)"),
            category("RPG", "04 角色扮演(RPG)"),
            category("SLG", "05 策略模擬(SLG)"),
            category("SIM", "06 經營養成(SIM)"),
            category("PZL", "08 益智解謎(PZL)"),
            category("HAG", "10 動畫支向(HAG)"),
            category("ETC", "99 其他(ETC)"),
        ],
        vec![
            code("ADV", "AVG", "ADV", false),
            code("AVG", "AVG", "AVG", false),
            code("NVL", "AVG", "NVL", false),
            code("ACT", "ACT", "ACT", false),
            code("FTG", "ACT", "FTG", false),
            code("ARPG", "ACT", "ARPG", false),
            code("STG", "STG", "STG", false),
            code("RPG", "RPG", "RPG", false),
            code("SLG", "SLG", "SLG", false),
            code("TD", "SLG", "TD", false),
            code("T-RPG", "SLG", "T-RPG", false),
            code("SRPG", "SLG", "SRPG", false),
            code("SIM", "SIM", "SIM", false),
            code("EDU", "SIM", "EDU", false),
            code("PZL", "PZL", "PZL", false),
            code("TAB", "PZL", "TAB", false),
            code("TPG", "PZL", "TPG", false),
            erg,
            code("HAG", "HAG", "HAG", true),
            code("HAG²", "HAG", "HAG", true),
            code("HAP", "HAG", "HAP", true),
            code("LS", "HAG", "LS", true),
            code("ETC", "ETC", "ETC", false),
        ],
    )
}

#[test]
fn normalizes_case_and_digits() {
    let v = sample();
    assert_eq!(v.normalize("hag").as_deref(), Some("HAG"));
    assert_eq!(v.normalize(" hag2 ").as_deref(), Some("HAG²"));
    assert_eq!(v.normalize("HAG²").as_deref(), Some("HAG²"));
}

#[test]
fn resolves_aliases() {
    let v = sample();
    assert_eq!(v.normalize("h-erg").as_deref(), Some("ERG"));
}

#[test]
fn prefixes_only_apply_to_animation_codes() {
    let v = sample();
    assert_eq!(v.normalize("t-hag").as_deref(), Some("T-HAG"));
    assert_eq!(v.normalize("X-HAG2").as_deref(), Some("X-HAG²"));
    assert_eq!(v.normalize("T-LS").as_deref(), Some("T-LS"));
    assert_eq!(v.normalize("X-RPG"), None);
}

#[test]
fn exact_codes_win_over_prefix_parsing() {
    let v = sample();
    assert_eq!(v.normalize("t-rpg").as_deref(), Some("T-RPG"));
    assert_eq!(v.category_of("T-RPG"), Some("SLG"));
}

#[test]
fn variants_share_family_and_category() {
    let v = sample();
    assert_eq!(family_of(&v, "T-HAG²"), Some("HAG"));
    assert_eq!(v.category_of("X-HAP"), Some("HAG"));
    assert_eq!(family_of(&v, "HAP"), Some("HAP"));
}

#[test]
fn any_code_takes_a_level() {
    let v = sample();
    assert_eq!(v.normalize("rpg3").as_deref(), Some("RPG³"));
    assert_eq!(v.category_of("RPG³"), Some("RPG"));
    assert_eq!(family_of(&v, "RPG³"), Some("RPG"));
    assert_eq!(v.normalize("h-erg4").as_deref(), Some("ERG⁴"));
    assert_eq!(v.normalize("t-rpg2").as_deref(), Some("T-RPG²"));
}

#[test]
fn named_levels_win_over_the_generic_reading() {
    let v = sample();
    let named = v.resolve("HAG²").unwrap();
    assert_eq!((named.base().code(), named.level()), ("HAG²", None));
    let generic = v.resolve("HAG³").unwrap();
    assert_eq!((generic.base().code(), generic.level()), ("HAG", Some(3)));
}

#[test]
fn prefixes_and_levels_combine() {
    let v = sample();
    assert_eq!(v.normalize("t-hap3").as_deref(), Some("T-HAP³"));
    assert_eq!(family_of(&v, "X-HAG⁴"), Some("HAG"));
    assert_eq!(v.normalize("T-EDU2"), None);
}

#[test]
fn rejects_levels_outside_two_to_nine() {
    let v = sample();
    assert_eq!(v.normalize("RPG1"), None);
    assert_eq!(v.normalize("RPG0"), None);
    assert_eq!(v.normalize("RPG23"), None);
    assert_eq!(v.normalize("HAG²3"), None);
}

#[test]
fn rejects_unknown_codes() {
    let v = sample();
    assert_eq!(v.normalize("ファンタジー"), None);
    assert_eq!(v.normalize(""), None);
    assert_eq!(v.normalize("T-"), None);
}

#[test]
fn normalize_all_dedupes_and_reports_first_unknown() {
    let v = sample();
    assert_eq!(
        v.normalize_all(["rpg", "RPG", "edu"]).unwrap(),
        vec!["RPG".to_string(), "EDU".to_string()]
    );
    assert_eq!(
        v.normalize_all(["RPG", "nope"]).unwrap_err(),
        VocabularyError::UnknownCode("nope".into())
    );
}

#[test]
fn classification_adds_the_category_code_when_missing() {
    let v = sample();
    assert_eq!(
        v.prepare_classification(Some("RPG"), ["edu"]).unwrap(),
        vec!["RPG".to_string(), "EDU".to_string()]
    );
}

#[test]
fn classification_keeps_codes_that_already_cover_the_category() {
    let v = sample();
    assert_eq!(
        v.prepare_classification(Some("HAG"), ["t-hag2", "EDU"])
            .unwrap(),
        vec!["T-HAG²".to_string(), "EDU".to_string()]
    );
}

#[test]
fn classification_rejects_unknown_category_or_code() {
    let v = sample();
    assert_eq!(
        v.prepare_classification(Some("NOPE"), ["RPG"]).unwrap_err(),
        VocabularyError::UnknownCategory("NOPE".to_string())
    );
    assert_eq!(
        v.prepare_classification(None, ["??"]).unwrap_err(),
        VocabularyError::UnknownCode("??".to_string())
    );
}

#[test]
fn clearing_the_category_keeps_codes() {
    let v = sample();
    assert_eq!(
        v.prepare_classification(None, ["edu"]).unwrap(),
        vec!["EDU".to_string()]
    );
}

#[test]
fn removal_is_refused_while_anything_depends_on_the_code() {
    let v = sample();
    assert_eq!(
        v.check_removal("RPG", &[], &[]),
        Err(VocabularyError::CategoryOwnCode("RPG".into()))
    );
    assert_eq!(
        v.check_removal("TD", &["X-TD".into(), "TD³".into()], &[]),
        Err(VocabularyError::UsedByWorks("TD".into()))
    );
    assert_eq!(
        v.check_removal("FTG", &[], &["FTG".into()]),
        Err(VocabularyError::UsedByRules("FTG".into()))
    );
    assert_eq!(v.check_removal("SRPG", &["RPG".into()], &[]), Ok(()));
}

#[test]
fn canonical_form_keeps_digit_only_input() {
    assert_eq!(canonical_form("42"), "42");
    assert_eq!(canonical_form("hag4"), "HAG⁴");
}
