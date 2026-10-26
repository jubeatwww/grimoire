use serde::Serialize;

#[derive(Debug, Serialize)]
pub(crate) struct ListCommonTagsOutput {
    pub(crate) tags: Vec<CommonTagListItemOutput>,
}

/// A source tag and how many unclassified works carry it.
#[derive(Debug, Clone, Serialize)]
pub(crate) struct CommonTagListItemOutput {
    pub(crate) label: String,
    pub(crate) source: String,
    pub(crate) count: i64,
}
