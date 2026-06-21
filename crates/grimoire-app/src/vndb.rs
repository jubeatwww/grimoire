use crate::metadata_source::{null_to_default, MetadataSource, ProductDetail};
use chrono::NaiveDate;
use grimoire_domain::metadata::MetadataCandidate;
use regex::Regex;
use reqwest::Client;
use serde::Deserialize;
use std::sync::LazyLock;
use uuid::Uuid;

const VNDB_API: &str = "https://api.vndb.org/kana/vn";

static VNDB_ID_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\bv(\d{1,7})\b").unwrap());

pub struct VndbSource {
    client: Client,
}

impl VndbSource {
    pub fn new() -> Self {
        Self {
            client: Client::builder()
                .user_agent("grimoire/0.1")
                .build()
                .expect("failed to build HTTP client"),
        }
    }

    /// Detect a VNDB id (`v17`) inside an arbitrary input — bare token or vndb.org URL.
    pub fn extract_id(input: &str) -> Option<String> {
        // Prefer the URL form when the input contains vndb.org — avoids matching
        // an unrelated "v123" elsewhere in the string.
        if let Some(after) = input.find("vndb.org/") {
            let tail = &input[after + "vndb.org/".len()..];
            let id_end = tail
                .find(|c: char| !c.is_ascii_alphanumeric())
                .unwrap_or(tail.len());
            let id = &tail[..id_end];
            if id.len() > 1
                && id.starts_with('v')
                && id[1..].chars().all(|c| c.is_ascii_digit())
            {
                return Some(id.to_lowercase());
            }
        }
        VNDB_ID_RE.find(input).map(|m| m.as_str().to_lowercase())
    }

    pub async fn fetch_product_detail(
        &self,
        vndb_id: &str,
    ) -> anyhow::Result<Option<ProductDetail>> {
        let body = serde_json::json!({
            "filters": ["id", "=", vndb_id],
            "fields": "title,olang,titles.lang,titles.title,titles.main,\
                       description,released,rating,votecount,image.url,\
                       tags.name,tags.rating,tags.spoiler,developers.name,screenshots.url",
        });
        let resp = self.client.post(VNDB_API).json(&body).send().await?;
        if !resp.status().is_success() {
            return Ok(None);
        }
        let parsed: VndbList<VnFull> = resp.json().await?;
        Ok(parsed.results.into_iter().next().map(VnFull::into_detail))
    }
}

#[async_trait::async_trait]
impl MetadataSource for VndbSource {
    async fn search(&self, query: &str) -> anyhow::Result<Vec<MetadataCandidate>> {
        let fields = "id,title,olang,titles.lang,titles.title,titles.main,\
                      description,released,image.url,developers.name";
        let body = if let Some(id) = Self::extract_id(query) {
            serde_json::json!({
                "filters": ["id", "=", id],
                "fields": fields,
                "results": 1,
            })
        } else {
            serde_json::json!({
                "filters": ["search", "=", query],
                "fields": fields,
                "results": 10,
                "sort": "searchrank",
            })
        };
        let resp = self.client.post(VNDB_API).json(&body).send().await?;
        if !resp.status().is_success() {
            return Ok(Vec::new());
        }
        let parsed: VndbList<VnLite> = resp.json().await?;
        let candidates = parsed
            .results
            .into_iter()
            .enumerate()
            .map(|(i, vn)| {
                let source_url = format!("https://vndb.org/{}", vn.id);
                let circle = vn.developers.first().map(|d| d.name.clone());
                let intro_full = vn.description.as_deref().map(strip_bbcode);
                let intro_s = intro_full.as_deref().map(truncate_blurb);
                let title = pick_original_title(&vn.title, &vn.titles, vn.olang.as_deref());
                MetadataCandidate {
                    id: Uuid::new_v4(),
                    source_name: "vndb".to_string(),
                    source_work_id: vn.id.clone(),
                    source_url,
                    query_used: query.to_string(),
                    rank: (i + 1) as i32,
                    title: title.clone(),
                    circle: circle.clone(),
                    cover_url: vn.image.and_then(|i| i.url),
                    normalized_payload: serde_json::json!({
                        "title": title,
                        "circle": circle,
                        "released": vn.released,
                        "intro_s": intro_s,
                        "work_type": "ADV",
                        "work_type_label": "視覺小說",
                    }),
                }
            })
            .collect();
        Ok(candidates)
    }
}

