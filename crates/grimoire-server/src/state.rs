use anyhow::Context;
use grimoire_app::storage::StorageRoot;
use sqlx::{postgres::PgPoolOptions, Executor, PgPool};
use std::path::PathBuf;
use std::time::Duration;

#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub library_root: StorageRoot,
    pub asset_cache_root: PathBuf,
}

impl AppState {
    pub async fn connect(
        database_url: &str,
        database_schema: &str,
        library_root: StorageRoot,
        asset_cache_root: PathBuf,
    ) -> anyhow::Result<Self> {
        let set_search_path = format!("SET search_path TO {database_schema}, public;");
        let db = PgPoolOptions::new()
            .max_connections(5)
            .acquire_timeout(Duration::from_secs(5))
            .after_connect(move |conn, _meta| {
                let sql = set_search_path.clone();
                Box::pin(async move {
                    conn.execute(sql.as_str()).await?;
                    Ok(())
                })
            })
            .connect(database_url)
            .await
            .context(
                "failed to connect to PostgreSQL; check DATABASE_URL or GRIMOIRE_DATABASE_*",
            )?;

        sqlx::migrate!("./migrations")
            .run(&db)
            .await
            .context("failed to run database migrations")?;

        // Ensure the asset directory exists so uploads don't 500 on first use.
        if let Err(e) = std::fs::create_dir_all(&asset_cache_root) {
            tracing::warn!(error = %e, path = %asset_cache_root.display(),
                "failed to create asset cache directory; uploads will fail until it exists");
        }

        Ok(Self {
            db,
            library_root,
            asset_cache_root,
        })
    }
}
