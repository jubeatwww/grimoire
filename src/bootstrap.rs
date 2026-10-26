//! Reads the configuration, migrates the database, wires every adapter and use case, and
//! serves the API.

mod app_config;
mod wiring;

use std::path::Path;
use std::time::Duration;

use anyhow::Context;
use sqlx::postgres::PgPoolOptions;
use sqlx::{Executor, PgPool};
use tokio::net::TcpListener;
use tower_http::trace::{DefaultMakeSpan, DefaultOnRequest, DefaultOnResponse, TraceLayer};
use tracing::Level;
use tracing_subscriber::EnvFilter;

use app_config::AppConfig;

const DEFAULT_LOG_FILTER: &str = "grimoire=info,tower_http=info";

pub async fn run() -> anyhow::Result<()> {
    load_dotenv()?;
    tracing_subscriber::fmt()
        .with_env_filter(env_filter())
        .init();

    let config = AppConfig::from_env();
    let pool = connect(&config.database_url, &config.database_schema).await?;
    migrate(&pool).await?;
    ensure_asset_cache(&config.asset_cache_root);

    let app = wiring::wire(pool, config.library_root, config.asset_cache_root);
    // Settle move batches a crash or restart left half-done.
    match app.reconcile.execute().await {
        Ok(output) if output.reconciled == 0 => {}
        Ok(output) => tracing::warn!(
            batches = output.reconciled,
            "reconciled interrupted move batches"
        ),
        Err(e) => tracing::error!(error = ?e, "could not reconcile interrupted move batches"),
    }

    let router = app.router.layer(
        TraceLayer::new_for_http()
            .make_span_with(DefaultMakeSpan::new().level(Level::INFO))
            .on_request(DefaultOnRequest::new().level(Level::INFO))
            .on_response(DefaultOnResponse::new().level(Level::INFO)),
    );
    let listener = TcpListener::bind(&config.bind_addr).await?;
    tracing::info!("listening on {}", config.bind_addr);
    axum::serve(listener, router).await?;
    Ok(())
}

async fn connect(database_url: &str, schema: &str) -> anyhow::Result<PgPool> {
    let set_search_path = format!("SET search_path TO {schema}, public;");
    PgPoolOptions::new()
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
        .context("failed to connect to PostgreSQL; check DATABASE_URL or GRIMOIRE_DATABASE_*")
}

/// One migration sequence for the whole crate; the history of the earlier layout carries on.
async fn migrate(pool: &PgPool) -> anyhow::Result<()> {
    sqlx::migrate!("./migrations")
        .run(pool)
        .await
        .context("failed to run database migrations")
}

/// Uploads would fail until the asset directory exists.
fn ensure_asset_cache(path: &Path) {
    if let Err(e) = std::fs::create_dir_all(path) {
        tracing::warn!(error = %e, path = %path.display(),
            "failed to create asset cache directory; uploads will fail until it exists");
    }
}

fn load_dotenv() -> anyhow::Result<()> {
    match dotenvy::dotenv() {
        Ok(_) => Ok(()),
        Err(error) if error.not_found() => Ok(()),
        Err(error) => Err(error.into()),
    }
}

fn env_filter() -> EnvFilter {
    EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(DEFAULT_LOG_FILTER))
}
