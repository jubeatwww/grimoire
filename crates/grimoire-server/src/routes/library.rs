use crate::state::AppState;
use axum::{
    routing::{get, MethodRouter},
    Json,
};
use serde::Serialize;

#[derive(Serialize)]
struct EmptyList<T> {
    items: Vec<T>,
}

pub(crate) fn list_route() -> MethodRouter<AppState> {
    get(list)
}

async fn list() -> Json<EmptyList<serde_json::Value>> {
    Json(EmptyList { items: Vec::new() })
}
