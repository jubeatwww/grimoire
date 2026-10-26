//! Each source's own tags and work type for a work. `GameWork` keeps whatever was applied last;
//! this keeps every source's values, so tags can show where they came from and the classifier
//! can read DLsite's work type even after a VNDB link.

mod events;
mod work_source_data_repository;

use crate::domain::game_work::GameWorkId;
use crate::domain::metadata_source::{MetadataProvider, ProductDetail};
use crate::kernel::Timestamp;

pub(crate) use events::WorkSourceDataRecorded;
pub(crate) use work_source_data_repository::{
    WorkSourceDataRepository, WorkSourceDataRepositoryError,
};

/// One source's raw tags and work type for one work. Identified by the work and the source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WorkSourceData {
    work_id: GameWorkId,
    provider: MetadataProvider,
    tags: Vec<String>,
    work_type: Option<String>,
    work_type_label: Option<String>,
    fetched_at: Timestamp,
}

impl WorkSourceData {
    /// What `provider` said about the work at `at`; replaces anything recorded before.
    pub(crate) fn record(
        work_id: GameWorkId,
        provider: MetadataProvider,
        product: &ProductDetail,
        at: Timestamp,
    ) -> (Self, WorkSourceDataRecorded) {
        let detail = product.detail();
        let data = Self {
            work_id,
            provider,
            tags: detail.tags().to_vec(),
            work_type: detail.work_type().map(str::to_owned),
            work_type_label: detail.work_type_label().map(str::to_owned),
            fetched_at: at,
        };
        (
            data,
            WorkSourceDataRecorded {
                work_id,
                provider,
                at,
            },
        )
    }

    pub(crate) fn work_id(&self) -> GameWorkId {
        self.work_id
    }

    pub(crate) fn provider(&self) -> MetadataProvider {
        self.provider
    }

    pub(crate) fn tags(&self) -> &[String] {
        &self.tags
    }

    pub(crate) fn work_type(&self) -> Option<&str> {
        self.work_type.as_deref()
    }

    pub(crate) fn work_type_label(&self) -> Option<&str> {
        self.work_type_label.as_deref()
    }

    pub(crate) fn fetched_at(&self) -> Timestamp {
        self.fetched_at
    }
}
