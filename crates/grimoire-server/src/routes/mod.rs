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
        .route("/api/library", library::list_route())
        .route("/api/library/", library::list_route())
        .nest("/api/downloads", download::router())
        .nest("/api/metadata", metadata::router())
        .nest("/api/staging", staging::router())
        .nest("/api/export", export::router())
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

        assert_eq!(response.status(), StatusCode::OK);
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

        assert_eq!(response.status(), StatusCode::OK);
    }

    fn test_router() -> Router {
        let db = PgPoolOptions::new()
            .connect_lazy("postgres://postgres:postgres@localhost/grimoire")
            .unwrap();

        router(AppState {
            db,
            library_root: StorageRoot::new("/mnt/games"),
        })
    }
}
