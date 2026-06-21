use crate::state::AppState;
use axum::{
    body::Body,
    extract::{Multipart, Path, State},
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Json, Response},
    routing::{get, post},
    Router,
};
use serde::Serialize;
use tokio::io::AsyncReadExt;
use uuid::Uuid;

const MAX_BYTES: usize = 20 * 1024 * 1024;
const ALLOWED_EXT: &[&str] = &["jpg", "jpeg", "png", "gif", "webp", "avif"];

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/upload", post(upload))
        .route("/{name}", get(serve))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct UploadResponse {
    url: String,
}

async fn upload(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> Result<Json<UploadResponse>, StatusCode> {
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|_| StatusCode::BAD_REQUEST)?
    {
        if field.name() != Some("file") {
            continue;
        }
        let original = field.file_name().unwrap_or("upload").to_string();
        let ext = std::path::Path::new(&original)
            .extension()
            .and_then(|s| s.to_str())
            .map(str::to_ascii_lowercase)
            .filter(|e| ALLOWED_EXT.iter().any(|a| *a == e.as_str()))
            .ok_or(StatusCode::UNSUPPORTED_MEDIA_TYPE)?;
        let bytes = field
            .bytes()
            .await
            .map_err(|_| StatusCode::PAYLOAD_TOO_LARGE)?;
        if bytes.len() > MAX_BYTES {
            return Err(StatusCode::PAYLOAD_TOO_LARGE);
        }
        let stored_name = format!("{}.{}", Uuid::new_v4(), ext);
        let path = state.asset_cache_root.join(&stored_name);
        tokio::fs::write(&path, &bytes)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        return Ok(Json(UploadResponse {
            url: format!("/api/assets/{stored_name}"),
        }));
    }
    Err(StatusCode::BAD_REQUEST)
}

async fn serve(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<Response, StatusCode> {
    // Reject anything that could walk out of the asset cache.
    if name.is_empty() || name.contains('/') || name.contains('\\') || name.contains("..") {
        return Err(StatusCode::BAD_REQUEST);
    }
    let path = state.asset_cache_root.join(&name);
    let mut file = tokio::fs::File::open(&path)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;
    let metadata = file
        .metadata()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let mut buf = Vec::with_capacity(metadata.len() as usize);
    file.read_to_end(&mut buf)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mime = match std::path::Path::new(&name)
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("jpg" | "jpeg") => "image/jpeg",
        Some("png") => "image/png",
        Some("gif") => "image/gif",
        Some("webp") => "image/webp",
        Some("avif") => "image/avif",
        _ => "application/octet-stream",
    };

    let mut headers = HeaderMap::new();
    headers.insert(
        header::CONTENT_TYPE,
        header::HeaderValue::from_static(mime),
    );
    headers.insert(
        header::CACHE_CONTROL,
        header::HeaderValue::from_static("public, max-age=31536000, immutable"),
    );
    Ok((headers, Body::from(buf)).into_response())
}
