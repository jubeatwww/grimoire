//! A one-off background re-fetch of every linked work's source data. It rewrites only each
//! source's own tags and work type, so manual edits on the works survive.

mod error;
mod events;
mod refetch_task;

use crate::kernel::Timestamp;

pub(crate) use error::SourceRefetchError;
pub(crate) use events::{SourceRefetchFinished, SourceRefetchProgressed, SourceRefetchStarted};
pub(crate) use refetch_task::RefetchTask;

/// Progress of the latest re-fetch. Only one runs at a time.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct SourceRefetch {
    running: bool,
    total: usize,
    done: usize,
    failed: usize,
    started_at: Option<Timestamp>,
    finished_at: Option<Timestamp>,
    last_error: Option<String>,
}

impl SourceRefetch {
    pub(crate) fn idle() -> Self {
        Self::default()
    }

    pub(crate) fn start(
        &mut self,
        total: usize,
        at: Timestamp,
    ) -> Result<SourceRefetchStarted, SourceRefetchError> {
        if self.running {
            return Err(SourceRefetchError::AlreadyRunning);
        }
        *self = Self {
            running: true,
            total,
            started_at: Some(at),
            ..Self::default()
        };
        Ok(SourceRefetchStarted { total, at })
    }

    pub(crate) fn record_success(&mut self) -> SourceRefetchProgressed {
        self.done += 1;
        self.progressed()
    }

    pub(crate) fn record_failure(
        &mut self,
        task: &RefetchTask,
        message: &str,
    ) -> SourceRefetchProgressed {
        self.done += 1;
        self.failed += 1;
        self.last_error = Some(format!(
            "{} {}: {}",
            task.provider().name(),
            task.external_id(),
            message
        ));
        self.progressed()
    }

    fn progressed(&self) -> SourceRefetchProgressed {
        SourceRefetchProgressed {
            done: self.done,
            failed: self.failed,
        }
    }

    pub(crate) fn finish(&mut self, at: Timestamp) -> SourceRefetchFinished {
        self.running = false;
        self.finished_at = Some(at);
        SourceRefetchFinished { at }
    }

    pub(crate) fn running(&self) -> bool {
        self.running
    }

    pub(crate) fn total(&self) -> usize {
        self.total
    }

    pub(crate) fn done(&self) -> usize {
        self.done
    }

    pub(crate) fn failed(&self) -> usize {
        self.failed
    }

    pub(crate) fn started_at(&self) -> Option<Timestamp> {
        self.started_at
    }

    pub(crate) fn finished_at(&self) -> Option<Timestamp> {
        self.finished_at
    }

    pub(crate) fn last_error(&self) -> Option<&str> {
        self.last_error.as_deref()
    }
}
