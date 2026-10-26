use serde::Serialize;

use crate::domain::metadata_source::MetadataProvider;

#[must_use = "publish the event or discard it explicitly"]
#[derive(Debug, Clone, Serialize)]
pub(crate) struct WorkTypeRuleDefined {
    pub(crate) source: MetadataProvider,
    pub(crate) work_type: String,
}
