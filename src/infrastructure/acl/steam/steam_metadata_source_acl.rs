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

const APPDETAILS_URL: &str = "https://store.steampowered.com/api/appdetails";
const SEARCH_URL: &str = "https://steamcommunity.com/actions/SearchApps";

static APP_ID_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)store\.steampowered\.com/app/(\d+)").unwrap());

/// The Steam store's app details, and the community app search.
pub(crate) struct SteamMetadataSourceAcl {
    client: Client,
}

impl SteamMetadataSourceAcl {
    pub(crate) fn new() -> Self {
        Self {
            client: Client::builder()
                .user_agent("grimoire/0.1")
                .build()
                .expect("failed to build HTTP client"),
        }
    }

    /// A Steam app id from a store URL. Bare numbers are intentionally not accepted: too
    /// ambiguous against VNDB / DLsite numeric ids.
    fn app_id(input: &str) -> Option<String> {
        APP_ID_RE.captures(input).map(|c| c[1].to_string())
    }
}

#[async_trait]
impl MetadataSource for SteamMetadataSourceAcl {
    fn provider(&self) -> MetadataProvider {
        MetadataProvider::Steam
    }

    fn recognize(&self, input: &str) -> Option<String> {
        Self::app_id(input)
    }

    async fn search(&self, query: &str) -> Result<Vec<SourceMatch>, MetadataSourceError> {
        // If the query is itself a Steam URL, do an exact lookup; suggest gets noisy.
        if let Some(id) = Self::app_id(query) {
            if let Ok(Some(detail)) = self.find_detail(&id).await {
                return Ok(vec![match_from_detail(&id, &detail)]);
            }
            return Ok(Vec::new());
        }

        let url = format!("{SEARCH_URL}/{}", urlencoding::encode(query));
        let resp = self.client.get(&url).send().await.map_err(unavailable)?;
        if !resp.status().is_success() {
            return Ok(Vec::new());
        }
        let hits: Vec<SteamSearchHit> = resp.json().await.unwrap_or_default();
        let matches = hits
            .into_iter()
            .enumerate()
            .take(8)
            .map(|(i, h)| {
                SourceMatch::new(h.appid, i32::try_from(i + 1).unwrap_or(i32::MAX), h.name)
                    .with_cover_url(h.logo.or(h.icon))
                    .with_work_type(Some("GAME".to_string()), Some("Steam".to_string()))
            })
            .collect();
        Ok(matches)
    }

    async fn find_detail(
        &self,
        external_id: &str,
    ) -> Result<Option<ProductDetail>, MetadataSourceError> {
        // Traditional Chinese: gives Chinese descriptions / genres / categories when available,
        // falling back to English otherwise. The date format is localised too; see
        // parse_steam_date for the Chinese pattern.
        let url = format!("{APPDETAILS_URL}?appids={external_id}&l=tchinese&cc=tw");
        let resp = self.client.get(&url).send().await.map_err(unavailable)?;
        if !resp.status().is_success() {
            return Ok(None);
        }
        let body: serde_json::Value = resp.json().await.map_err(unavailable)?;
        let Some(entry) = body.get(external_id) else {
            return Ok(None);
        };
        if !entry
            .get("success")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
        {
            return Ok(None);
        }
        let Some(data_value) = entry.get("data") else {
            return Ok(None);
        };
        let Ok(detail) = serde_json::from_value::<SteamAppDetails>(data_value.clone()) else {
            return Ok(None);
        };
        Ok(Some(ProductDetail::from(detail)))
    }
}

