use anyhow::Context;
use grimoire_app::storage::StorageRoot;
use sqlx::{postgres::PgPoolOptions, Executor, PgPool};
use std::time::Duration;

#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub library_root: StorageRoot,
}

impl AppState {
    pub async fn connect(
        database_url: &str,
        database_schema: &str,
        library_root: StorageRoot,
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

        Ok(Self { db, library_root })
    }
}
