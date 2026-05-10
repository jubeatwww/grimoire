use crate::state::AppState;
use axum::{routing::get, Json, Router};
use serde::Serialize;

#[derive(Serialize)]
struct DownloadStatus {
    status: &'static str,
}

pub fn router() -> Router<AppState> {
    Router::new().route("/", get(index))
}

async fn index() -> Json<DownloadStatus> {
    Json(DownloadStatus { status: "download routes ready" })
}
