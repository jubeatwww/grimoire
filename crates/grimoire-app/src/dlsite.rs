use crate::metadata_source::MetadataSource;
use grimoire_domain::metadata::MetadataCandidate;
use regex::Regex;
use reqwest::Client;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::LazyLock;
use uuid::Uuid;

static RJ_CODE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)(RJ|VJ|BJ)\d{6,8}").unwrap());

pub struct DlsiteSource {
    client: Client,
}

impl DlsiteSource {
    pub fn new() -> Self {
        Self {
            client: Client::builder()
                .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64)")
                .build()
                .expect("failed to build HTTP client"),
        }
    }

    pub fn extract_work_id(filename: &str) -> Option<String> {
        RJ_CODE_RE
            .find(filename)
            .map(|m| m.as_str().to_uppercase())
    }

    async fn fetch_product_info(
        &self,
        work_id: &str,
    ) -> anyhow::Result<Option<DlsiteProductInfo>> {
        let url = format!(
            "https://www.dlsite.com/maniax/product/info/ajax?product_id={}",
            work_id
        );

        let resp = self.client.get(&url).send().await?;

        if !resp.status().is_success() {
            return Ok(None);
        }

        let body: HashMap<String, DlsiteProductInfo> = resp.json().await?;
        Ok(body.into_values().next())
    }

    async fn search_by_keyword(&self, query: &str) -> anyhow::Result<Vec<DlsiteSearchItem>> {
        let url = format!(
            "https://www.dlsite.com/maniax/fsr/=/language/jp/keyword/{}/per_page/10/page/1/order/trend/options_and_or/and/.js",
            urlencoding::encode(query)
        );

        let resp = self.client.get(&url).send().await?;

        if !resp.status().is_success() {
            return Ok(Vec::new());
        }

        let body: DlsiteSearchResponse = resp.json().await?;
        Ok(body.work)
    }
}

#[async_trait::async_trait]
impl MetadataSource for DlsiteSource {
    async fn search(&self, query: &str) -> anyhow::Result<Vec<MetadataCandidate>> {
        // First try to extract RJ code from query and fetch directly
        if let Some(work_id) = Self::extract_work_id(query) {
            if let Some(info) = self.fetch_product_info(&work_id).await? {
                return Ok(vec![info.into_candidate(&work_id, query, 1)]);
            }
        }

        // Fallback to keyword search
        let items = self.search_by_keyword(query).await?;
        let candidates = items
            .into_iter()
            .enumerate()
            .map(|(i, item)| item.into_candidate(query, (i + 1) as i32))
            .collect();

        Ok(candidates)
    }
}

#[derive(Debug, Deserialize)]
struct DlsiteProductInfo {
    work_name: Option<String>,
    maker_name: Option<String>,
    work_image: Option<String>,
    #[serde(default)]
    genre: Vec<DlsiteGenre>,
    regist_date: Option<String>,
}

#[derive(Debug, Deserialize)]
struct DlsiteGenre {
    name: Option<String>,
}

impl DlsiteProductInfo {
    fn into_candidate(self, work_id: &str, query: &str, rank: i32) -> MetadataCandidate {
        let title = self.work_name.unwrap_or_default();
        let circle = self.maker_name.clone();
        let cover_url = self.work_image.map(|img| {
            if img.starts_with("//") {
                format!("https:{img}")
            } else {
                img
            }
        });
        let source_url = format!(
            "https://www.dlsite.com/maniax/work/=/product_id/{}.html",
            work_id
        );
        let genres: Vec<String> = self
            .genre
            .into_iter()
            .filter_map(|g| g.name)
            .collect();

        MetadataCandidate {
            id: Uuid::new_v4(),
            source_name: "dlsite".to_string(),
            source_work_id: work_id.to_string(),
            source_url,
            query_used: query.to_string(),
            rank,
            title: title.clone(),
            circle: circle.clone(),
            cover_url,
            normalized_payload: serde_json::json!({
                "title": title,
                "circle": circle,
                "genres": genres,
                "release_date": self.regist_date,
            }),
        }
    }
}

#[derive(Debug, Deserialize)]
struct DlsiteSearchResponse {
    #[serde(default)]
    work: Vec<DlsiteSearchItem>,
}

#[derive(Debug, Deserialize)]
struct DlsiteSearchItem {
    workno: Option<String>,
    work_name: Option<String>,
    maker_name: Option<String>,
    work_image: Option<String>,
}

impl DlsiteSearchItem {
    fn into_candidate(self, query: &str, rank: i32) -> MetadataCandidate {
        let work_id = self.workno.unwrap_or_default();
        let title = self.work_name.unwrap_or_default();
        let circle = self.maker_name;
        let cover_url = self.work_image.map(|img| {
            if img.starts_with("//") {
                format!("https:{img}")
            } else {
                img
            }
        });
        let source_url = format!(
            "https://www.dlsite.com/maniax/work/=/product_id/{}.html",
            work_id
        );

        MetadataCandidate {
            id: Uuid::new_v4(),
            source_name: "dlsite".to_string(),
            source_work_id: work_id,
            source_url,
            query_used: query.to_string(),
            rank,
            title: title.clone(),
            circle: circle.clone(),
            cover_url,
            normalized_payload: serde_json::json!({
                "title": title,
                "circle": circle,
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_rj_code_from_filename() {
        assert_eq!(
            DlsiteSource::extract_work_id("RJ01234567_some_game.zip"),
            Some("RJ01234567".to_string())
        );
        assert_eq!(
            DlsiteSource::extract_work_id("[Circle] Game Name (rj123456).rar"),
            Some("RJ123456".to_string())
        );
        assert_eq!(
            DlsiteSource::extract_work_id("VJ012345_title.zip"),
            Some("VJ012345".to_string())
        );
        assert_eq!(DlsiteSource::extract_work_id("no_code_here.zip"), None);
    }
}
