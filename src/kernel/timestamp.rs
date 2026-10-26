use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// An instant in UTC. Domain code never reads the clock; it receives a `Timestamp`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub(crate) struct Timestamp(DateTime<Utc>);

impl Timestamp {
    pub(crate) const fn from_utc(at: DateTime<Utc>) -> Self {
        Self(at)
    }

    pub(crate) const fn as_utc(self) -> DateTime<Utc> {
        self.0
    }
}
