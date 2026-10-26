//! The closed genre-code vocabulary (forum rules) and how user input maps onto it.
//!
//! Codes are stored exactly as the forum writes them (`T-HAG`, `HAG²`). Input is forgiving: case
//! is ignored, trailing ASCII digits become superscripts (`hag2` → `HAG²`) and aliases resolve to
//! their code (`H-ERG` → `ERG`).
//!
//! A stored code is `[prefix]<code>[level]`. Any code takes a ² to ⁹ level (`SSG³`); a level the
//! forum names has its own row (`HAG²` 動漫風3D動畫), which wins over the generic reading. The
//! `T-` / `X-` prefixes only combine with codes that accept them, so `T-RPG` stays its own 策略模擬
//! code instead of becoming "touch RPG".

mod category;
mod category_id;
mod error;
mod genre_code_usage;
mod vocabulary_source;

use crate::domain::genre_code::GenreCode;

pub(crate) use category::Category;
pub(crate) use category_id::CategoryId;
pub(crate) use error::VocabularyError;
pub(crate) use genre_code_usage::{GenreCodeUsage, GenreCodeUsageError};
pub(crate) use vocabulary_source::{VocabularySource, VocabularySourceError};

pub(crate) const VARIANT_PREFIXES: [&str; 2] = ["T-", "X-"];

/// Every category folder and genre code, as stored.
#[derive(Debug, Clone, Default)]
pub(crate) struct Vocabulary {
    categories: Vec<Category>,
    codes: Vec<GenreCode>,
}

/// A stored code split into its optional variant prefix, its base entry and an optional level the
/// base row does not spell out itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ResolvedCode<'a> {
    prefix: Option<&'static str>,
    base: &'a GenreCode,
    level: Option<u8>,
}

impl<'a> ResolvedCode<'a> {
    pub(crate) fn prefix(&self) -> Option<&'static str> {
        self.prefix
    }

    pub(crate) fn base(&self) -> &'a GenreCode {
        self.base
    }

    pub(crate) fn level(&self) -> Option<u8> {
        self.level
    }

    pub(crate) fn canonical(&self) -> String {
        let level = self
            .level
            .and_then(|l| char::from_digit(u32::from(l), 10))
            .map(superscript_digit);
        let mut out = format!("{}{}", self.prefix.unwrap_or(""), self.base.code());
        out.extend(level);
        out
    }
}

impl Vocabulary {
    pub(crate) fn new(categories: Vec<Category>, codes: Vec<GenreCode>) -> Self {
        Self { categories, codes }
    }

    pub(crate) fn categories(&self) -> &[Category] {
        &self.categories
    }

    pub(crate) fn codes(&self) -> &[GenreCode] {
        &self.codes
    }

    pub(crate) fn category(&self, id: &str) -> Option<&Category> {
        self.categories.iter().find(|c| c.id() == id)
    }

    fn exact(&self, key: &str) -> Option<&GenreCode> {
        self.codes.iter().find(|c| {
            c.code().to_uppercase() == key || c.aliases().iter().any(|a| a.to_uppercase() == key)
        })
    }

    /// A code without its prefix: a row of its own, or a row plus a level.
    fn resolve_stem(&self, key: &str) -> Option<(&GenreCode, Option<u8>)> {
        if let Some(base) = self.exact(key) {
            return Some((base, None));
        }
        let (head, level) = split_level(key)?;
        Some((self.exact(head)?, Some(level)))
    }