fn truncate_blurb(s: &str) -> String {
    if s.chars().count() <= 220 {
        s.to_string()
    } else {
        let cap: String = s.chars().take(220).collect();
        format!("{cap}…")
    }
}

fn parse_date(s: Option<&str>) -> Option<NaiveDate> {
    let s = s?.trim();
    if s.is_empty() || s.eq_ignore_ascii_case("TBA") {
        return None;
    }
    if let Ok(d) = NaiveDate::parse_from_str(s, "%Y-%m-%d") {
        return Some(d);
    }
    if let Ok(d) = NaiveDate::parse_from_str(&format!("{s}-01"), "%Y-%m-%d") {
        return Some(d);
    }
    if let Ok(d) = NaiveDate::parse_from_str(&format!("{s}-01-01"), "%Y-%m-%d") {
        return Some(d);
    }
    None
}

/// Drop [spoiler]…[/spoiler] entirely (avoid leaking surprises); unwrap
/// [url=x]label[/url] to its label; strip inline formatting tags.
fn strip_bbcode(input: &str) -> String {
    static SPOILER_RE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"(?is)\[spoiler\].*?\[/spoiler\]").unwrap());
    static URL_RE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"(?i)\[url=[^\]]*\](.*?)\[/url\]").unwrap());
    static SIMPLE_RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(?i)\[/?(?:b|i|u|s|sub|sup|code|quote|center)\]").unwrap()
    });

    let no_spoiler = SPOILER_RE.replace_all(input, "");
    let no_urls = URL_RE.replace_all(&no_spoiler, "$1");
    SIMPLE_RE.replace_all(&no_urls, "").into_owned()
}

#[derive(Debug, Deserialize)]
struct VndbList<T> {
    results: Vec<T>,
}

#[derive(Debug, Deserialize)]
struct VnLite {
    id: String,
    title: String,
    olang: Option<String>,
    #[serde(default, deserialize_with = "null_to_default")]
    titles: Vec<VnTitleEntry>,
    description: Option<String>,
    released: Option<String>,
    image: Option<VndbImage>,
    #[serde(default, deserialize_with = "null_to_default")]
    developers: Vec<VndbDeveloper>,
}

#[derive(Debug, Deserialize)]
struct VnFull {
    title: String,
    olang: Option<String>,
    #[serde(default, deserialize_with = "null_to_default")]
    titles: Vec<VnTitleEntry>,
    description: Option<String>,
    released: Option<String>,
    rating: Option<f32>,
    votecount: Option<i32>,
    image: Option<VndbImage>,
    #[serde(default, deserialize_with = "null_to_default")]
    tags: Vec<VndbTag>,
    #[serde(default, deserialize_with = "null_to_default")]
    developers: Vec<VndbDeveloper>,
    #[serde(default, deserialize_with = "null_to_default")]
    screenshots: Vec<VndbImage>,
}

#[derive(Debug, Deserialize)]
struct VnTitleEntry {
    lang: String,
    title: String,
    #[serde(default)]
    main: bool,
}

