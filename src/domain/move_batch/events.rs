use serde::Serialize;

use super::{MoveBatchId, MoveCounts, MoveOpStatus};
use crate::domain::library_path::LibraryPath;
use crate::kernel::Timestamp;

#[must_use = "publish the event or discard it explicitly"]
#[derive(Debug, Clone, Serialize)]
pub(crate) struct MoveBatchStarted {
    pub(crate) batch_id: MoveBatchId,
    pub(crate) at: Timestamp,
}

/// An op written to the log before its file is touched.
#[must_use = "publish the event or discard it explicitly"]
#[derive(Debug, Clone, Serialize)]
pub(crate) struct MoveOpLogged {
    pub(crate) batch_id: MoveBatchId,
    pub(crate) seq: i32,
    pub(crate) status: MoveOpStatus,
}

#[must_use = "publish the event or discard it explicitly"]
#[derive(Debug, Clone, Serialize)]
pub(crate) struct MoveOpSettled {
    pub(crate) batch_id: MoveBatchId,
    pub(crate) seq: i32,
    pub(crate) status: MoveOpStatus,
}

#[must_use = "publish the event or discard it explicitly"]
#[derive(Debug, Clone, Serialize)]
pub(crate) struct MoveBatchFinished {
    pub(crate) batch_id: MoveBatchId,
    pub(crate) counts: MoveCounts,
    pub(crate) at: Timestamp,
}

#[must_use = "publish the event or discard it explicitly"]
#[derive(Debug, Clone, Serialize)]
pub(crate) struct MoveBatchUndoStarted {
    pub(crate) batch_id: MoveBatchId,
}

#[must_use = "publish the event or discard it explicitly"]
#[derive(Debug, Clone, Serialize)]
pub(crate) struct MoveBatchUndone {
    pub(crate) batch_id: MoveBatchId,
    pub(crate) at: Timestamp,
}

#[must_use = "publish the event or discard it explicitly"]
#[derive(Debug, Clone, Serialize)]
pub(crate) struct MoveOpReconciled {
    pub(crate) batch_id: MoveBatchId,
    pub(crate) seq: i32,
    pub(crate) status: MoveOpStatus,
    pub(crate) file_at: Option<LibraryPath>,
}

#[must_use = "publish the event or discard it explicitly"]
#[derive(Debug, Clone, Serialize)]
pub(crate) struct MoveBatchRecovered {
    pub(crate) batch_id: MoveBatchId,
    pub(crate) status: super::MoveBatchStatus,
    pub(crate) at: Timestamp,
}
