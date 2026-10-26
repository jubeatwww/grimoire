//! Undoing a finished batch: its moved files go back, newest first.

use super::{
    MoveBatch, MoveBatchError, MoveBatchStatus, MoveBatchUndoStarted, MoveBatchUndone, MoveOp,
    MoveOpLogged, MoveOpSettled, MoveOpStatus,
};
use crate::domain::library_path::LibraryPath;
use crate::kernel::Timestamp;

/// What happened to a file the undo tried to move back.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum UndoOutcome {
    Undone,
    Skipped(String),
    Failed(String),
}

impl UndoOutcome {
    /// The folder to prune afterwards: the batch target folder a file left.
    pub(crate) fn folder_to_prune(&self, op: &MoveOp) -> Option<LibraryPath> {
        match self {
            Self::Undone => op.to().parent(),
            Self::Skipped(_) | Self::Failed(_) => None,
        }
    }
}

impl MoveBatch {
    /// Ops whose file is at its batch target, newest first: the order an undo walks.
    pub(crate) fn moved_ops_newest_first(&self) -> Vec<i32> {
        let mut seqs: Vec<i32> = self
            .ops
            .iter()
            .filter(|op| op.status() == MoveOpStatus::Moved)
            .map(MoveOp::seq)
            .collect();
        seqs.sort_unstable_by(|a, b| b.cmp(a));
        seqs
    }

    /// Only a finished batch can be undone.
    pub(crate) fn start_undo(&mut self) -> Result<MoveBatchUndoStarted, MoveBatchError> {
        if self.status != MoveBatchStatus::Done {
            return Err(MoveBatchError::NotFinished(self.id));
        }
        self.status = MoveBatchStatus::Undoing;
        Ok(MoveBatchUndoStarted { batch_id: self.id })
    }

    /// Marks a moved op as being undone, before its file is touched.
    pub(crate) fn log_undo(&mut self, seq: i32) -> Result<MoveOpLogged, MoveBatchError> {
        self.require(MoveBatchStatus::Undoing)?;
        let batch_id = self.id;
        let op = self.op_mut(seq)?;
        if op.status() != MoveOpStatus::Moved {
            return Err(MoveBatchError::OpNotPending(seq));
        }
        op.change(MoveOpStatus::UndoPending, None);
        Ok(MoveOpLogged {
            batch_id,
            seq,
            status: MoveOpStatus::UndoPending,
        })
    }

    /// Records how an undo of one op ended. A message also goes into the batch notes.
    pub(crate) fn settle_undo(
        &mut self,
        seq: i32,
        outcome: UndoOutcome,
    ) -> Result<MoveOpSettled, MoveBatchError> {
        self.require(MoveBatchStatus::Undoing)?;
        let (status, message) = match outcome {
            UndoOutcome::Undone => (MoveOpStatus::Undone, None),
            UndoOutcome::Skipped(m) => (MoveOpStatus::UndoSkipped, Some(m)),
            UndoOutcome::Failed(m) => (MoveOpStatus::UndoFailed, Some(m)),
        };
        let settled = self.settle(seq, MoveOpStatus::UndoPending, status, message.clone())?;
        if let Some(m) = message {
            let to = self.op(seq)?.to().to_string();
            self.notes.add_note(format!("{to}: {m}"));
        }
        Ok(settled)
    }

    /// Ends the undo, adding what pruning removed and reported to the batch's log.
    pub(crate) fn finish_undo(
        &mut self,
        removed_dirs: Vec<String>,
        notes: Vec<String>,
        at: Timestamp,
    ) -> Result<MoveBatchUndone, MoveBatchError> {
        self.require(MoveBatchStatus::Undoing)?;
        self.status = MoveBatchStatus::Undone;
        self.notes.add(removed_dirs, notes);
        self.times = self.times.undone(at);
        Ok(MoveBatchUndone {
            batch_id: self.id,
            at,
        })
    }
}