fn match_from_detail(app_id: &str, detail: &ProductDetail) -> SourceMatch {
    let title = detail
        .work_name()
        .map_or_else(|| app_id.to_string(), str::to_owned);
    SourceMatch::new(app_id.to_string(), 1, title)
        .with_circle(detail.maker_name().map(str::to_owned))
        .with_cover_url(detail.detail().cover_image_url().map(str::to_owned))
        .with_work_type(
            detail.detail().work_type().map(str::to_owned),
            detail.detail().work_type_label().map(str::to_owned),
        )
        .with_intro(detail.detail().description().map(truncate))
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
struct SteamSearchHit {
    appid: String,
    name: String,
    icon: Option<String>,
    logo: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SteamAppDetails {
    name: Option<String>,
    short_description: Option<String>,
    about_the_game: Option<String>,
    #[serde(default, deserialize_with = "null_to_default")]
    developers: Vec<String>,
    release_date: Option<SteamReleaseDate>,
    #[serde(default, deserialize_with = "null_to_default")]
    genres: Vec<SteamNamedEntry>,
    header_image: Option<String>,
    #[serde(default, deserialize_with = "null_to_default")]
    screenshots: Vec<SteamScreenshot>,
    recommendations: Option<SteamRecommendations>,
    #[serde(default, deserialize_with = "null_to_default")]
    categories: Vec<SteamNamedEntry>,
}

#[derive(Debug, Deserialize)]
struct SteamReleaseDate {
    coming_soon: Option<bool>,
    date: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SteamNamedEntry {
    description: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SteamScreenshot {
    path_full: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SteamRecommendations {
    total: Option<i32>,
}

impl From<SteamAppDetails> for ProductDetail {
    fn from(product: SteamAppDetails) -> Self {
        // about_the_game is HTML-heavy; short_description is clean text and already trimmed to a
        // concise paragraph. Prefer it.
        let description = product
            .short_description
            .filter(|s| !s.is_empty())
            .or_else(|| product.about_the_game.map(|h| html_to_text(&h)));
        let release_date = product
            .release_date
            .as_ref()
            .filter(|r| !r.coming_soon.unwrap_or(false))
            .and_then(|r| r.date.as_deref())
            .and_then(parse_steam_date);
        let mut tags: Vec<String> = product
            .genres
            .into_iter()
            .filter_map(|g| g.description)
            .collect();
        // Append non-trivial categories (skip empty noise).
        for c in product.categories.into_iter().filter_map(|c| c.description) {
            if !tags.iter().any(|t| t == &c) {
                tags.push(c);
            }
        }
        let preview = product
            .screenshots
            .into_iter()
            .filter_map(|s| s.path_full)
            .collect();
        let detail = WorkDetail::default()
            .with_description(description)
            .with_release_date(release_date)
            .with_tags(tags)
            .with_cover_image_url(product.header_image)
            .with_preview_image_urls(preview)
            .with_sales(
                None,
                None,
                product.recommendations.and_then(|r| r.total),
                None,
            )
            .with_work_type(Some("STEAM".to_string()), Some("Steam".to_string()));
        ProductDetail::new(product.name, product.developers.into_iter().next(), detail)
    }
}

static CJK_DATE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(\d{4})\s*年\s*(\d{1,2})\s*月\s*(\d{1,2})\s*日").unwrap());

fn parse_steam_date(s: &str) -> Option<NaiveDate> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    // CJK format ("2022 年 12 月 18 日"): Chinese/Japanese Steam locales.
    if let Some(c) = CJK_DATE_RE.captures(s) {
        let y: i32 = c[1].parse().ok()?;
        let m: u32 = c[2].parse().ok()?;
        let d: u32 = c[3].parse().ok()?;
        return NaiveDate::from_ymd_opt(y, m, d);
    }
    // English-ish formats: fallback for cases where the locale flips back.
    for fmt in ["%b %d, %Y", "%d %b, %Y", "%B %d, %Y"] {
        if let Ok(d) = NaiveDate::parse_from_str(s, fmt) {
            return Some(d);
        }
    }
    None
}

/// Bare-bones HTML → text. Steam's about_the_game is full of <br>, <h1>, <ul>,
/// <span class="bb_img_ctn">…<img/></span> markers; this turns it into a flat paragraph good
/// enough to display.
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
    MULTI_NL_RE.replace_all(&decoded, "\n\n").trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognizes_app_id_from_store_url() {
        let acl = SteamMetadataSourceAcl::new();
        assert_eq!(
            acl.recognize("https://store.steampowered.com/app/2022180/Miss_Neko_3/?l=tchinese")
                .as_deref(),
            Some("2022180"),
        );
        assert_eq!(
            acl.recognize("https://store.steampowered.com/app/440")
                .as_deref(),
            Some("440")
        );
        assert_eq!(acl.recognize("vndb.org/v17").as_deref(), None);
        assert_eq!(acl.recognize("2022180").as_deref(), None);
    }

    #[test]
    fn parses_common_steam_date_formats() {
        assert_eq!(
            parse_steam_date("Dec 18, 2022"),
            Some(NaiveDate::from_ymd_opt(2022, 12, 18).unwrap())
        );
        assert_eq!(
            parse_steam_date("18 Dec, 2022"),
            Some(NaiveDate::from_ymd_opt(2022, 12, 18).unwrap())
        );
        assert_eq!(
            parse_steam_date("2022 年 12 月 18 日"),
            Some(NaiveDate::from_ymd_opt(2022, 12, 18).unwrap()),
        );
        assert_eq!(
            parse_steam_date("2022年5月3日"),
            Some(NaiveDate::from_ymd_opt(2022, 5, 3).unwrap())
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
