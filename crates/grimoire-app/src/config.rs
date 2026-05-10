use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub database_url: String,
    pub library_root: PathBuf,
    pub staging_root: PathBuf,
    pub asset_cache_root: PathBuf,
    pub bind_addr: String,
}

impl AppConfig {
    pub fn from_env() -> Self {
        Self {
            database_url: std::env::var("DATABASE_URL")
                .unwrap_or_else(|_| "postgres://postgres:postgres@localhost/grimoire".to_string()),
            library_root: std::env::var("GRIMOIRE_LIBRARY_ROOT")
                .map(PathBuf::from)
                .unwrap_or_else(|_| PathBuf::from("/mnt/games")),
            staging_root: std::env::var("GRIMOIRE_STAGING_ROOT")
                .map(PathBuf::from)
                .unwrap_or_else(|_| PathBuf::from("./var/staging")),
            asset_cache_root: std::env::var("GRIMOIRE_ASSET_CACHE_ROOT")
                .map(PathBuf::from)
                .unwrap_or_else(|_| PathBuf::from("./var/assets")),
            bind_addr: std::env::var("GRIMOIRE_BIND_ADDR")
                .unwrap_or_else(|_| "127.0.0.1:3000".to_string()),
        }
    }
}
