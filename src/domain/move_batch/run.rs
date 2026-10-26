//! A batch's run: each planned move is logged before its file is touched, settled after.

use super::{
    BatchNotes, MoveBatch, MoveBatchError, MoveBatchFinished, MoveBatchStatus, MoveCounts, MoveOp,
    MoveOpId, MoveOpLogged, MoveOpSettled, MoveOpStatus,
};
use crate::domain::library_path::LibraryPath;
use crate::domain::move_plan::PlannedMove;
use crate::kernel::Timestamp;

/// What happened to a file the batch tried to move.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum MoveOutcome {
    Moved,
    Skipped(String),
    Failed(String),
}

impl MoveOutcome {
    /// The folder to prune afterwards: the one a moved file left, or the target folders a
    /// failed move may have created empty.
    pub(crate) fn folder_to_prune(&self, planned: &PlannedMove) -> Option<LibraryPath> {
        match self {
            Self::Moved => planned.from().parent(),
            Self::Failed(_) => planned.to().parent(),
            Self::Skipped(_) => None,
        }
    }
}

impl MoveBatch {
    /// Logs the next planned move as pending, before its file is touched.
    pub(crate) fn log_move(
        &mut self,
        op_id: MoveOpId,
        planned: &PlannedMove,
    ) -> Result<MoveOpLogged, MoveBatchError> {
        self.push_op(op_id, planned, MoveOpStatus::Pending, None)
    }

    /// Logs the next planned move as skipped, for a move blocked by a conflict.
    pub(crate) fn log_skip(
        &mut self,
        op_id: MoveOpId,
        planned: &PlannedMove,
        reason: String,
    ) -> Result<MoveOpLogged, MoveBatchError> {
        self.push_op(op_id, planned, MoveOpStatus::Skipped, Some(reason))
    }

    fn push_op(
        &mut self,
        op_id: MoveOpId,
        planned: &PlannedMove,
        status: MoveOpStatus,
        message: Option<String>,
    ) -> Result<MoveOpLogged, MoveBatchError> {
        self.require(MoveBatchStatus::Running)?;
        let seq = i32::try_from(self.ops.len()).unwrap_or(i32::MAX);
        self.ops.push(MoveOp::from_persisted(
            op_id,
            seq,
            Some(planned.item_id()),
            planned.from().clone(),
            planned.to().clone(),
            status,
            message,
        ));
        Ok(MoveOpLogged {
            batch_id: self.id,
            seq,
            status,
        })
    }

    /// Records how a pending move ended.
    pub(crate) fn settle_move(
        &mut self,
        seq: i32,
        outcome: MoveOutcome,
    ) -> Result<MoveOpSettled, MoveBatchError> {
        self.require(MoveBatchStatus::Running)?;
        let (status, message) = match outcome {
            MoveOutcome::Moved => (MoveOpStatus::Moved, None),
            MoveOutcome::Skipped(m) => (MoveOpStatus::Skipped, Some(m)),
            MoveOutcome::Failed(m) => (MoveOpStatus::Failed, Some(m)),
        };
        self.settle(seq, MoveOpStatus::Pending, status, message)
    }

    /// Ends the run: counts the ops and keeps what pruning removed and reported.
    pub(crate) fn finish(
        &mut self,
        removed_dirs: Vec<String>,
        notes: Vec<String>,
        at: Timestamp,
    ) -> Result<MoveBatchFinished, MoveBatchError> {
        self.require(MoveBatchStatus::Running)?;
        self.status = MoveBatchStatus::Done;
        self.counts = MoveCounts::of(&self.ops);
        self.notes = BatchNotes::new(removed_dirs, notes);
        self.times = self.times.finished(at);
        Ok(MoveBatchFinished {
            batch_id: self.id,
            counts: self.counts,
            at,
        })
    }
}
