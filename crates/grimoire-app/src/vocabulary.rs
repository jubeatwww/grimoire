//! The closed genre-code vocabulary (forum rules) and how user input maps
//! onto it.
//!
//! Codes are stored exactly as the forum writes them (`T-HAG`, `HAG²`). Input
//! is forgiving: case is ignored, trailing ASCII digits become superscripts
//! (`hag2` → `HAG²`) and aliases resolve to their code (`H-ERG` → `ERG`). The
//! `T-` / `X-` prefixes only combine with codes that accept them, so `T-RPG`
//! stays its own 策略模擬 code instead of becoming "touch RPG".

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CategoryDef {
    pub id: String,
    pub label: String,
    pub folder_name: String,
    pub sort_order: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CodeDef {
    pub code: String,
    pub label: String,
    pub category_id: String,
    pub family: String,
    pub accepts_prefix: bool,
    pub aliases: Vec<String>,
    pub sort_order: i32,
}

pub const VARIANT_PREFIXES: [&str; 2] = ["T-", "X-"];

#[derive(Debug, Clone, Default)]
pub struct Vocabulary {
    categories: Vec<CategoryDef>,
    codes: Vec<CodeDef>,
}

/// A stored code split into its optional variant prefix and its base entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResolvedCode<'a> {
    pub prefix: Option<&'static str>,
    pub base: &'a CodeDef,
}

impl ResolvedCode<'_> {
    pub fn canonical(&self) -> String {
        format!("{}{}", self.prefix.unwrap_or(""), self.base.code)
    }
}

impl Vocabulary {
    pub fn new(categories: Vec<CategoryDef>, codes: Vec<CodeDef>) -> Self {
        Self { categories, codes }
    }

    pub fn categories(&self) -> &[CategoryDef] {
        &self.categories
    }

    pub fn codes(&self) -> &[CodeDef] {
        &self.codes
    }

    pub fn category(&self, id: &str) -> Option<&CategoryDef> {
        self.categories.iter().find(|c| c.id == id)
    }

    fn exact(&self, key: &str) -> Option<&CodeDef> {
        self.codes.iter().find(|c| {
            c.code.to_uppercase() == key || c.aliases.iter().any(|a| a.to_uppercase() == key)
        })
    }

    /// Resolve user input or a stored code against the vocabulary.
    pub fn resolve(&self, input: &str) -> Option<ResolvedCode<'_>> {
        let key = canonical_form(input);
        if key.is_empty() {
            return None;
        }
        if let Some(base) = self.exact(&key) {
            return Some(ResolvedCode { prefix: None, base });
        }
        for prefix in VARIANT_PREFIXES {
            if let Some(rest) = key.strip_prefix(prefix) {
                let base = self
                    .codes
                    .iter()
                    .find(|c| c.accepts_prefix && c.code.to_uppercase() == rest)?;
                return Some(ResolvedCode {
                    prefix: Some(prefix),
                    base,
                });
            }
        }
        None
    }

    /// The canonical stored spelling for `input`, or `None` when it is not in
    /// the vocabulary.
    pub fn normalize(&self, input: &str) -> Option<String> {
        self.resolve(input).map(|r| r.canonical())
    }

    pub fn category_of(&self, code: &str) -> Option<&str> {
        self.resolve(code).map(|r| r.base.category_id.as_str())
    }

    pub fn family_of(&self, code: &str) -> Option<&str> {
        self.resolve(code).map(|r| r.base.family.as_str())
    }

    /// Normalise a whole facet list: canonical spellings, duplicates dropped,
    /// original order kept. Returns the first input that is not a known code.
    pub fn normalize_all<'a>(
        &self,
        inputs: impl IntoIterator<Item = &'a str>,
    ) -> Result<Vec<String>, String> {
        let mut out: Vec<String> = Vec::new();
        for input in inputs {
            let code = self.normalize(input).ok_or_else(|| input.to_string())?;
            if !out.contains(&code) {
                out.push(code);
            }
        }
        Ok(out)
    }

    /// Validate and normalise a classification before it is stored.
    ///
    /// Codes are normalised and deduplicated. When a category is set and none
    /// of the codes belongs to it, the category id (always also a code of that
    /// category) is put first so every confirmed work carries its own code.
    pub fn prepare_classification<'a>(
        &self,
        category_id: Option<&str>,
        facets: impl IntoIterator<Item = &'a str>,
    ) -> Result<Vec<String>, ClassificationError> {
        let mut codes = self
            .normalize_all(facets)
            .map_err(ClassificationError::UnknownCode)?;
        let Some(category_id) = category_id else {
            return Ok(codes);
        };
        if self.category(category_id).is_none() {
            return Err(ClassificationError::UnknownCategory(category_id.to_string()));
        }
        let has_own_code = codes
            .iter()
            .any(|c| self.category_of(c) == Some(category_id));
        if !has_own_code {
            let own = self
                .normalize(category_id)
                .ok_or_else(|| ClassificationError::UnknownCode(category_id.to_string()))?;
            codes.insert(0, own);
        }
        Ok(codes)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClassificationError {
    UnknownCategory(String),
    UnknownCode(String),
}

impl std::fmt::Display for ClassificationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownCategory(c) => write!(f, "unknown category: {c}"),
            Self::UnknownCode(c) => write!(f, "unknown genre code: {c}"),
        }
    }
}

