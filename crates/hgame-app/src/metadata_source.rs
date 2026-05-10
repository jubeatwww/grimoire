use hgame_domain::metadata::MetadataCandidate;
use uuid::Uuid;

#[async_trait::async_trait]
pub trait MetadataSource {
    async fn search(&self, query: &str) -> anyhow::Result<Vec<MetadataCandidate>>;
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
