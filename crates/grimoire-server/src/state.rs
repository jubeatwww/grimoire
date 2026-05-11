use anyhow::Context;
use grimoire_app::storage::StorageRoot;
use sqlx::{postgres::PgPoolOptions, PgPool};
use std::time::Duration;

#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub library_root: StorageRoot,
}

impl AppState {
    pub async fn connect(database_url: &str, library_root: StorageRoot) -> anyhow::Result<Self> {
        let db = PgPoolOptions::new()
            .max_connections(5)
            .acquire_timeout(Duration::from_secs(5))
            .connect(database_url)
            .await
            .context(
                "failed to connect to PostgreSQL; check DATABASE_URL or GRIMOIRE_DATABASE_*",
            )?;

        sqlx::migrate!("./migrations")
            .run(&db)
            .await
            .context("failed to run database migrations")?;

        Ok(Self { db, library_root })
    }
}
