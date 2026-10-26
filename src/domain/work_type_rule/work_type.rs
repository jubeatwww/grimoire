use serde::Deserialize;

/// A source's own work type code (`RPG`, `SLN`): trimmed, upper-case, never blank.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(try_from = "String")]
pub(crate) struct WorkType(String);

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("work type is required")]
pub(crate) struct WorkTypeRequired;

impl WorkType {
    pub(crate) fn parse(raw: &str) -> Result<Self, WorkTypeRequired> {
        let work_type = raw.trim().to_uppercase();
        if work_type.is_empty() {
            return Err(WorkTypeRequired);
        }
        Ok(Self(work_type))
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for WorkType {
    type Error = WorkTypeRequired;

    fn try_from(raw: String) -> Result<Self, Self::Error> {
        Self::parse(&raw)
    }
}
