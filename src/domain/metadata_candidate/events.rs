use serde::Serialize;

use super::MetadataCandidateId;
use crate::domain::metadata_source::MetadataProvider;

#[must_use = "publish the event or discard it explicitly"]
#[derive(Debug, Clone, Serialize)]
pub(crate) struct MetadataCandidateFound {
    pub(crate) candidate_id: MetadataCandidateId,
    pub(crate) provider: MetadataProvider,
    pub(crate) source_work_id: String,
}
