use serde::Serialize;

use crate::domain::source_refetch::SourceRefetch;
use crate::kernel::Timestamp;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SourceRefetchOutput {
    running: bool,
    total: usize,
    done: usize,
    failed: usize,
    started_at: Option<Timestamp>,
    finished_at: Option<Timestamp>,
    last_error: Option<String>,
}

impl From<&SourceRefetch> for SourceRefetchOutput {
    fn from(refetch: &SourceRefetch) -> Self {
        Self {
            running: refetch.running(),
            total: refetch.total(),
            done: refetch.done(),
            failed: refetch.failed(),
            started_at: refetch.started_at(),
            finished_at: refetch.finished_at(),
            last_error: refetch.last_error().map(str::to_owned),
        }
    }
}
