//! A rule from a word in a source's tags to a genre code: `タワーディフェンス` → `TD`.

mod error;
mod events;
mod keyword;
mod keyword_rule_repository;

use crate::domain::metadata_source::MetadataProvider;
use crate::domain::vocabulary::Vocabulary;
use crate::kernel::Id;

pub(crate) use error::KeywordRuleError;
pub(crate) use events::KeywordRuleDefined;
pub(crate) use keyword::Keyword;
pub(crate) use keyword_rule_repository::{KeywordRuleRepository, KeywordRuleRepositoryError};

pub(crate) type KeywordRuleId = Id<KeywordRule>;

/// Matches any tag containing the keyword, case-insensitively. Without a source it applies to
/// every source. `contests_type` marks a keyword that, landing in another category than the
/// source's work type, makes a suggestion not confident (アクションRPG on a DLsite RPG).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct KeywordRule {
    id: KeywordRuleId,
    source: Option<MetadataProvider>,
    keyword: String,
    code: String,
    contests_type: bool,
}

impl KeywordRule {
    pub(crate) fn define(
        vocabulary: &Vocabulary,
        id: KeywordRuleId,
        source: Option<MetadataProvider>,
        keyword: &Keyword,
        code: &str,
        contests_type: bool,
    ) -> Result<(Self, KeywordRuleDefined), KeywordRuleError> {
        let keyword = keyword.as_str().to_owned();
        let code = vocabulary
            .normalize(code)
            .ok_or_else(|| KeywordRuleError::UnknownCode(code.to_owned()))?;
        let defined = KeywordRuleDefined {
            rule_id: id,
            keyword: keyword.clone(),
        };
        Ok((
            Self {
                id,
                source,
                keyword,
                code,
                contests_type,
            },
            defined,
        ))
    }

    /// An edit from Settings: every field is replaced, the identity stays.
    pub(crate) fn redefine(
        &mut self,
        vocabulary: &Vocabulary,
        source: Option<MetadataProvider>,
        keyword: &Keyword,
        code: &str,
        contests_type: bool,
    ) -> Result<KeywordRuleDefined, KeywordRuleError> {
        let (rule, defined) =
            Self::define(vocabulary, self.id, source, keyword, code, contests_type)?;
        *self = rule;
        Ok(defined)
    }

    /// Rebuilds a stored rule. Repositories only.
    pub(crate) fn from_persisted(
        id: KeywordRuleId,
        source: Option<MetadataProvider>,
        keyword: String,
        code: String,
        contests_type: bool,
    ) -> Self {
        Self {
            id,
            source,
            keyword,
            code,
            contests_type,
        }
    }

    pub(crate) fn applies_to(&self, source: MetadataProvider) -> bool {
        self.source.is_none_or(|s| s == source)
    }

    /// Whether `tag` contains the keyword, ignoring case.
    pub(crate) fn matches_tag(&self, tag: &str) -> bool {
        let needle = self.keyword.to_lowercase();
        !needle.is_empty() && tag.to_lowercase().contains(&needle)
    }

    pub(crate) fn id(&self) -> KeywordRuleId {
        self.id
    }

    pub(crate) fn source(&self) -> Option<MetadataProvider> {
        self.source
    }

    pub(crate) fn keyword(&self) -> &str {
        &self.keyword
    }

    pub(crate) fn code(&self) -> &str {
        &self.code
    }

    pub(crate) fn contests_type(&self) -> bool {
        self.contests_type
    }
}
