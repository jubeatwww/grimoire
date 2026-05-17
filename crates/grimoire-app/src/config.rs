use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub database_url: String,
    pub database_schema: String,
    pub library_root: PathBuf,
    pub staging_root: PathBuf,
    pub asset_cache_root: PathBuf,
    pub bind_addr: String,
}

impl AppConfig {
    pub fn from_env() -> Self {
        Self::from_lookup(|key| std::env::var(key).ok())
    }

    fn from_lookup(mut lookup: impl FnMut(&str) -> Option<String>) -> Self {
        let database_url = lookup_non_empty(&mut lookup, "DATABASE_URL")
            .unwrap_or_else(|| database_url_from_lookup(&mut lookup));

        Self {
            database_url,
            database_schema: lookup_or_default(&mut lookup, "GRIMOIRE_DATABASE_SCHEMA", "grimoire"),
            library_root: lookup_non_empty(&mut lookup, "GRIMOIRE_LIBRARY_ROOT")
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("/mnt/games")),
            staging_root: lookup_non_empty(&mut lookup, "GRIMOIRE_STAGING_ROOT")
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("./var/staging")),
            asset_cache_root: lookup_non_empty(&mut lookup, "GRIMOIRE_ASSET_CACHE_ROOT")
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("./var/assets")),
            bind_addr: lookup_non_empty(&mut lookup, "GRIMOIRE_BIND_ADDR")
                .unwrap_or_else(|| "127.0.0.1:3000".to_string()),
        }
    }
}

fn database_url_from_lookup(lookup: &mut impl FnMut(&str) -> Option<String>) -> String {
    let host = lookup_or_default(lookup, "GRIMOIRE_DATABASE_HOST", "localhost");
    let port = lookup_or_default(lookup, "GRIMOIRE_DATABASE_PORT", "5432");
    let name = lookup_or_default(lookup, "GRIMOIRE_DATABASE_NAME", "grimoire");
    let user = lookup_or_default(lookup, "GRIMOIRE_DATABASE_USER", "postgres");
    let password = lookup_or_default(lookup, "GRIMOIRE_DATABASE_PASSWORD", "postgres");

    format!(
        "postgres://{}:{}@{}:{}/{}",
        encode_url_component(&user),
        encode_url_component(&password),
        host,
        port,
        encode_url_component(&name)
    )
}

fn lookup_or_default(
    lookup: &mut impl FnMut(&str) -> Option<String>,
    key: &str,
    default: &str,
) -> String {
    lookup_non_empty(lookup, key).unwrap_or_else(|| default.to_string())
}

fn lookup_non_empty(lookup: &mut impl FnMut(&str) -> Option<String>, key: &str) -> Option<String> {
    lookup(key).filter(|value| !value.is_empty())
}

fn encode_url_component(value: &str) -> String {
    let mut encoded = String::with_capacity(value.len());
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                encoded.push(byte as char);
            }
            _ => {
                use std::fmt::Write;
                write!(&mut encoded, "%{byte:02X}").expect("writing to string cannot fail");
            }
        }
    }
    encoded
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_database_url_from_grimoire_database_credentials() {
        let config = AppConfig::from_lookup(|key| match key {
            "GRIMOIRE_DATABASE_HOST" => Some("db.internal".to_string()),
            "GRIMOIRE_DATABASE_PORT" => Some("5433".to_string()),
            "GRIMOIRE_DATABASE_NAME" => Some("grimoire_test".to_string()),
            "GRIMOIRE_DATABASE_USER" => Some("app_user".to_string()),
            "GRIMOIRE_DATABASE_PASSWORD" => Some("s3cret".to_string()),
            _ => None,
        });

        assert_eq!(
            config.database_url,
            "postgres://app_user:s3cret@db.internal:5433/grimoire_test"
        );
    }

    #[test]
    fn database_url_overrides_grimoire_database_credentials() {
        let config = AppConfig::from_lookup(|key| match key {
            "DATABASE_URL" => Some("postgres://custom:secret@db/custom_db".to_string()),
            "GRIMOIRE_DATABASE_USER" => Some("ignored".to_string()),
            "GRIMOIRE_DATABASE_PASSWORD" => Some("ignored".to_string()),
            _ => None,
        });

        assert_eq!(config.database_url, "postgres://custom:secret@db/custom_db");
    }
}
