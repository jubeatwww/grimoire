//! One forum genre code (`RPG`, `HAG²`, `T-RPG`), editable in Settings.

mod code_spelling;
mod events;
mod genre_code_repository;

use crate::domain::vocabulary::{Vocabulary, VocabularyError, canonical_form};

pub(crate) use code_spelling::{CodeLabel, CodeSpelling};
pub(crate) use events::GenreCodeDefined;
pub(crate) use genre_code_repository::{GenreCodeRepository, GenreCodeRepositoryError};

/// Identified by its code. `family` groups variants for search (HAG² belongs to HAG);
/// `accepts_prefix` allows the T- (touch) and X- (partial / cross-section) prefixes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GenreCode {
    code: String,
    label: String,
    category_id: String,
    family: String,
    accepts_prefix: bool,
    aliases: Vec<String>,
    sort_order: i32,
}

/// What the user typed to add or edit a code.
#[derive(Debug, Clone)]
pub(crate) struct CodeDefinition {
    code: CodeSpelling,
    label: CodeLabel,
    category_id: String,
    family: Option<String>,
    accepts_prefix: bool,
    aliases: Vec<String>,
}

impl CodeDefinition {
    pub(crate) fn new(code: CodeSpelling, label: CodeLabel, category_id: String) -> Self {
        Self {
            code,
            label,
            category_id,
            family: None,
            accepts_prefix: false,
            aliases: Vec::new(),
        }
    }

    pub(crate) fn with_family(mut self, family: Option<String>) -> Self {
        self.family = family;
        self
    }

    pub(crate) fn with_prefixes(mut self, accepts_prefix: bool) -> Self {
        self.accepts_prefix = accepts_prefix;
        self
    }

    pub(crate) fn with_aliases(mut self, aliases: Vec<String>) -> Self {
        self.aliases = aliases;
        self
    }
}

impl GenreCode {
    /// Adds a code or redefines an existing one. The code is stored in canonical form, may not
    /// spell a T- / X- variant of another code, and a named level (`SSG³`) joins the family of
    /// the code it levels unless a family is given. A new code goes last in its category.
    pub(crate) fn define(
        vocabulary: &Vocabulary,
        definition: CodeDefinition,
    ) -> Result<(Self, GenreCodeDefined), VocabularyError> {
        let code = definition.code.as_str().to_owned();
        let label = definition.label.as_str().to_owned();
        let existing = vocabulary.resolve(&code);
        if existing.is_some_and(|r| r.prefix().is_some()) {
            return Err(VocabularyError::PrefixVariantTaken(code));
        }
        if vocabulary.category(&definition.category_id).is_none() {
            return Err(VocabularyError::UnknownCategory(definition.category_id));
        }
        let default_family = existing
            .filter(|r| r.level().is_some())
            .map_or_else(|| code.clone(), |r| r.base().family().to_owned());
        let family = definition
            .family
            .map(|f| canonical_form(&f))
            .filter(|f| !f.is_empty())
            .unwrap_or(default_family);
        let aliases: Vec<String> = definition
            .aliases
            .iter()
            .map(|a| canonical_form(a))
            .filter(|a| !a.is_empty() && *a != code)
            .collect();
        let sort_order = vocabulary.code(&code).map_or_else(
            || vocabulary.next_sort_order(&definition.category_id),
            GenreCode::sort_order,
        );
        let defined = GenreCodeDefined {
            code: code.clone(),
            category_id: definition.category_id.clone(),
        };
        let genre_code = Self {
            code,
            label,
            category_id: definition.category_id,
            family,
            accepts_prefix: definition.accepts_prefix,
            aliases,
            sort_order,
        };
        Ok((genre_code, defined))
    }

    /// Rebuilds a stored code. Repositories only.
    pub(crate) fn from_persisted(
        code: String,
        label: String,
        category_id: String,
        family: String,
        accepts_prefix: bool,
        aliases: Vec<String>,
        sort_order: i32,
    ) -> Self {
        Self {
            code,
            label,
            category_id,
            family,
            accepts_prefix,
            aliases,
            sort_order,
        }
    }

    pub(crate) fn code(&self) -> &str {
        &self.code
    }

    pub(crate) fn label(&self) -> &str {
        &self.label
    }

    pub(crate) fn category_id(&self) -> &str {
        &self.category_id
    }

    pub(crate) fn family(&self) -> &str {
        &self.family
    }

    pub(crate) fn accepts_prefix(&self) -> bool {
        self.accepts_prefix
    }

    pub(crate) fn aliases(&self) -> &[String] {
        &self.aliases
    }

    pub(crate) fn sort_order(&self) -> i32 {
        self.sort_order
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::vocabulary::tests::sample;

    fn definition(code: &str, label: &str, category: String) -> CodeDefinition {
        CodeDefinition::new(
            CodeSpelling::parse(code).unwrap(),
            CodeLabel::parse(label).unwrap(),
            category,
        )
    }

    #[test]
    fn a_named_level_joins_its_base_family_and_goes_last() {
        let (code, _defined) =
            GenreCode::define(&sample(), definition("ssg3", " 寫實潛行 ", "RPG".into())).unwrap();
        assert_eq!(code.code(), "SSG³");
        assert_eq!(
            code.family(),
            "SSG³",
            "SSG is not in the sample, so nothing to join"
        );
        let (level, _defined) =
            GenreCode::define(&sample(), definition("rpg3", "寫實", "RPG".into())).unwrap();
        assert_eq!(level.family(), "RPG");
        assert_eq!(level.label(), "寫實");
        assert_eq!(level.sort_order(), 1);
    }

    #[test]
    fn refuses_a_variant_spelling_and_an_unknown_category() {
        let v = sample();
        assert_eq!(
            GenreCode::define(&v, definition("t-hag", "x", "HAG".into())).unwrap_err(),
            VocabularyError::PrefixVariantTaken("T-HAG".into())
        );
        assert_eq!(
            GenreCode::define(&v, definition("NEW", "x", "NOPE".into())).unwrap_err(),
            VocabularyError::UnknownCategory("NOPE".into())
        );
        assert_eq!(
            CodeSpelling::parse(" "),
            Err(code_spelling::CodeAndLabelRequired)
        );
        assert_eq!(
            CodeLabel::parse(" "),
            Err(code_spelling::CodeAndLabelRequired)
        );
    }

    #[test]
    fn aliases_are_canonical_and_never_the_code_itself() {
        let (code, _defined) = GenreCode::define(
            &sample(),
            definition("NEW", "x", "PZL".into()).with_aliases(vec![
                "new".into(),
                "old2".into(),
                " ".into(),
            ]),
        )
        .unwrap();
        assert_eq!(code.aliases(), ["OLD²"]);
    }
}
