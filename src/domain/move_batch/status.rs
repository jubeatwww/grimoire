use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum MoveBatchStatus {
    Running,
    Done,
    Undoing,
    Undone,
}

impl MoveBatchStatus {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Running => "running",
            Self::Done => "done",
            Self::Undoing => "undoing",
            Self::Undone => "undone",
        }
    }
}

/// `Pending` and `UndoPending` are written before a file is touched, so an interrupted batch can
/// be reconciled against the disk afterwards.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum MoveOpStatus {
    Pending,
    Moved,
    Skipped,
    Failed,
    UndoPending,
    Undone,
    UndoSkipped,
    UndoFailed,
}

impl MoveOpStatus {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Moved => "moved",
            Self::Skipped => "skipped",
            Self::Failed => "failed",
            Self::UndoPending => "undo_pending",
            Self::Undone => "undone",
            Self::UndoSkipped => "undo_skipped",
            Self::UndoFailed => "undo_failed",
        }
    }
}
