use chrono::NaiveDate;
use grimoire_domain::metadata::MetadataCandidate;
use serde::{Deserialize, Deserializer};
use uuid::Uuid;

/// Coerce JSON `null` into the default for `T`. `#[serde(default)]` only
/// covers missing keys; DLsite (and occasionally VNDB) sends explicit nulls
/// for absent arrays, which would otherwise fail Vec deserialization.
pub fn null_to_default<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: Default + Deserialize<'de>,
{
    Option::<T>::deserialize(deserializer).map(|o| o.unwrap_or_default())
}

#[async_trait::async_trait]
pub trait MetadataSource {
    async fn search(&self, query: &str) -> anyhow::Result<Vec<MetadataCandidate>>;
}

/// Normalised product detail shared across sources. Fields the source does not
/// have stay `None` / empty; the persistence layer uses COALESCE / overwrite
/// per the configured precedence.
#[derive(Debug, Clone, Default)]
pub struct ProductDetail {
    pub work_name: Option<String>,
    pub maker_name: Option<String>,
    pub description: Option<String>,
    pub release_date: Option<NaiveDate>,
    pub series: Option<String>,
    pub tags: Vec<String>,
    pub cover_image_url: Option<String>,
    pub preview_image_urls: Vec<String>,
    pub file_type: Option<String>,
    pub file_size_bytes: Option<i64>,
    pub dl_count: Option<i32>,
    pub rate_average: Option<f32>,
    pub rate_count: Option<i32>,
    pub price_jpy: Option<i32>,
    pub work_type: Option<String>,
    pub work_type_label: Option<String>,
}

pub struct FixtureMetadataSource;

#[async_trait::async_trait]
impl MetadataSource for FixtureMetadataSource {
    async fn search(&self, query: &str) -> anyhow::Result<Vec<MetadataCandidate>> {
        Ok(vec![MetadataCandidate {
            id: Uuid::new_v4(),
            source_name: "dlsite-fixture".to_string(),
            source_work_id: "RJ000001".to_string(),
            source_url: "https://www.dlsite.com/maniax/work/=/product_id/RJ000001.html".to_string(),
            query_used: query.to_string(),
            rank: 1,
            title: "Sample Candidate".to_string(),
            circle: Some("Sample Circle".to_string()),
            cover_url: Some("https://example.invalid/cover.jpg".to_string()),
            normalized_payload: serde_json::json!({
                "title": "Sample Candidate",
                "circle": "Sample Circle"
            }),
        }])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn fixture_source_returns_ranked_candidate() {
        let source = FixtureMetadataSource;
        let result = source.search("sample").await.unwrap();
        assert_eq!(result[0].source_name, "dlsite-fixture");
        assert_eq!(result[0].rank, 1);
    }
}
