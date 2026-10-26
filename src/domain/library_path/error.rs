#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub(crate) enum LibraryPathError {
    #[error("relative path cannot be empty")]
    Empty,
    #[error("path traversal is not allowed")]
    Traversal,
    #[error("absolute client paths are not allowed")]
    Absolute,
}

impl crate::kernel::ErrorCode for LibraryPathError {
    fn code(&self) -> &'static str {
        match self {
            Self::Empty => "library_path.empty",
            Self::Traversal => "library_path.traversal",
            Self::Absolute => "library_path.absolute",
        }
    }

    fn kind(&self) -> crate::kernel::ErrorKind {
        crate::kernel::ErrorKind::Invalid
    }
}
