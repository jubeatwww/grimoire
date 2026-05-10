mod state;

use hgame_app::config::AppConfig;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let config = AppConfig::from_env();
    let _state = state::AppState::connect(&config.database_url).await?;
    tracing::info!("hgame-server initialized");
    Ok(())
}
