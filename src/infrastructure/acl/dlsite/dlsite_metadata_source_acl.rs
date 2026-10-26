use std::sync::LazyLock;

use async_trait::async_trait;
use chrono::NaiveDate;
use regex::Regex;
use reqwest::Client;
use serde::Deserialize;

use super::super::{null_to_default, unavailable};
use crate::domain::game_work::WorkDetail;
use crate::domain::metadata_source::{
    MetadataProvider, MetadataSource, MetadataSourceError, ProductDetail, SourceMatch,
};

const DLSITE_IMG_BASE: &str = "https://img.dlsite.jp/";

static RJ_CODE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)(RJ|VJ|BJ)\d{6,8}").unwrap());

/// DLsite: the product API for detail, the suggest endpoint for search.
pub(crate) struct DlsiteMetadataSourceAcl {
    client: Client,
}

impl DlsiteMetadataSourceAcl {
    pub(crate) fn new() -> Self {
        Self {
            client: Client::builder()
                .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64)")
                .build()
                .expect("failed to build HTTP client"),
        }
    }

    fn work_id(input: &str) -> Option<String> {
        RJ_CODE_RE.find(input).map(|m| m.as_str().to_uppercase())
    }

    async fn suggest(&self, term: &str) -> Result<DlsiteSuggestResponse, reqwest::Error> {
        let url = format!(
            "https://www.dlsite.com/suggest/?term={}&site=adult-jp&time=1&touch=0&_=1",
            urlencoding::encode(term)
        );
        let resp = self.client.get(&url).send().await?;
        if !resp.status().is_success() {
            return Ok(DlsiteSuggestResponse::default());
        }
        let body = resp.text().await?;
        // The response may be JSONP (with a callback wrapper) or plain JSON.
        let json_str = if let Some(start) = body.find('(') {
            &body[start + 1..body.len().saturating_sub(1)]
        } else {
            &body
        };
        Ok(serde_json::from_str(json_str).unwrap_or_default())
    }
}

#[async_trait]
impl MetadataSource for DlsiteMetadataSourceAcl {
    fn provider(&self) -> MetadataProvider {
        MetadataProvider::Dlsite
    }

    fn recognize(&self, input: &str) -> Option<String> {
        Self::work_id(input)
    }

    async fn search(&self, query: &str) -> Result<Vec<SourceMatch>, MetadataSourceError> {
        // RJ codes get exact-match candidates from suggest; otherwise it's a keyword search.
        let term = Self::work_id(query).unwrap_or_else(|| query.to_string());
        let suggest = self.suggest(&term).await.map_err(unavailable)?;
        let matches = suggest
            .work
            .into_iter()
            // Drop delisted ("ana") works: their pages 404 and image URLs don't resolve.
            .filter(|item| !item.is_ana.unwrap_or(false))
            .enumerate()
            .map(|(i, item)| {
                let work_id = item.workno.unwrap_or_default();
                let cover_url = cover_url_for_workno(&work_id);
                SourceMatch::new(work_id, rank(i), item.work_name.unwrap_or_default())
                    .with_circle(item.maker_name)
                    .with_cover_url(cover_url)
                    .with_work_type(item.work_type, None)
                    .with_intro(item.intro_s)
            })
            .collect();
        Ok(matches)
    }

    async fn find_detail(
        &self,
        external_id: &str,
    ) -> Result<Option<ProductDetail>, MetadataSourceError> {
        let url = format!("https://www.dlsite.com/maniax/api/=/product.json?workno={external_id}");
        let resp = self.client.get(&url).send().await.map_err(unavailable)?;
        if !resp.status().is_success() {
            return Ok(None);
        }
        let mut items: Vec<DlsiteProduct> = resp.json().await.map_err(unavailable)?;
        Ok(items.pop().map(ProductDetail::from))
    }
}

