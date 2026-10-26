//! A rule from a source's own work type to a genre code: DLsite `RPG` → `RPG`.

mod error;
mod events;
mod work_type;
mod work_type_rule_repository;

use crate::domain::metadata_source::MetadataProvider;
use crate::domain::vocabulary::Vocabulary;

pub(crate) use error::WorkTypeRuleError;
pub(crate) use events::WorkTypeRuleDefined;
pub(crate) use work_type::WorkType;
pub(crate) use work_type_rule_repository::{WorkTypeRuleRepository, WorkTypeRuleRepositoryError};

/// Identified by its source and work type. Without a code the work's tags decide; such a rule
/// can never be confident.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WorkTypeRule {
    source: MetadataProvider,
    work_type: String,
    code: Option<String>,
    confident: bool,
}

impl WorkTypeRule {
    /// Adds a rule or replaces the one for the same source and work type. The work type is
    /// stored upper-case; the code in canonical form.
    pub(crate) fn define(
        vocabulary: &Vocabulary,
        source: MetadataProvider,
        work_type: &WorkType,
        code: Option<&str>,
        confident: bool,
    ) -> Result<(Self, WorkTypeRuleDefined), WorkTypeRuleError> {
        let work_type = work_type.as_str().to_owned();
        let code = match code.map(str::trim).filter(|c| !c.is_empty()) {
            Some(c) => Some(
                vocabulary
                    .normalize(c)
                    .ok_or_else(|| WorkTypeRuleError::UnknownCode(c.to_owned()))?,
            ),
            None => None,
        };
        let confident = confident && code.is_some();
        let defined = WorkTypeRuleDefined {
            source,
            work_type: work_type.clone(),
        };
        Ok((
            Self {
                source,
                work_type,
                code,
                confident,
            },
            defined,
        ))
    }

    /// Rebuilds a stored rule. Repositories only.
    pub(crate) fn from_persisted(
        source: MetadataProvider,
        work_type: String,
        code: Option<String>,
        confident: bool,
    ) -> Self {
        Self {
            source,
            work_type,
            code,
            confident,
        }
    }

    pub(crate) fn source(&self) -> MetadataProvider {
        self.source
    }

    pub(crate) fn work_type(&self) -> &str {
        &self.work_type
    }

    pub(crate) fn code(&self) -> Option<&str> {
        self.code.as_deref()
    }

    pub(crate) fn confident(&self) -> bool {
        self.confident
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::vocabulary::tests::sample;

    fn work_type(raw: &str) -> WorkType {
        WorkType::parse(raw).unwrap()
    }

    #[test]
    fn a_blank_work_type_is_refused() {
        assert_eq!(WorkType::parse(" "), Err(work_type::WorkTypeRequired));
    }

    #[test]
    fn a_rule_without_a_code_is_never_confident() {
        let (rule, _defined) = WorkTypeRule::define(
            &sample(),
            MetadataProvider::Dlsite,
            &work_type(" sln "),
            None,
            true,
        )
        .unwrap();
        assert_eq!(rule.work_type(), "SLN");
        assert!(!rule.confident());
    }

    #[test]
    fn the_code_is_stored_canonical() {
        let (rule, _defined) = WorkTypeRule::define(
            &sample(),
            MetadataProvider::Dlsite,
            &work_type("RPG"),
            Some("rpg"),
            true,
        )
        .unwrap();
        assert_eq!(rule.code(), Some("RPG"));
        assert!(rule.confident());
        assert_eq!(
            WorkTypeRule::define(
                &sample(),
                MetadataProvider::Dlsite,
                &work_type("X"),
                Some("nope"),
                true
            )
            .unwrap_err(),
            WorkTypeRuleError::UnknownCode("nope".into())
        );
    }
}
