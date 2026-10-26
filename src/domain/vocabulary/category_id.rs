use serde::{Deserialize, Deserializer};

/// A primary category as chosen in the UI (`RPG`): trimmed. Whether the category exists is the
/// vocabulary's call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CategoryId(String);

impl CategoryId {
    /// `None` for a blank choice, which clears the category.
    pub(crate) fn parse(raw: &str) -> Option<Self> {
        let id = raw.trim();
        (!id.is_empty()).then(|| Self(id.to_owned()))
    }

    /// For `#[serde(deserialize_with)]`: a missing, null or blank field is no category.
    pub(crate) fn deserialize_optional<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Option<Self>, D::Error> {
        Ok(Option::<String>::deserialize(deserializer)?.and_then(|raw| Self::parse(&raw)))
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}
