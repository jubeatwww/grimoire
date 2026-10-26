use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::Mutex;

use super::SourceRefetchOutput;
use crate::domain::game_work::{GameWorkRepository, GameWorkRepositoryError};
use crate::domain::metadata_source::{MetadataProvider, MetadataSource};
use crate::domain::source_refetch::{RefetchTask, SourceRefetch, SourceRefetchError};
use crate::domain::work_source_data::{WorkSourceData, WorkSourceDataRepository};
use crate::kernel::{Clock, ErrorCode, ErrorKind};

/// One request per second, to stay polite with the sites.
const REQUEST_INTERVAL: Duration = Duration::from_secs(1);
/// Linked works; this only guards against a runaway table.
const WORK_LIMIT: i64 = 100_000;

#[derive(Debug, thiserror::Error)]
pub(crate) enum StartSourceRefetchError {
    #[error(transparent)]
    Refetch(#[from] SourceRefetchError),
    #[error(transparent)]
    Works(#[from] GameWorkRepositoryError),
}

impl ErrorCode for StartSourceRefetchError {
    fn code(&self) -> &'static str {
        match self {
            Self::Refetch(e) => e.code(),
            Self::Works(_) => "grimoire.unavailable",
        }
    }

    fn kind(&self) -> ErrorKind {
        match self {
            Self::Refetch(e) => e.kind(),
            Self::Works(_) => ErrorKind::Unavailable,
        }
    }
}

/// Starts walking every site id of every linked work in the background. Existing tags were
/// stored without their source, and a later link to another source could overwrite DLsite's
/// work type; this rewrites only each source's own data, so titles, circles, descriptions and
/// images on the works (and manual edits) survive.
pub(crate) struct StartSourceRefetchUseCase {
    works: Arc<dyn GameWorkRepository>,
    sources: HashMap<MetadataProvider, Arc<dyn MetadataSource>>,
    source_data: Arc<dyn WorkSourceDataRepository>,
    progress: Arc<Mutex<SourceRefetch>>,
    clock: Arc<dyn Clock>,
}

impl StartSourceRefetchUseCase {
    pub(crate) fn new(
        works: Arc<dyn GameWorkRepository>,
        sources: HashMap<MetadataProvider, Arc<dyn MetadataSource>>,
        source_data: Arc<dyn WorkSourceDataRepository>,
        progress: Arc<Mutex<SourceRefetch>>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            works,
            sources,
            source_data,
            progress,
            clock,
        }
    }

    #[tracing::instrument(name = "use_case.start_source_refetch", skip(self))]
    pub(crate) async fn execute(
        self: Arc<Self>,
    ) -> Result<SourceRefetchOutput, StartSourceRefetchError> {
        let tasks = RefetchTask::for_works(&self.works.list_linked(WORK_LIMIT).await?);
        let snapshot = {
            let mut progress = self.progress.lock().await;
            let _started = progress.start(tasks.len(), self.clock.now())?;
            SourceRefetchOutput::from(&*progress)
        };
        tokio::spawn(async move { self.run(tasks).await });
        Ok(snapshot)
    }

    async fn run(&self, tasks: Vec<RefetchTask>) {
        for (index, task) in tasks.iter().enumerate() {
            if index > 0 {
                tokio::time::sleep(REQUEST_INTERVAL).await;
            }
            let outcome = self.refetch_one(task).await;
            let mut progress = self.progress.lock().await;
            let _progressed = match outcome {
                Ok(()) => progress.record_success(),
                Err(message) => {
                    tracing::warn!(
                        work = %task.work_id(),
                        source = task.provider().name(),
                        id = task.external_id(),
                        error = %message,
                        "re-fetch failed"
                    );
                    progress.record_failure(task, &message)
                }
            };
        }
        let _finished = self.progress.lock().await.finish(self.clock.now());
    }

    async fn refetch_one(&self, task: &RefetchTask) -> Result<(), String> {
        let source = self
            .sources
            .get(&task.provider())
            .ok_or_else(|| "source is not configured".to_string())?;
        let detail = match source.find_detail(task.external_id()).await {
            Ok(Some(detail)) => detail,
            Ok(None) => return Err("source has no record".to_string()),
            Err(e) => return Err(e.to_string()),
        };
        let (data, _recorded) =
            WorkSourceData::record(task.work_id(), task.provider(), &detail, self.clock.now());
        self.source_data
            .save(&data)
            .await
            .map_err(|e| e.to_string())
    }
}