fn rank(index: usize) -> i32 {
    i32::try_from(index + 1).unwrap_or(i32::MAX)
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
struct DlsiteSuggestResponse {
    #[serde(default, deserialize_with = "null_to_default")]
    work: Vec<DlsiteSuggestWork>,
}

#[derive(Debug, Deserialize)]
struct DlsiteSuggestWork {
    work_name: Option<String>,
    workno: Option<String>,
    maker_name: Option<String>,
    work_type: Option<String>,
    intro_s: Option<String>,
    is_ana: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct DlsiteProduct {
    work_name: Option<String>,
    maker_name: Option<String>,
    intro: Option<String>,
    intro_s: Option<String>,
    regist_date: Option<String>,
    series_name: Option<String>,
    #[serde(default, deserialize_with = "null_to_default")]
    genres: Vec<DlsiteGenre>,
    image_main: Option<DlsiteImage>,
    #[serde(default, deserialize_with = "null_to_default")]
    image_samples: Vec<DlsiteImage>,
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
struct DlsiteGenre {
    name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct DlsiteImage {
    relative_url: Option<String>,
}

impl DlsiteImage {
    fn to_url(&self) -> Option<String> {
        self.relative_url
            .as_deref()
            .map(|u| format!("{DLSITE_IMG_BASE}{}", u.trim_start_matches('/')))
    }
}

impl From<DlsiteProduct> for ProductDetail {
    fn from(product: DlsiteProduct) -> Self {
        let detail = WorkDetail::default()
            .with_description(product.intro.or(product.intro_s))
            .with_release_date(product.regist_date.as_deref().and_then(parse_dlsite_date))
            .with_series(product.series_name.filter(|s| !s.is_empty()))
            .with_tags(product.genres.into_iter().filter_map(|g| g.name).collect())
            .with_cover_image_url(product.image_main.and_then(|i| i.to_url()))
            .with_preview_image_urls(
                product
                    .image_samples
                    .into_iter()
                    .filter_map(|i| i.to_url())
                    .collect(),
            )
            .with_file(product.file_type_string, product.contents_file_size)
            .with_sales(
                product.dl_count,
                product.rate_average_2dp,
                product.rate_count,
                product.price,
            )
            .with_work_type(product.work_type, product.work_type_string);
        ProductDetail::new(product.work_name, product.maker_name, detail)
    }
}

fn parse_dlsite_date(s: &str) -> Option<NaiveDate> {
    // regist_date is "YYYY-MM-DD HH:MM:SS"; keep the date portion.
    let date_part = s.split_whitespace().next()?;
    NaiveDate::parse_from_str(date_part, "%Y-%m-%d").ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognizes_rj_codes_in_file_names() {
        let acl = DlsiteMetadataSourceAcl::new();
        assert_eq!(
            acl.recognize("RJ01234567_some_game.zip"),
            Some("RJ01234567".to_string())
        );
        assert_eq!(
            acl.recognize("[Circle] Game Name (rj123456).rar"),
            Some("RJ123456".to_string())
        );
        assert_eq!(
            acl.recognize("VJ012345_title.zip"),
            Some("VJ012345".to_string())
        );
        assert_eq!(acl.recognize("no_code_here.zip"), None);
    }

    #[test]
    fn parses_suggest_json() {
        let json = r#"{"work":[{"work_name":"Test","workno":"RJ123456","maker_name":"Circle","work_type":"RPG"}],"maker":[],"reqtime":1}"#;
        let resp: DlsiteSuggestResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.work.len(), 1);
        assert_eq!(resp.work[0].workno.as_deref(), Some("RJ123456"));
        assert_eq!(resp.work[0].maker_name.as_deref(), Some("Circle"));
    }

    #[test]
    fn builds_cover_url_for_each_prefix() {
        assert_eq!(
            cover_url_for_workno("RJ01402281").as_deref(),
            Some(
                "https://img.dlsite.jp/modpub/images2/work/doujin/RJ01403000/RJ01402281_img_main.jpg"
            )
        );
        assert_eq!(
            cover_url_for_workno("RJ123456").as_deref(),
            Some("https://img.dlsite.jp/modpub/images2/work/doujin/RJ124000/RJ123456_img_main.jpg")
        );
        assert_eq!(
            cover_url_for_workno("VJ015501").as_deref(),
            Some(
                "https://img.dlsite.jp/modpub/images2/work/professional/VJ016000/VJ015501_img_main.jpg"
            )
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
