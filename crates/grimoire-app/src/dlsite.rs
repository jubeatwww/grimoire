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

pub fn clean_query(filename: &str) -> String {
    let s = Regex::new(r"(?i)\.(zip|rar|7z|exe|iso)$").unwrap().replace(filename, "");
    let s = Regex::new(r"\[.*?\]").unwrap().replace_all(&s, " ");
    let s = Regex::new(r"\(.*?\)").unwrap().replace_all(&s, " ");
    let s = Regex::new(r"(?i)[vV]?\d+\.\d+[\d.]*").unwrap().replace_all(&s, " ");
    let s = Regex::new(r"\+\d+").unwrap().replace_all(&s, " ");
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

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
        RJ_CODE_RE.find(filename).map(|m| m.as_str().to_uppercase())
    }

    async fn fetch_by_work_id(&self, work_id: &str) -> anyhow::Result<Option<MetadataCandidate>> {
        let ajax_url = format!(
            "https://www.dlsite.com/maniax/product/info/ajax?product_id={}",
            work_id
        );
        let resp = self.client.get(&ajax_url).send().await?;
        if !resp.status().is_success() {
            return Ok(None);
        }
        let ajax_map: HashMap<String, ProductAjax> = resp.json().await?;
        let Some(ajax) = ajax_map.into_values().next() else {
            return Ok(None);
        };

        let title = ajax.work_name.unwrap_or_default();
        let cover_url = ajax.work_image.map(|img| normalize_url(&img));
        let source_url = format!(
            "https://www.dlsite.com/maniax/work/=/product_id/{}.html",
            work_id
        );

        // Use suggest API to get maker_name (product info ajax doesn't include it)
        let circle = self.suggest_maker(work_id).await.ok().flatten();

        Ok(Some(MetadataCandidate {
            id: Uuid::new_v4(),
            source_name: "dlsite".to_string(),
            source_work_id: work_id.to_string(),
            source_url,
            query_used: work_id.to_string(),
            rank: 1,
            title: title.clone(),
            circle: circle.clone(),
            cover_url,
            normalized_payload: serde_json::json!({
                "title": title,
                "circle": circle,
                "release_date": ajax.regist_date,
            }),
        }))
    }

    async fn suggest_maker(&self, work_id: &str) -> anyhow::Result<Option<String>> {
        let suggest = self.suggest(work_id).await?;
        Ok(suggest.work.into_iter().next().and_then(|w| w.maker_name))
    }

    async fn suggest(&self, term: &str) -> anyhow::Result<SuggestResponse> {
        let url = format!(
            "https://www.dlsite.com/suggest/?term={}&site=adult-jp&time=1&touch=0&_=1",
            urlencoding::encode(term)
        );
        let resp = self.client.get(&url).send().await?;
        if !resp.status().is_success() {
            return Ok(SuggestResponse::default());
        }
        let body = resp.text().await?;
        // Response may be JSONP (with callback wrapper) or plain JSON
        let json_str = if let Some(start) = body.find('(') {
            &body[start + 1..body.len().saturating_sub(1)]
        } else {
            &body
        };
        Ok(serde_json::from_str(json_str).unwrap_or_default())
    }

    async fn search_by_keyword(&self, query: &str) -> anyhow::Result<Vec<MetadataCandidate>> {
        let suggest = self.suggest(query).await?;

        let candidates = suggest
            .work
            .into_iter()
            .enumerate()
            .map(|(i, item)| {
                let work_id = item.workno.unwrap_or_default();
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
                    rank: (i + 1) as i32,
                    title: item.work_name.clone().unwrap_or_default(),
                    circle: item.maker_name.clone(),
                    cover_url: None,
                    normalized_payload: serde_json::json!({
                        "title": item.work_name,
                        "circle": item.maker_name,
                        "work_type": item.work_type,
                    }),
                }
            })
            .collect();

        Ok(candidates)
    }
}

#[async_trait::async_trait]
impl MetadataSource for DlsiteSource {
    async fn search(&self, query: &str) -> anyhow::Result<Vec<MetadataCandidate>> {
        if let Some(work_id) = Self::extract_work_id(query) {
            if let Some(candidate) = self.fetch_by_work_id(&work_id).await? {
                return Ok(vec![candidate]);
            }
        }

        self.search_by_keyword(query).await
    }
}

#[derive(Debug, Deserialize)]
struct ProductAjax {
    work_name: Option<String>,
    work_image: Option<String>,
    regist_date: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
struct SuggestResponse {
    #[serde(default)]
    work: Vec<SuggestWork>,
}

#[derive(Debug, Deserialize)]
struct SuggestWork {
    work_name: Option<String>,
    workno: Option<String>,
    maker_name: Option<String>,
    work_type: Option<String>,
}

fn normalize_url(url: &str) -> String {
    if url.starts_with("//") {
        format!("https:{url}")
    } else {
        url.to_string()
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

    #[test]
    fn cleans_filename_for_search() {
        assert_eq!(clean_query("RoomGirl V2.0.1+200.rar"), "RoomGirl");
        assert_eq!(clean_query("[SomeCircle] My Game (v1.02).zip"), "My Game");
        assert_eq!(clean_query("魔法少女RPG.zip"), "魔法少女RPG");
    }

    #[test]
    fn parses_suggest_json() {
        let json = r#"{"work":[{"work_name":"Test","workno":"RJ123456","maker_name":"Circle","work_type":"RPG"}],"maker":[],"reqtime":1}"#;
        let resp: SuggestResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.work.len(), 1);
        assert_eq!(resp.work[0].workno.as_deref(), Some("RJ123456"));
        assert_eq!(resp.work[0].maker_name.as_deref(), Some("Circle"));
    }
}