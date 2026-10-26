//! Adapters onto the external metadata sites. One sub-module per site.

pub(crate) mod dlsite;
pub(crate) mod steam;
pub(crate) mod vndb;

use serde::{Deserialize, Deserializer};

/// Coerce JSON `null` into the default for `T`. `#[serde(default)]` only covers missing keys;
/// DLsite (and occasionally VNDB) sends explicit nulls for absent arrays, which would otherwise
/// fail `Vec` deserialization.
fn null_to_default<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: Default + Deserialize<'de>,
{
    Option::<T>::deserialize(deserializer).map(|o| o.unwrap_or_default())
}

fn unavailable(
    e: impl std::error::Error + Send + Sync + 'static,
) -> crate::domain::metadata_source::MetadataSourceError {
    crate::domain::metadata_source::MetadataSourceError::Unavailable(Box::new(e))
}
