use crate::metadata_source::{null_to_default, MetadataSource, ProductDetail};
use chrono::NaiveDate;
use grimoire_domain::metadata::MetadataCandidate;
use regex::Regex;
use reqwest::Client;
use serde::Deserialize;
use std::sync::LazyLock;
use uuid::Uuid;

const APPDETAILS_URL: &str = "https://store.steampowered.com/api/appdetails";
const SEARCH_URL: &str = "https://steamcommunity.com/actions/SearchApps";

static APP_ID_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)store\.steampowered\.com/app/(\d+)").unwrap());

pub struct SteamSource {
    client: Client,
}

impl SteamSource {
    pub fn new() -> Self {
        Self {
            client: Client::builder()
                .user_agent("grimoire/0.1")
                .build()
                .expect("failed to build HTTP client"),
        }
    }

    /// Extract a Steam app id from a store URL. Bare numbers are intentionally
    /// not accepted — too ambiguous against VNDB / DLsite numeric ids.
    pub fn extract_app_id(input: &str) -> Option<String> {
        APP_ID_RE.captures(input).map(|c| c[1].to_string())
    }

    pub async fn fetch_product_detail(
        &self,
        app_id: &str,
    ) -> anyhow::Result<Option<ProductDetail>> {
        // Traditional Chinese: gives Chinese descriptions / genres / categories
        // when available, falling back to English otherwise. Date format is
        // localised too — see parse_steam_date for the Chinese pattern.
        let url = format!("{APPDETAILS_URL}?appids={app_id}&l=tchinese&cc=tw");
        let resp = self.client.get(&url).send().await?;
        if !resp.status().is_success() {
            return Ok(None);
        }
        let body: serde_json::Value = resp.json().await?;
        let Some(entry) = body.get(app_id) else { return Ok(None); };
        if !entry.get("success").and_then(|v| v.as_bool()).unwrap_or(false) {
            return Ok(None);
        }
        let Some(data_value) = entry.get("data") else { return Ok(None); };
        let detail: AppDetails = match serde_json::from_value(data_value.clone()) {
            Ok(d) => d,
            Err(_) => return Ok(None),
        };
        Ok(Some(detail.into_detail()))
    }
}

#[async_trait::async_trait]
impl MetadataSource for SteamSource {
    async fn search(&self, query: &str) -> anyhow::Result<Vec<MetadataCandidate>> {
        // If the query is itself a Steam URL, do an exact lookup; suggest gets noisy.
        if let Some(id) = Self::extract_app_id(query) {
            if let Ok(Some(detail)) = self.fetch_product_detail(&id).await {
                return Ok(vec![candidate_from_detail(&id, &detail, query, 1)]);
            }
            return Ok(Vec::new());
        }

        let url = format!("{SEARCH_URL}/{}", urlencoding::encode(query));
        let resp = self.client.get(&url).send().await?;
        if !resp.status().is_success() {
            return Ok(Vec::new());
        }
        let hits: Vec<SearchHit> = resp.json().await.unwrap_or_default();
        let candidates = hits
            .into_iter()
            .enumerate()
            .take(8)
            .map(|(i, h)| {
                let source_url = format!("https://store.steampowered.com/app/{}/", h.appid);
                MetadataCandidate {
                    id: Uuid::new_v4(),
                    source_name: "steam".to_string(),
                    source_work_id: h.appid.clone(),
                    source_url,
                    query_used: query.to_string(),
                    rank: (i + 1) as i32,
                    title: h.name.clone(),
                    circle: None,
                    cover_url: h.logo.or(h.icon),
                    normalized_payload: serde_json::json!({
                        "title": h.name,
                        "work_type": "GAME",
                        "work_type_label": "Steam",
                    }),
                }
            })
            .collect();
        Ok(candidates)
    }
}

fn candidate_from_detail(
    app_id: &str,
    detail: &ProductDetail,
    query: &str,
    rank: i32,
) -> MetadataCandidate {
    MetadataCandidate {
        id: Uuid::new_v4(),
        source_name: "steam".to_string(),
        source_work_id: app_id.to_string(),
        source_url: format!("https://store.steampowered.com/app/{app_id}/"),
        query_used: query.to_string(),
        rank,
        title: detail.work_name.clone().unwrap_or_else(|| app_id.to_string()),
        circle: detail.maker_name.clone(),
        cover_url: detail.cover_image_url.clone(),
        normalized_payload: serde_json::json!({
            "title": detail.work_name,
            "circle": detail.maker_name,
            "intro_s": detail.description.as_deref().map(truncate),
            "work_type": detail.work_type,
            "work_type_label": detail.work_type_label,
        }),
    }
}

fn truncate(s: &str) -> String {
    if s.chars().count() <= 220 {
        s.to_string()
    } else {
        let cap: String = s.chars().take(220).collect();
        format!("{cap}…")
    }
}

