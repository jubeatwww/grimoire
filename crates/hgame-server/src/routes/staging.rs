use crate::state::AppState;
use axum::{routing::get, Json, Router};
use serde::Serialize;

#[derive(Serialize)]
struct EmptyList<T> {
    items: Vec<T>,
}

pub fn router() -> Router<AppState> {
    Router::new().route("/", get(list))
}

async fn list() -> Json<EmptyList<serde_json::Value>> {
    Json(EmptyList { items: Vec::new() })
}
