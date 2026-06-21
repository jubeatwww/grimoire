use crate::metadata_source::{null_to_default, MetadataSource, ProductDetail};
use chrono::NaiveDate;
use grimoire_domain::metadata::MetadataCandidate;
use regex::Regex;
use reqwest::Client;
use serde::Deserialize;
use std::sync::LazyLock;
use uuid::Uuid;

const DLSITE_IMG_BASE: &str = "https://img.dlsite.jp/";

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

    pub async fn fetch_product_detail(&self, workno: &str) -> anyhow::Result<Option<ProductDetail>> {
        let url = format!(
            "https://www.dlsite.com/maniax/api/=/product.json?workno={}",
            workno
        );
        let resp = self.client.get(&url).send().await?;
        if !resp.status().is_success() {
            return Ok(None);
        }
        let mut items: Vec<ProductJson> = resp.json().await?;
        let Some(p) = items.pop() else { return Ok(None); };
        Ok(Some(p.into_detail()))
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

    async fn search_by_term(&self, term: &str) -> anyhow::Result<Vec<MetadataCandidate>> {
        let suggest = self.suggest(term).await?;

        let candidates = suggest
            .work
            .into_iter()
            // Drop delisted ("ana") works — their pages 404 and image URLs don't resolve.
            .filter(|item| !item.is_ana.unwrap_or(false))
            .enumerate()
            .map(|(i, item)| {
                let work_id = item.workno.unwrap_or_default();
                let source_url = format!(
                    "https://www.dlsite.com/maniax/work/=/product_id/{}.html",
                    work_id
                );
                let cover_url = cover_url_for_workno(&work_id);

                MetadataCandidate {
                    id: Uuid::new_v4(),
                    source_name: "dlsite".to_string(),
                    source_work_id: work_id,
                    source_url,
                    query_used: term.to_string(),
                    rank: (i + 1) as i32,
                    title: item.work_name.clone().unwrap_or_default(),
                    circle: item.maker_name.clone(),
                    cover_url,
                    normalized_payload: serde_json::json!({
                        "title": item.work_name,
                        "circle": item.maker_name,
                        "work_type": item.work_type,
                        "intro_s": item.intro_s,
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
        // RJ codes get exact-match candidates from suggest; otherwise it's a keyword search.
        let term = Self::extract_work_id(query).unwrap_or_else(|| query.to_string());
        self.search_by_term(&term).await
    }
}

fn cover_url_for_workno(workno: &str) -> Option<String> {
    if workno.len() < 3 {
        return None;
    }
    let (prefix, digits) = workno.split_at(2);
    let n: u64 = digits.parse().ok()?;
    // Folder bucket is the next multiple of 1000 (n=1..1000 -> 1000; n=1001..2000 -> 2000).
    let folder_num = (n.saturating_sub(1) / 1000 + 1) * 1000;
    let folder = format!("{prefix}{folder_num:0width$}", width = digits.len());
    let category = match prefix {
        "RJ" => "doujin",
        "VJ" => "professional",
        "BJ" => "books",
        _ => return None,
    };
    Some(format!(
        "https://img.dlsite.jp/modpub/images2/work/{category}/{folder}/{workno}_img_main.jpg"
    ))
}

#[derive(Debug, Default, Deserialize)]
struct SuggestResponse {
    #[serde(default, deserialize_with = "null_to_default")]
    work: Vec<SuggestWork>,
}

#[derive(Debug, Deserialize)]
struct SuggestWork {
    work_name: Option<String>,
    workno: Option<String>,
    maker_name: Option<String>,
    work_type: Option<String>,
    intro_s: Option<String>,
    is_ana: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct ProductJson {
    work_name: Option<String>,
    maker_name: Option<String>,
    intro: Option<String>,
    intro_s: Option<String>,
    regist_date: Option<String>,
    series_name: Option<String>,
    #[serde(default, deserialize_with = "null_to_default")]
    genres: Vec<GenreJson>,
    image_main: Option<ImageJson>,
    #[serde(default, deserialize_with = "null_to_default")]
    image_samples: Vec<ImageJson>,
    file_type_string: Option<String>,
    contents_file_size: Option<i64>,
    dl_count: Option<i32>,
    rate_average_2dp: Option<f32>,
    rate_count: Option<i32>,
    price: Option<i32>,
    work_type: Option<String>,
    work_type_string: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GenreJson {
    name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ImageJson {
    relative_url: Option<String>,
}

impl ImageJson {
    fn to_url(&self) -> Option<String> {
        self.relative_url
            .as_deref()
            .map(|u| format!("{DLSITE_IMG_BASE}{}", u.trim_start_matches('/')))
    }
}

impl ProductJson {
    fn into_detail(self) -> ProductDetail {
        ProductDetail {
            work_name: self.work_name,
            maker_name: self.maker_name,
            description: self.intro.or(self.intro_s),
            release_date: self
                .regist_date
                .as_deref()
                .and_then(parse_dlsite_date),
            series: self.series_name.filter(|s| !s.is_empty()),
            tags: self.genres.into_iter().filter_map(|g| g.name).collect(),
            cover_image_url: self.image_main.and_then(|i| i.to_url()),
            preview_image_urls: self
                .image_samples
                .into_iter()
                .filter_map(|i| i.to_url())
                .collect(),
            file_type: self.file_type_string,
            file_size_bytes: self.contents_file_size,
            dl_count: self.dl_count,
            rate_average: self.rate_average_2dp,
            rate_count: self.rate_count,
            price_jpy: self.price,
            work_type: self.work_type,
            work_type_label: self.work_type_string,
        }
    }
}

fn parse_dlsite_date(s: &str) -> Option<NaiveDate> {
    // regist_date is "YYYY-MM-DD HH:MM:SS" — keep the date portion.
    let date_part = s.split_whitespace().next()?;
    NaiveDate::parse_from_str(date_part, "%Y-%m-%d").ok()
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

    #[test]
    fn builds_cover_url_for_each_prefix() {
        assert_eq!(
            cover_url_for_workno("RJ01402281").as_deref(),
            Some("https://img.dlsite.jp/modpub/images2/work/doujin/RJ01403000/RJ01402281_img_main.jpg")
        );
        assert_eq!(
            cover_url_for_workno("RJ123456").as_deref(),
            Some("https://img.dlsite.jp/modpub/images2/work/doujin/RJ124000/RJ123456_img_main.jpg")
        );
        assert_eq!(
            cover_url_for_workno("VJ015501").as_deref(),
            Some("https://img.dlsite.jp/modpub/images2/work/professional/VJ016000/VJ015501_img_main.jpg")
        );
        assert_eq!(
            cover_url_for_workno("BJ437100").as_deref(),
            Some("https://img.dlsite.jp/modpub/images2/work/books/BJ438000/BJ437100_img_main.jpg")
        );
    }

    #[test]
    fn cover_url_buckets_exact_thousand_in_own_folder() {
        assert_eq!(
            cover_url_for_workno("RJ001000").as_deref(),
            Some("https://img.dlsite.jp/modpub/images2/work/doujin/RJ001000/RJ001000_img_main.jpg")
        );
        assert_eq!(
            cover_url_for_workno("RJ001001").as_deref(),
            Some("https://img.dlsite.jp/modpub/images2/work/doujin/RJ002000/RJ001001_img_main.jpg")
        );
    }
}