use serde::Serialize;

/// The stored image's name in the asset cache.
#[derive(Debug, Serialize)]
pub(crate) struct UploadAssetOutput {
    pub(crate) name: String,
}

/// An image's bytes and how to serve them.
#[derive(Debug)]
pub(crate) struct AssetOutput {
    pub(crate) bytes: Vec<u8>,
    pub(crate) media_type: &'static str,
}