/// Uppercase, trim, and turn trailing ASCII digits into superscripts so
/// `hag2` and `HAG²` compare equal.
pub fn canonical_form(input: &str) -> String {
    let upper = input.trim().to_uppercase();
    let split = upper
        .char_indices()
        .rev()
        .take_while(|(_, ch)| ch.is_ascii_digit())
        .last()
        .map(|(i, _)| i);
    match split {
        // Only a suffix: a code made of digits alone stays as typed.
        Some(i) if i > 0 => {
            let (head, digits) = upper.split_at(i);
            let sup: String = digits.chars().map(superscript_digit).collect();
            format!("{head}{sup}")
        }
        _ => upper,
    }
}

fn superscript_digit(ch: char) -> char {
    match ch {
        '0' => '⁰',
        '1' => '¹',
        '2' => '²',
        '3' => '³',
        '4' => '⁴',
        '5' => '⁵',
        '6' => '⁶',
        '7' => '⁷',
        '8' => '⁸',
        '9' => '⁹',
        other => other,
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    fn code(code: &str, category: &str, family: &str, accepts_prefix: bool) -> CodeDef {
        CodeDef {
            code: code.to_string(),
            label: code.to_string(),
            category_id: category.to_string(),
            family: family.to_string(),
            accepts_prefix,
            aliases: Vec::new(),
            sort_order: 0,
        }
    }

    fn category(id: &str, folder: &str) -> CategoryDef {
        CategoryDef {
            id: id.to_string(),
            label: id.to_string(),
            folder_name: folder.to_string(),
            // "04 角色扮演(RPG)" sorts as 4, like the seeded data.
            sort_order: folder[..2].parse().unwrap(),
        }
    }

    /// A trimmed-down copy of the seeded vocabulary, enough for unit tests
    /// across modules.
    pub(crate) fn sample() -> Vocabulary {
        let mut erg = code("ERG", "PZL", "ERG", false);
        erg.aliases.push("H-ERG".to_string());
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
        assert_eq!(v.family_of("T-HAG²"), Some("HAG"));
        assert_eq!(v.category_of("X-HAP"), Some("HAG"));
        assert_eq!(v.family_of("HAP"), Some("HAP"));
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
        assert_eq!(v.normalize_all(["RPG", "nope"]).unwrap_err(), "nope");
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
            v.prepare_classification(Some("HAG"), ["t-hag2", "EDU"]).unwrap(),
            vec!["T-HAG²".to_string(), "EDU".to_string()]
        );
    }

    #[test]
    fn classification_rejects_unknown_category_or_code() {
        let v = sample();
        assert_eq!(
            v.prepare_classification(Some("NOPE"), ["RPG"]).unwrap_err(),
            ClassificationError::UnknownCategory("NOPE".to_string())
        );
        assert_eq!(
            v.prepare_classification(None, ["??"]).unwrap_err(),
            ClassificationError::UnknownCode("??".to_string())
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
    fn canonical_form_keeps_digit_only_input() {
        assert_eq!(canonical_form("42"), "42");
        assert_eq!(canonical_form("hag4"), "HAG⁴");
    }
}
