use crate::kernel::{ErrorCode, ErrorKind};

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub(crate) enum WorkTypeRuleError {
    #[error("unknown genre code: {0}")]
    UnknownCode(String),
}

impl ErrorCode for WorkTypeRuleError {
    fn code(&self) -> &'static str {
        match self {
            Self::UnknownCode(_) => "work_type_rule.unknown_code",
        }
    }

    fn kind(&self) -> ErrorKind {
        ErrorKind::Invalid
    }
}
