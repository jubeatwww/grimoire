use crate::state::AppState;
use axum::{
    body::Body,
    extract::{Path, State},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use sqlx::Row;
use tokio::fs::File;
use tokio_util::io::ReaderStream;
use uuid::Uuid;

pub fn router() -> Router<AppState> {
    Router::new().route("/{id}", get(download_item))
}

async fn download_item(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Response, StatusCode> {
    let row = sqlx::query("SELECT path, file_name FROM inventory_items WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let Some(row) = row else {
        return Err(StatusCode::NOT_FOUND);
    };

    let item_path: String = row.get("path");
    let file_name: String = row.get("file_name");

    let relative = std::path::Path::new(&item_path)
        .strip_prefix(state.library_root.root())
        .map_err(|_| StatusCode::FORBIDDEN)?;
    let resolved = state
        .library_root
        .resolve_relative(relative)
        .map_err(|_| StatusCode::FORBIDDEN)?;

    let file = File::open(resolved)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;
    let stream = ReaderStream::new(file);
    let body = Body::from_stream(stream);

    let content_disposition = format!("attachment; filename=\"{}\"", file_name);

    Ok((
        [
            (header::CONTENT_TYPE, "application/octet-stream".to_string()),
            (header::CONTENT_DISPOSITION, content_disposition),
        ],
        body,
    )
        .into_response())
}
