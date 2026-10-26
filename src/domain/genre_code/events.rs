use serde::Serialize;

#[must_use = "publish the event or discard it explicitly"]
#[derive(Debug, Clone, Serialize)]
pub(crate) struct GenreCodeDefined {
    pub(crate) code: String,
    pub(crate) category_id: String,
}
