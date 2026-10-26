use crate::kernel::{ErrorCode, ErrorKind};

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub(crate) enum KeywordRuleError {
    #[error("unknown genre code: {0}")]
    UnknownCode(String),
}

impl ErrorCode for KeywordRuleError {
    fn code(&self) -> &'static str {
        match self {
            Self::UnknownCode(_) => "keyword_rule.unknown_code",
        }
    }

    fn kind(&self) -> ErrorKind {
        ErrorKind::Invalid
    }
}