/// Pick the title in the work's original language. VNDB's top-level `title` is
/// often the English/preferred version, which loses the native title for non-EN
/// works (Chinese indie sims show as English, etc.). Fall back to the top-level
/// when no original-language entry exists.
fn pick_original_title(
    top_level: &str,
    titles: &[VnTitleEntry],
    olang: Option<&str>,
) -> String {
    if let Some(ol) = olang {
        if let Some(t) = titles.iter().filter(|t| t.lang == ol).find(|t| t.main) {
            return t.title.clone();
        }
        if let Some(t) = titles.iter().find(|t| t.lang == ol) {
            return t.title.clone();
        }
    }
    top_level.to_string()
}

#[derive(Debug, Deserialize)]
struct VndbImage {
    url: Option<String>,
}

#[derive(Debug, Deserialize)]
struct VndbTag {
    name: String,
    rating: f32,
    spoiler: i32,
}

#[derive(Debug, Deserialize)]
struct VndbDeveloper {
    name: String,
}

impl VnFull {
    fn into_detail(self) -> ProductDetail {
        let description = self.description.as_deref().map(strip_bbcode);
        let circle = self.developers.into_iter().next().map(|d| d.name);
        let title = pick_original_title(&self.title, &self.titles, self.olang.as_deref());
        let mut tags: Vec<(String, f32)> = self
            .tags
            .into_iter()
            .filter(|t| t.spoiler == 0 && t.rating >= 1.5)
            .map(|t| (t.name, t.rating))
            .collect();
        tags.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        let tag_names: Vec<String> = tags.into_iter().take(30).map(|(n, _)| n).collect();
        let preview_urls: Vec<String> = self
            .screenshots
            .into_iter()
            .filter_map(|s| s.url)
            .collect();
        ProductDetail {
            work_name: Some(title),
            maker_name: circle,
            description,
            release_date: parse_date(self.released.as_deref()),
            series: None,
            tags: tag_names,
            cover_image_url: self.image.and_then(|i| i.url),
            preview_image_urls: preview_urls,
            file_type: None,
            file_size_bytes: None,
            dl_count: None,
            rate_average: self.rating.map(|r| r / 10.0),
            rate_count: self.votecount,
            price_jpy: None,
            work_type: Some("ADV".to_string()),
            work_type_label: Some("視覺小說".to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_vndb_id_from_token() {
        assert_eq!(VndbSource::extract_id("v17").as_deref(), Some("v17"));
        assert_eq!(VndbSource::extract_id("V2002").as_deref(), Some("v2002"));
        assert_eq!(
            VndbSource::extract_id("steins gate v2002 demo").as_deref(),
            Some("v2002"),
        );
        assert_eq!(VndbSource::extract_id("nothing here").as_deref(), None);
    }

    #[test]
    fn detects_vndb_id_from_url() {
        assert_eq!(
            VndbSource::extract_id("https://vndb.org/v17").as_deref(),
            Some("v17"),
        );
        assert_eq!(
            VndbSource::extract_id("https://vndb.org/v2002/screenshots").as_deref(),
            Some("v2002"),
        );
    }

    #[test]
    fn strips_bbcode_tags() {
        assert_eq!(strip_bbcode("[b]bold[/b]"), "bold");
        assert_eq!(strip_bbcode("[url=https://x.com]label[/url] end"), "label end");
        assert_eq!(strip_bbcode("safe [spoiler]surprise[/spoiler] tail"), "safe  tail");
        assert_eq!(strip_bbcode("[i]a[/i] [b]b[/b]"), "a b");
    }

    #[test]
    fn parses_partial_release_dates() {
        assert_eq!(
            parse_date(Some("2009-10-15")),
            Some(NaiveDate::from_ymd_opt(2009, 10, 15).unwrap()),
        );
        assert_eq!(
            parse_date(Some("2009-10")),
            Some(NaiveDate::from_ymd_opt(2009, 10, 1).unwrap()),
        );
        assert_eq!(
            parse_date(Some("2009")),
            Some(NaiveDate::from_ymd_opt(2009, 1, 1).unwrap()),
        );
        assert_eq!(parse_date(Some("TBA")), None);
        assert_eq!(parse_date(None), None);
    }
}
