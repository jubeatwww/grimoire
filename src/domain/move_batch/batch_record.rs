use super::{MoveOp, MoveOpStatus};
use crate::kernel::Timestamp;

/// How a batch's run ended, counted once when it finished.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize)]
pub(crate) struct MoveCounts {
    moved: i32,
    skipped: i32,
    failed: i32,
}

impl MoveCounts {
    pub(crate) fn new(moved: i32, skipped: i32, failed: i32) -> Self {
        Self {
            moved,
            skipped,
            failed,
        }
    }

    pub(super) fn of(ops: &[MoveOp]) -> Self {
        let count = |status: MoveOpStatus| {
            i32::try_from(ops.iter().filter(|op| op.status() == status).count()).unwrap_or(i32::MAX)
        };
        Self::new(
            count(MoveOpStatus::Moved),
            count(MoveOpStatus::Skipped),
            count(MoveOpStatus::Failed),
        )
    }

    pub(crate) fn moved(self) -> i32 {
        self.moved
    }

    pub(crate) fn skipped(self) -> i32 {
        self.skipped
    }

    pub(crate) fn failed(self) -> i32 {
        self.failed
    }
}

/// Folders the batch's pruning removed and what it reported, run and undo together.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct BatchNotes {
    removed_dirs: Vec<String>,
    notes: Vec<String>,
}

impl BatchNotes {
    pub(crate) fn new(removed_dirs: Vec<String>, notes: Vec<String>) -> Self {
        Self {
            removed_dirs,
            notes,
        }
    }

    pub(super) fn add(&mut self, removed_dirs: Vec<String>, notes: Vec<String>) {
        self.removed_dirs.extend(removed_dirs);
        self.notes.extend(notes);
    }

    pub(super) fn add_note(&mut self, note: String) {
        self.notes.push(note);
    }

    pub(crate) fn removed_dirs(&self) -> &[String] {
        &self.removed_dirs
    }

    pub(crate) fn notes(&self) -> &[String] {
        &self.notes
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct BatchTimes {
    created_at: Timestamp,
    finished_at: Option<Timestamp>,
    undone_at: Option<Timestamp>,
}

impl BatchTimes {
    pub(crate) fn new(
        created_at: Timestamp,
        finished_at: Option<Timestamp>,
        undone_at: Option<Timestamp>,
    ) -> Self {
        Self {
            created_at,
            finished_at,
            undone_at,
        }
    }

    pub(super) fn finished(self, at: Timestamp) -> Self {
        Self {
            finished_at: Some(at),
            ..self
        }
    }

    /// Keeps an earlier finish time.
    pub(super) fn finished_by(self, at: Timestamp) -> Self {
        Self {
            finished_at: self.finished_at.or(Some(at)),
            ..self
        }
    }

    pub(super) fn undone(self, at: Timestamp) -> Self {
        Self {
            undone_at: Some(at),
            ..self
        }
    }

    /// Keeps an earlier undo time.
    pub(super) fn undone_by(self, at: Timestamp) -> Self {
        Self {
            undone_at: self.undone_at.or(Some(at)),
            ..self
        }
    }

    pub(crate) fn created_at(self) -> Timestamp {
        self.created_at
    }

    pub(crate) fn finished_at(self) -> Option<Timestamp> {
        self.finished_at
    }

    pub(crate) fn undone_at(self) -> Option<Timestamp> {
        self.undone_at
    }
}
