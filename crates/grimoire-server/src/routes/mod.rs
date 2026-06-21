pub mod assets;
pub mod download;
pub mod export;
pub mod health;
pub mod library;
pub mod metadata;
pub mod scan;
pub mod staging;

use crate::state::AppState;
use axum::{
    routing::{get, post},
    Router,
};

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/api/health", get(health::health))
        .route("/api/library", library::list_route())
        .route("/api/library/", library::list_route())
        .route("/api/scan", post(scan::scan))
        .nest("/api/downloads", download::router())
        .nest("/api/metadata", metadata::router())
        .nest("/api/staging", staging::router())
        .nest("/api/export", export::router())
        .nest("/api/assets", assets::router())
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use grimoire_app::storage::StorageRoot;
    use sqlx::postgres::PgPoolOptions;
    use tower::ServiceExt;

    #[tokio::test]
    async fn library_endpoint_accepts_no_trailing_slash() {
        let app = test_router();

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/library")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_ne!(response.status(), StatusCode::NOT_FOUND);
        assert_ne!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
    }

    #[tokio::test]
    async fn library_endpoint_accepts_trailing_slash() {
        let app = test_router();

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/library/")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_ne!(response.status(), StatusCode::NOT_FOUND);
        assert_ne!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
    }

    fn test_router() -> Router {
        let db = PgPoolOptions::new()
            .connect_lazy("postgres://postgres:postgres@localhost/grimoire")
            .unwrap();

        router(AppState {
            db,
            library_root: StorageRoot::new("/mnt/games"),
            asset_cache_root: std::path::PathBuf::from("/tmp/grimoire-assets"),
        })
    }
}
