use crate::kernel::{ErrorCode, ErrorKind};

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub(crate) enum VocabularyError {
    #[error("{0} already means a T-/X- variant of an existing code")]
    PrefixVariantTaken(String),
    #[error("unknown category: {0}")]
    UnknownCategory(String),
    #[error("unknown genre code: {0}")]
    UnknownCode(String),
    #[error("{0} is a category's own code and cannot be removed")]
    CategoryOwnCode(String),
    #[error("{0} is still used by at least one game")]
    UsedByWorks(String),
    #[error("{0} is still used by a classification rule")]
    UsedByRules(String),
}

impl ErrorCode for VocabularyError {
    fn code(&self) -> &'static str {
        match self {
            Self::PrefixVariantTaken(_) => "vocabulary.prefix_variant_taken",
            Self::UnknownCategory(_) => "vocabulary.unknown_category",
            Self::UnknownCode(_) => "vocabulary.unknown_code",
            Self::CategoryOwnCode(_) => "vocabulary.category_own_code",
            Self::UsedByWorks(_) => "vocabulary.used_by_works",
            Self::UsedByRules(_) => "vocabulary.used_by_rules",
        }
    }

    fn kind(&self) -> ErrorKind {
        match self {
            Self::UnknownCategory(_) | Self::UnknownCode(_) => ErrorKind::Invalid,
            Self::PrefixVariantTaken(_)
            | Self::CategoryOwnCode(_)
            | Self::UsedByWorks(_)
            | Self::UsedByRules(_) => ErrorKind::Conflict,
        }
    }
}
