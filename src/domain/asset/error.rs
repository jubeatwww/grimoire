use crate::kernel::{ErrorCode, ErrorKind};

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub(crate) enum AssetError {
    #[error("only jpg, png, gif, webp and avif images can be uploaded")]
    UnsupportedType,
    #[error("the image is larger than 20 MiB")]
    TooLarge,
    #[error("not an asset name: {0}")]
    InvalidName(String),
}

impl ErrorCode for AssetError {
    fn code(&self) -> &'static str {
        match self {
            Self::UnsupportedType => "asset.unsupported_type",
            Self::TooLarge => "asset.too_large",
            Self::InvalidName(_) => "asset.invalid_name",
        }
    }

    fn kind(&self) -> ErrorKind {
        ErrorKind::Invalid
    }
}
