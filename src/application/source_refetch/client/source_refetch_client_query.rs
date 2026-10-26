use std::sync::Arc;

use tokio::sync::Mutex;

use super::SourceRefetchOutput;
use crate::domain::source_refetch::SourceRefetch;

pub(crate) struct SourceRefetchClientQueryUseCase {
    progress: Arc<Mutex<SourceRefetch>>,
}

impl SourceRefetchClientQueryUseCase {
    /// `progress` is shared with `StartSourceRefetchUseCase`.
    pub(crate) fn new(progress: Arc<Mutex<SourceRefetch>>) -> Self {
        Self { progress }
    }

    #[tracing::instrument(name = "use_case.find_source_refetch", skip(self))]
    pub(crate) async fn find_progress(&self) -> SourceRefetchOutput {
        SourceRefetchOutput::from(&*self.progress.lock().await)
    }
}
