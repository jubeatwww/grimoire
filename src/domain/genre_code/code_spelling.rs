use serde::Deserialize;

use crate::domain::vocabulary::canonical_form;

/// A genre code as typed, in canonical form (`hag2` → `HAG²`): never blank.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(try_from = "String")]
pub(crate) struct CodeSpelling(String);

/// A genre code's label (`動畫遊戲`): trimmed, never blank.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(try_from = "String")]
pub(crate) struct CodeLabel(String);

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("code and label are required")]
pub(crate) struct CodeAndLabelRequired;

impl CodeSpelling {
    pub(crate) fn parse(raw: &str) -> Result<Self, CodeAndLabelRequired> {
        let code = canonical_form(raw);
        if code.is_empty() {
            return Err(CodeAndLabelRequired);
        }
        Ok(Self(code))
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for CodeSpelling {
    type Error = CodeAndLabelRequired;

    fn try_from(raw: String) -> Result<Self, Self::Error> {
        Self::parse(&raw)
    }
}

impl CodeLabel {
    pub(crate) fn parse(raw: &str) -> Result<Self, CodeAndLabelRequired> {
        let label = raw.trim();
        if label.is_empty() {
            return Err(CodeAndLabelRequired);
        }
        Ok(Self(label.to_owned()))
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for CodeLabel {
    type Error = CodeAndLabelRequired;

    fn try_from(raw: String) -> Result<Self, Self::Error> {
        Self::parse(&raw)
    }
}