#[derive(Debug, Deserialize)]
struct SearchHit {
    appid: String,
    name: String,
    icon: Option<String>,
    logo: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AppDetails {
    name: Option<String>,
    short_description: Option<String>,
    about_the_game: Option<String>,
    #[serde(default, deserialize_with = "null_to_default")]
    developers: Vec<String>,
    release_date: Option<ReleaseDate>,
    #[serde(default, deserialize_with = "null_to_default")]
    genres: Vec<NamedEntry>,
    header_image: Option<String>,
    #[serde(default, deserialize_with = "null_to_default")]
    screenshots: Vec<Screenshot>,
    recommendations: Option<Recommendations>,
    #[serde(default, deserialize_with = "null_to_default")]
    categories: Vec<NamedEntry>,
}

#[derive(Debug, Deserialize)]
struct ReleaseDate {
    coming_soon: Option<bool>,
    date: Option<String>,
}

#[derive(Debug, Deserialize)]
struct NamedEntry {
    description: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Screenshot {
    path_full: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Recommendations {
    total: Option<i32>,
}

impl AppDetails {
    fn into_detail(self) -> ProductDetail {
        // about_the_game is HTML-heavy; short_description is clean text and
        // already trimmed to a concise paragraph. Prefer it.
        let description = self
            .short_description
            .filter(|s| !s.is_empty())
            .or_else(|| self.about_the_game.map(|h| html_to_text(&h)));
        let release_date = self
            .release_date
            .as_ref()
            .filter(|r| !r.coming_soon.unwrap_or(false))
            .and_then(|r| r.date.as_deref())
            .and_then(parse_steam_date);
        let mut tags: Vec<String> = self
            .genres
            .into_iter()
            .filter_map(|g| g.description)
            .collect();
        // Append non-trivial categories (skip empty noise).
        for c in self.categories.into_iter().filter_map(|c| c.description) {
            if !tags.iter().any(|t| t == &c) {
                tags.push(c);
            }
        }
        let preview = self
            .screenshots
            .into_iter()
            .filter_map(|s| s.path_full)
            .collect();
        ProductDetail {
            work_name: self.name,
            maker_name: self.developers.into_iter().next(),
            description,
            release_date,
            series: None,
            tags,
            cover_image_url: self.header_image,
            preview_image_urls: preview,
            file_type: None,
            file_size_bytes: None,
            dl_count: None,
            rate_average: None,
            rate_count: self.recommendations.and_then(|r| r.total),
            price_jpy: None,
            work_type: Some("STEAM".to_string()),
            work_type_label: Some("Steam".to_string()),
        }
    }
}

static CJK_DATE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(\d{4})\s*年\s*(\d{1,2})\s*月\s*(\d{1,2})\s*日").unwrap());

fn parse_steam_date(s: &str) -> Option<NaiveDate> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    // CJK format ("2022 年 12 月 18 日") — Chinese/Japanese Steam locales.
    if let Some(c) = CJK_DATE_RE.captures(s) {
        let y: i32 = c[1].parse().ok()?;
        let m: u32 = c[2].parse().ok()?;
        let d: u32 = c[3].parse().ok()?;
        return NaiveDate::from_ymd_opt(y, m, d);
    }
    // English-ish formats — fallback for cases where the locale flips back.
    for fmt in ["%b %d, %Y", "%d %b, %Y", "%B %d, %Y"] {
        if let Ok(d) = NaiveDate::parse_from_str(s, fmt) {
            return Some(d);
        }
    }
    None
}

/// Bare-bones HTML → text. Steam's about_the_game is full of <br>, <h1>, <ul>,
/// <span class="bb_img_ctn">…<img/></span> markers; this turns it into a flat
/// paragraph good enough to display.
fn html_to_text(html: &str) -> String {
    static IMG_CTN_RE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r#"(?is)<span class="bb_img_ctn">.*?</span>"#).unwrap());
    static BR_RE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"(?i)<br\s*/?>|</p>|</li>|</h\d>").unwrap());
    static TAG_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"<[^>]+>").unwrap());
    static MULTI_NL_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\n{3,}").unwrap());

    let no_inline_media = IMG_CTN_RE.replace_all(html, "");
    let with_newlines = BR_RE.replace_all(&no_inline_media, "\n");
    let no_tags = TAG_RE.replace_all(&with_newlines, "");
    let decoded = no_tags
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&nbsp;", " ");
    MULTI_NL_RE
        .replace_all(&decoded, "\n\n")
        .trim()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_app_id_from_store_url() {
        assert_eq!(
            SteamSource::extract_app_id("https://store.steampowered.com/app/2022180/Miss_Neko_3/?l=tchinese")
                .as_deref(),
            Some("2022180"),
        );
        assert_eq!(
            SteamSource::extract_app_id("https://store.steampowered.com/app/440")
                .as_deref(),
            Some("440"),
        );
        assert_eq!(SteamSource::extract_app_id("vndb.org/v17").as_deref(), None);
        assert_eq!(SteamSource::extract_app_id("2022180").as_deref(), None);
    }

    #[test]
    fn parses_common_steam_date_formats() {
        assert_eq!(
            parse_steam_date("Dec 18, 2022"),
            Some(NaiveDate::from_ymd_opt(2022, 12, 18).unwrap()),
        );
        assert_eq!(
            parse_steam_date("18 Dec, 2022"),
            Some(NaiveDate::from_ymd_opt(2022, 12, 18).unwrap()),
        );
        assert_eq!(
            parse_steam_date("2022 年 12 月 18 日"),
            Some(NaiveDate::from_ymd_opt(2022, 12, 18).unwrap()),
        );
        assert_eq!(
            parse_steam_date("2022年5月3日"),
            Some(NaiveDate::from_ymd_opt(2022, 5, 3).unwrap()),
        );
        assert_eq!(parse_steam_date("Coming Soon"), None);
        assert_eq!(parse_steam_date("即將推出"), None);
        assert_eq!(parse_steam_date(""), None);
    }

    #[test]
    fn html_to_text_strips_and_normalises() {
        let in_ = r#"<h1>Title</h1><p>Hello <strong>world</strong>.</p><br><ul><li>One</li><li>Two</li></ul><span class="bb_img_ctn"><img src="x.jpg"/></span><br>End."#;
        let out = html_to_text(in_);
        assert!(out.contains("Title"));
        assert!(out.contains("Hello world."));
        assert!(out.contains("One"));
        assert!(out.contains("Two"));
        assert!(out.contains("End."));
        assert!(!out.contains("<"));
        assert!(!out.contains("bb_img_ctn"));
    }
}
