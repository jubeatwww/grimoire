use serde::Deserialize;

/// The word a keyword rule looks for in tags: trimmed, never blank.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(try_from = "String")]
pub(crate) struct Keyword(String);

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("keyword is required")]
pub(crate) struct KeywordRequired;

impl Keyword {
    pub(crate) fn parse(raw: &str) -> Result<Self, KeywordRequired> {
        let keyword = raw.trim();
        if keyword.is_empty() {
            return Err(KeywordRequired);
        }
        Ok(Self(keyword.to_owned()))
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for Keyword {
    type Error = KeywordRequired;

    fn try_from(raw: String) -> Result<Self, Self::Error> {
        Self::parse(&raw)
    }
}
