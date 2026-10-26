use crate::kernel::{ErrorCode, ErrorKind};

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub(crate) enum SourceRefetchError {
    #[error("a re-fetch is already running")]
    AlreadyRunning,
}

impl ErrorCode for SourceRefetchError {
    fn code(&self) -> &'static str {
        "source_refetch.already_running"
    }

    fn kind(&self) -> ErrorKind {
        ErrorKind::Conflict
    }
}
