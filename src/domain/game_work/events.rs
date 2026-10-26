use serde::Serialize;

use super::GameWorkId;
use crate::domain::metadata_source::MetadataProvider;
use crate::kernel::Timestamp;

#[must_use = "publish the event or discard it explicitly"]
#[derive(Debug, Clone, Serialize)]
pub(crate) struct GameWorkRegistered {
    pub(crate) work_id: GameWorkId,
    pub(crate) provider: MetadataProvider,
    pub(crate) external_id: String,
}

#[must_use = "publish the event or discard it explicitly"]
#[derive(Debug, Clone, Serialize)]
pub(crate) struct GameWorkEntered {
    pub(crate) work_id: GameWorkId,
    pub(crate) at: Timestamp,
}

#[must_use = "publish the event or discard it explicitly"]
#[derive(Debug, Clone, Serialize)]
pub(crate) struct GameWorkEnriched {
    pub(crate) work_id: GameWorkId,
    pub(crate) at: Timestamp,
}

#[must_use = "publish the event or discard it explicitly"]
#[derive(Debug, Clone, Serialize)]
pub(crate) struct GameWorkCoverPrefilled {
    pub(crate) work_id: GameWorkId,
}

#[must_use = "publish the event or discard it explicitly"]
#[derive(Debug, Clone, Serialize)]
pub(crate) struct GameWorkEdited {
    pub(crate) work_id: GameWorkId,
}

#[must_use = "publish the event or discard it explicitly"]
#[derive(Debug, Clone, Serialize)]
pub(crate) struct GameWorkClassified {
    pub(crate) work_id: GameWorkId,
    pub(crate) category: Option<String>,
    pub(crate) at: Timestamp,
}
