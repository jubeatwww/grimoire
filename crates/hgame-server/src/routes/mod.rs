pub mod download;
pub mod export;
pub mod health;
pub mod library;
pub mod metadata;
pub mod staging;

use crate::state::AppState;
use axum::{routing::get, Router};

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/api/health", get(health::health))
        .nest("/api/library", library::router())
        .nest("/api/downloads", download::router())
        .nest("/api/metadata", metadata::router())
        .nest("/api/staging", staging::router())
        .nest("/api/export", export::router())
        .with_state(state)
}
