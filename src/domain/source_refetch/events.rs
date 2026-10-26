use serde::Serialize;

use crate::kernel::Timestamp;

#[must_use = "publish the event or discard it explicitly"]
#[derive(Debug, Clone, Serialize)]
pub(crate) struct SourceRefetchStarted {
    pub(crate) total: usize,
    pub(crate) at: Timestamp,
}

#[must_use = "publish the event or discard it explicitly"]
#[derive(Debug, Clone, Serialize)]
pub(crate) struct SourceRefetchProgressed {
    pub(crate) done: usize,
    pub(crate) failed: usize,
}

#[must_use = "publish the event or discard it explicitly"]
#[derive(Debug, Clone, Serialize)]
pub(crate) struct SourceRefetchFinished {
    pub(crate) at: Timestamp,
}
