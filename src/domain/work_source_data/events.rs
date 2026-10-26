use serde::Serialize;

use crate::domain::game_work::GameWorkId;
use crate::domain::metadata_source::MetadataProvider;
use crate::kernel::Timestamp;

#[must_use = "publish the event or discard it explicitly"]
#[derive(Debug, Clone, Serialize)]
pub(crate) struct WorkSourceDataRecorded {
    pub(crate) work_id: GameWorkId,
    pub(crate) provider: MetadataProvider,
    pub(crate) at: Timestamp,
}
