mod routes;
mod state;

use grimoire_app::config::AppConfig;
use tokio::net::TcpListener;
use tower_http::trace::{DefaultMakeSpan, DefaultOnRequest, DefaultOnResponse, TraceLayer};
use tracing::Level;
use tracing_subscriber::EnvFilter;

const DEFAULT_LOG_FILTER: &str = "grimoire_server=info,tower_http=info";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    load_dotenv()?;

    tracing_subscriber::fmt()
        .with_env_filter(env_filter())
        .init();

    let config = AppConfig::from_env();
    let state = state::AppState::connect(
        &config.database_url,
        &config.database_schema,
        grimoire_app::storage::StorageRoot::new(config.library_root.clone()),
    )
    .await?;
    let app = routes::router(state).layer(
        TraceLayer::new_for_http()
            .make_span_with(DefaultMakeSpan::new().level(Level::INFO))
            .on_request(DefaultOnRequest::new().level(Level::INFO))
            .on_response(DefaultOnResponse::new().level(Level::INFO)),
    );

    let listener = TcpListener::bind(&config.bind_addr).await?;
    tracing::info!("listening on {}", config.bind_addr);
    axum::serve(listener, app).await?;
    Ok(())
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        env, fs,
        sync::{Mutex, OnceLock},
        time::{SystemTime, UNIX_EPOCH},
    };

    #[test]
    fn loads_dotenv_from_current_directory() {
        static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        let _guard = ENV_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap();

        let original_dir = env::current_dir().unwrap();
        let original_value = env::var("GRIMOIRE_DOTENV_TEST").ok();
        let test_dir = env::temp_dir().join(format!(
            "grimoire-dotenv-test-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));

        fs::create_dir(&test_dir).unwrap();
        fs::write(test_dir.join(".env"), "GRIMOIRE_DOTENV_TEST=loaded\n").unwrap();
        env::remove_var("GRIMOIRE_DOTENV_TEST");
        env::set_current_dir(&test_dir).unwrap();

        let result = load_dotenv();
        let loaded_value = env::var("GRIMOIRE_DOTENV_TEST").ok();

        env::set_current_dir(original_dir).unwrap();
        match original_value {
            Some(value) => env::set_var("GRIMOIRE_DOTENV_TEST", value),
            None => env::remove_var("GRIMOIRE_DOTENV_TEST"),
        }
        fs::remove_dir_all(test_dir).unwrap();

        result.unwrap();
        assert_eq!(loaded_value.as_deref(), Some("loaded"));
    }

    #[test]
    fn default_log_filter_enables_server_and_http_access_logs() {
        assert!(DEFAULT_LOG_FILTER.contains("grimoire_server=info"));
        assert!(DEFAULT_LOG_FILTER.contains("tower_http=info"));
    }
}
