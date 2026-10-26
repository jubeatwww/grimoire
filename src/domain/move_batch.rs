//! One run of moves: every planned file gets an op, logged before its file is touched, so an
//! interrupted batch can be reconciled against the disk and a finished one undone.

mod batch_record;
mod error;
mod events;
mod move_batch_repository;
mod move_op;
mod reconciliation;
mod run;
mod status;
#[cfg(test)]
mod tests;
mod undo;

use crate::kernel::{Id, Timestamp};

pub(crate) use batch_record::{BatchNotes, BatchTimes, MoveCounts};
pub(crate) use error::MoveBatchError;
pub(crate) use events::{
    MoveBatchFinished, MoveBatchRecovered, MoveBatchStarted, MoveBatchUndoStarted, MoveBatchUndone,
    MoveOpLogged, MoveOpReconciled, MoveOpSettled,
};
pub(crate) use move_batch_repository::{MoveBatchRepository, MoveBatchRepositoryError};
pub(crate) use move_op::{MoveOp, MoveOpId};
pub(crate) use run::MoveOutcome;
pub(crate) use status::{MoveBatchStatus, MoveOpStatus};
pub(crate) use undo::UndoOutcome;

pub(crate) type MoveBatchId = Id<MoveBatch>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MoveBatch {
    id: MoveBatchId,
    status: MoveBatchStatus,
    counts: MoveCounts,
    notes: BatchNotes,
    times: BatchTimes,
    ops: Vec<MoveOp>,
}

impl MoveBatch {
    pub(crate) fn start(id: MoveBatchId, at: Timestamp) -> (Self, MoveBatchStarted) {
        let batch = Self {
            id,
            status: MoveBatchStatus::Running,
            counts: MoveCounts::default(),
            notes: BatchNotes::default(),
            times: BatchTimes::new(at, None, None),
            ops: Vec::new(),
        };
        (batch, MoveBatchStarted { batch_id: id, at })
    }

    /// Rebuilds a stored batch. Repositories only.
    pub(crate) fn from_persisted(
        id: MoveBatchId,
        status: MoveBatchStatus,
        counts: MoveCounts,
        notes: BatchNotes,
        times: BatchTimes,
        ops: Vec<MoveOp>,
    ) -> Self {
        Self {
            id,
            status,
            counts,
            notes,
            times,
            ops,
        }
    }

    fn require(&self, status: MoveBatchStatus) -> Result<(), MoveBatchError> {
        if self.status == status {
            Ok(())
        } else {
            Err(MoveBatchError::WrongState {
                batch: self.id,
                expected: status,
            })
        }
    }

    /// Moves one op from `expected` to `status`.
    fn settle(
        &mut self,
        seq: i32,
        expected: MoveOpStatus,
        status: MoveOpStatus,
        message: Option<String>,
    ) -> Result<MoveOpSettled, MoveBatchError> {
        let batch_id = self.id;
        let op = self.op_mut(seq)?;
        if op.status() != expected {
            return Err(MoveBatchError::OpNotPending(seq));
        }
        op.change(status, message);
        Ok(MoveOpSettled {
            batch_id,
            seq,
            status,
        })
    }

    fn op_mut(&mut self, seq: i32) -> Result<&mut MoveOp, MoveBatchError> {
        self.ops
            .iter_mut()
            .find(|op| op.seq() == seq)
            .ok_or(MoveBatchError::UnknownOp(seq))
    }

    pub(crate) fn op(&self, seq: i32) -> Result<&MoveOp, MoveBatchError> {
        self.ops
            .iter()
            .find(|op| op.seq() == seq)
            .ok_or(MoveBatchError::UnknownOp(seq))
    }

    pub(crate) fn id(&self) -> MoveBatchId {
        self.id
    }

    pub(crate) fn status(&self) -> MoveBatchStatus {
        self.status
    }

    pub(crate) fn counts(&self) -> MoveCounts {
        self.counts
    }

    pub(crate) fn notes(&self) -> &BatchNotes {
        &self.notes
    }

    pub(crate) fn times(&self) -> BatchTimes {
        self.times
    }

    pub(crate) fn ops(&self) -> &[MoveOp] {
        &self.ops
    }
}