    /// Resolve user input or a stored code against the vocabulary.
    pub(crate) fn resolve(&self, input: &str) -> Option<ResolvedCode<'_>> {
        let key = canonical_form(input);
        if key.is_empty() {
            return None;
        }
        if let Some((base, level)) = self.resolve_stem(&key) {
            return Some(ResolvedCode {
                prefix: None,
                base,
                level,
            });
        }
        for prefix in VARIANT_PREFIXES {
            if let Some(rest) = key.strip_prefix(prefix) {
                let (base, level) = self.resolve_stem(rest)?;
                return base.accepts_prefix().then_some(ResolvedCode {
                    prefix: Some(prefix),
                    base,
                    level,
                });
            }
        }
        None
    }

    /// The canonical stored spelling for `input`, or `None` when it is not in the vocabulary.
    pub(crate) fn normalize(&self, input: &str) -> Option<String> {
        self.resolve(input).map(|r| r.canonical())
    }

    pub(crate) fn category_of(&self, code: &str) -> Option<&str> {
        self.resolve(code).map(|r| r.base.category_id())
    }

    /// Normalise a whole facet list: canonical spellings, duplicates dropped, original order
    /// kept. Fails on the first input that is not a known code.
    pub(crate) fn normalize_all<'a>(
        &self,
        inputs: impl IntoIterator<Item = &'a str>,
    ) -> Result<Vec<String>, VocabularyError> {
        let mut out: Vec<String> = Vec::new();
        for input in inputs {
            let code = self
                .normalize(input)
                .ok_or_else(|| VocabularyError::UnknownCode(input.to_string()))?;
            if !out.contains(&code) {
                out.push(code);
            }
        }
        Ok(out)
    }

    /// Validate and normalise a classification before it is stored.
    ///
    /// Codes are normalised and deduplicated. When a category is set and none of the codes
    /// belongs to it, the category id (always also a code of that category) is put first so
    /// every confirmed work carries its own code.
    pub(crate) fn prepare_classification<'a>(
        &self,
        category_id: Option<&str>,
        facets: impl IntoIterator<Item = &'a str>,
    ) -> Result<Vec<String>, VocabularyError> {
        let mut codes = self.normalize_all(facets)?;
        let Some(category_id) = category_id else {
            return Ok(codes);
        };
        if self.category(category_id).is_none() {
            return Err(VocabularyError::UnknownCategory(category_id.to_string()));
        }
        let has_own_code = codes
            .iter()
            .any(|c| self.category_of(c) == Some(category_id));
        if !has_own_code {
            let own = self
                .normalize(category_id)
                .ok_or_else(|| VocabularyError::UnknownCode(category_id.to_string()))?;
            codes.insert(0, own);
        }
        Ok(codes)
    }

    /// Refuses to remove a code that a category, a work or a rule still depends on. Stored codes
    /// may carry a prefix or a level, so they are matched through the vocabulary.
    pub(crate) fn check_removal(
        &self,
        code: &str,
        codes_in_works: &[String],
        codes_in_rules: &[String],
    ) -> Result<(), VocabularyError> {
        if self.category(code).is_some() {
            return Err(VocabularyError::CategoryOwnCode(code.to_owned()));
        }
        let uses = |stored: &[String]| {
            stored
                .iter()
                .any(|s| self.resolve(s).is_some_and(|r| r.base.code() == code))
        };
        if uses(codes_in_works) {
            return Err(VocabularyError::UsedByWorks(code.to_owned()));
        }
        if uses(codes_in_rules) {
            return Err(VocabularyError::UsedByRules(code.to_owned()));
        }
        Ok(())
    }

    /// Where a new code goes in its category's list: after the last one.
    pub(crate) fn next_sort_order(&self, category_id: &str) -> i32 {
        self.codes
            .iter()
            .filter(|c| c.category_id() == category_id)
            .map(|c| c.sort_order() + 1)
            .max()
            .unwrap_or(0)
    }

    pub(crate) fn code(&self, code: &str) -> Option<&GenreCode> {
        self.codes.iter().find(|c| c.code() == code)
    }
}

/// Uppercase, trim, and turn trailing ASCII digits into superscripts so `hag2` and `HAG²`
/// compare equal.
pub(crate) fn canonical_form(input: &str) -> String {
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

/// Split a single trailing ² to ⁹ off a code: `SSG³` → (`SSG`, 3).
fn split_level(key: &str) -> Option<(&str, u8)> {
    let last = key.chars().next_back()?;
    let level = match last {
        '²' => 2,
        '³' => 3,
        '⁴' => 4,
        '⁵' => 5,
        '⁶' => 6,
        '⁷' => 7,
        '⁸' => 8,
        '⁹' => 9,
        _ => return None,
    };
    let head = &key[..key.len() - last.len_utf8()];
    let head_ends_in_level = head
        .chars()
        .next_back()
        .is_some_and(|c| "⁰¹²³⁴⁵⁶⁷⁸⁹".contains(c));
    (!head.is_empty() && !head_ends_in_level).then_some((head, level))
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
pub(crate) mod tests;
