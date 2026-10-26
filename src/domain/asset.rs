//! Images kept in the asset cache: uploads from the item panel, served back by name.

mod asset_store;
mod error;

use std::path::Path;

use crate::kernel::Id;

pub(crate) use asset_store::{AssetStore, AssetStoreError};
pub(crate) use error::AssetError;

/// Upload size cap.
pub(crate) const MAX_ASSET_BYTES: usize = 20 * 1024 * 1024;
const IMAGE_EXTENSIONS: &[&str] = &["jpg", "jpeg", "png", "gif", "webp", "avif"];

pub(crate) type AssetId = Id<Asset>;

/// One image in the cache, addressed by its file name. A name never leaves the cache folder.
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
#[serde(try_from = "String")]
pub(crate) struct Asset {
    name: String,
}

impl Asset {
    /// An uploaded image, stored under its new id and its original extension.
    pub(crate) fn accept_upload(
        id: AssetId,
        original_name: &str,
        size: usize,
    ) -> Result<Self, AssetError> {
        let extension = Path::new(original_name)
            .extension()
            .and_then(|s| s.to_str())
            .map(str::to_ascii_lowercase)
            .filter(|e| IMAGE_EXTENSIONS.contains(&e.as_str()))
            .ok_or(AssetError::UnsupportedType)?;
        if size > MAX_ASSET_BYTES {
            return Err(AssetError::TooLarge);
        }
        Ok(Self {
            name: format!("{id}.{extension}"),
        })
    }

    /// A requested name; anything that could walk out of the cache is refused.
    pub(crate) fn named(name: &str) -> Result<Self, AssetError> {
        if name.is_empty() || name.contains('/') || name.contains('\\') || name.contains("..") {
            return Err(AssetError::InvalidName(name.to_owned()));
        }
        Ok(Self {
            name: name.to_owned(),
        })
    }

    pub(crate) fn name(&self) -> &str {
        &self.name
    }

    pub(crate) fn media_type(&self) -> &'static str {
        match Path::new(&self.name)
            .extension()
            .and_then(|e| e.to_str())
            .map(str::to_ascii_lowercase)
            .as_deref()
        {
            Some("jpg" | "jpeg") => "image/jpeg",
            Some("png") => "image/png",
            Some("gif") => "image/gif",
            Some("webp") => "image/webp",
            Some("avif") => "image/avif",
            _ => "application/octet-stream",
        }
    }
}

impl TryFrom<String> for Asset {
    type Error = AssetError;

    fn try_from(name: String) -> Result<Self, Self::Error> {
        Self::named(&name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uploads_keep_a_lowercase_image_extension() {
        let id = AssetId::from_raw(uuid::Uuid::from_u128(7));
        let asset = Asset::accept_upload(id, "Cover.PNG", 10).unwrap();
        assert_eq!(asset.name(), format!("{id}.png"));
        assert_eq!(asset.media_type(), "image/png");
        assert_eq!(
            Asset::accept_upload(id, "notes.txt", 10),
            Err(AssetError::UnsupportedType)
        );
        assert_eq!(
            Asset::accept_upload(id, "big.jpg", MAX_ASSET_BYTES + 1),
            Err(AssetError::TooLarge)
        );
    }

    #[test]
    fn names_cannot_leave_the_cache() {
        assert!(Asset::named("../secret").is_err());
        assert!(Asset::named("a/b.png").is_err());
        assert!(Asset::named("").is_err());
        assert!(Asset::named("x.webp").is_ok());
    }
}
