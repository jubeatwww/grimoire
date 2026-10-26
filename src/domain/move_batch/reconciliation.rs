//! Settling a batch a crash, restart or lost database connection left running or undoing. Where
//! the file really is decides each op; the inventory record follows the file.

use super::{
    MoveBatch, MoveBatchError, MoveBatchRecovered, MoveBatchStatus, MoveCounts, MoveOpReconciled,
    MoveOpStatus,
};
use crate::domain::library_files::MoveProbe;
use crate::domain::library_path::LibraryPath;
use crate::kernel::Timestamp;

/// What the caller has to do on disk and in the inventory after an op was reconciled.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OpRecovery {
    remove_link: Option<LibraryPath>,
    file_at: Option<LibraryPath>,
    /// The other end of the move, where a stale record may still point.
    other_end: LibraryPath,
}

impl OpRecovery {
    /// A second hard link of the file to remove: the link step of the move ran, the unlink did
    /// not.
    pub(crate) fn remove_link(&self) -> Option<&LibraryPath> {
        self.remove_link.as_ref()
    }

    /// The record follows the file: given where the record points, the `(from, to)` it has to
    /// be moved along, if at all. Nothing when the file is at neither end.
    pub(crate) fn record_move(
        &self,
        recorded: &LibraryPath,
    ) -> Option<(&LibraryPath, &LibraryPath)> {
        let target = self.file_at.as_ref()?;
        (recorded != target).then_some((&self.other_end, target))
    }
}

impl MoveBatch {
    /// Ops left pending by an interrupted run or undo.
    pub(crate) fn interrupted_ops(&self) -> Vec<i32> {
        self.ops
            .iter()
            .filter(|op| {
                matches!(
                    op.status(),
                    MoveOpStatus::Pending | MoveOpStatus::UndoPending
                )
            })
            .map(|op| op.seq())
            .collect()
    }

    /// Decides an interrupted op from what is on disk.
    pub(crate) fn reconcile_op(
        &mut self,
        seq: i32,
        probe: MoveProbe,
    ) -> Result<(OpRecovery, MoveOpReconciled), MoveBatchError> {
        let batch_id = self.id;
        let op = self.op_mut(seq)?;
        let (from, to) = (op.from().clone(), op.to().clone());
        let both_links = probe.source_exists() && probe.target_exists() && probe.same_file();
        let (status, message, remove_link, file_at) = match op.status() {
            MoveOpStatus::Pending => {
                if probe.target_exists() && !probe.source_exists() {
                    (
                        MoveOpStatus::Moved,
                        "recovered after an interrupted batch",
                        None,
                        Some(to),
                    )
                } else {
                    let remove_link = both_links.then(|| to.clone());
                    if probe.source_exists() {
                        (
                            MoveOpStatus::Failed,
                            "interrupted before the move finished; file left at its original path",
                            remove_link,
                            Some(from),
                        )
                    } else {
                        (
                            MoveOpStatus::Failed,
                            "interrupted; file found at neither path",
                            remove_link,
                            None,
                        )
                    }
                }
            }
            MoveOpStatus::UndoPending => {
                if probe.source_exists() && !probe.target_exists() {
                    (
                        MoveOpStatus::Undone,
                        "recovered after an interrupted undo",
                        None,
                        Some(from),
                    )
                } else {
                    let remove_link = both_links.then(|| from.clone());
                    (
                        MoveOpStatus::Moved,
                        "undo interrupted; file still at the batch target",
                        remove_link,
                        Some(to),
                    )
                }
            }
            _ => return Err(MoveBatchError::OpNotPending(seq)),
        };
        let other_end = if file_at.as_ref() == Some(op.to()) {
            op.from().clone()
        } else {
            op.to().clone()
        };
        op.change(status, Some(message.to_owned()));
        let reconciled = MoveOpReconciled {
            batch_id,
            seq,
            status,
            file_at: file_at.clone(),
        };
        Ok((
            OpRecovery {
                remove_link,
                file_at,
                other_end,
            },
            reconciled,
        ))
    }

    /// Closes an interrupted batch once its ops are reconciled. A run becomes done with its
    /// counts taken from the ops; an undo becomes done again while some files are still at
    /// their target, so they can be undone again, and undone otherwise.
    pub(crate) fn recover(&mut self, at: Timestamp) -> MoveBatchRecovered {
        match self.status {
            MoveBatchStatus::Running => {
                self.status = MoveBatchStatus::Done;
                self.counts = MoveCounts::of(&self.ops);
                self.times = self.times.finished_by(at);
                self.notes.add_note(
                    "the batch was interrupted and reconciled against the disk".to_owned(),
                );
            }
            MoveBatchStatus::Undoing => {
                let still_moved = self.ops.iter().any(|op| op.status() == MoveOpStatus::Moved);
                self.status = if still_moved {
                    MoveBatchStatus::Done
                } else {
                    MoveBatchStatus::Undone
                };
                self.times = self.times.undone_by(at);
                self.notes.add_note(
                    "the undo was interrupted and reconciled; files still at their target can be undone again"
                        .to_owned(),
                );
            }
            MoveBatchStatus::Done | MoveBatchStatus::Undone => {}
        }
        MoveBatchRecovered {
            batch_id: self.id,
            status: self.status,
            at,
        }
    }
}
