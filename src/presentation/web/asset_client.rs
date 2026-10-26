use std::sync::Arc;

use axum::body::Body;
use axum::extract::{FromRequest, Multipart, Request, State};
use axum::http::{HeaderMap, HeaderValue, header};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Serialize;

use crate::application::asset::client::{
    AssetClientQueryError, AssetClientQueryUseCase, UploadAssetError, UploadAssetInput,
    UploadAssetOutput, UploadAssetUseCase,
};
use crate::domain::asset::Asset;
use crate::kernel::http::{HttpError, PathParam};
use crate::kernel::{ErrorCode, ErrorKind};

#[derive(Clone)]
pub(crate) struct AssetClientHandlers {
    pub(crate) query: Arc<AssetClientQueryUseCase>,
    pub(crate) upload: Arc<UploadAssetUseCase>,
}

pub(crate) fn routes(handlers: AssetClientHandlers) -> Router {
    Router::new()
        .route("/api/assets/upload", post(upload_asset))
        .route("/api/assets/{name}", get(find_asset))
        .with_state(handlers)
}

/// The `file` field of a multipart upload.
pub(crate) struct UploadAssetRequest {
    original_name: String,
    bytes: Vec<u8>,
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum UploadAssetRequestError {
    #[error("the upload has no file field")]
    NoFile,
    #[error("the upload could not be read: {0}")]
    Unreadable(String),
}

impl ErrorCode for UploadAssetRequestError {
    fn code(&self) -> &'static str {
        match self {
            Self::NoFile => "request.no_file",
            Self::Unreadable(_) => "request.invalid",
        }
    }

    fn kind(&self) -> ErrorKind {
        ErrorKind::Invalid
    }
}

impl<S: Send + Sync> FromRequest<S> for UploadAssetRequest {
    type Rejection = HttpError<UploadAssetRequestError>;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let unreadable = |text: String| HttpError(UploadAssetRequestError::Unreadable(text));
        let mut multipart = Multipart::from_request(req, state)
            .await
            .map_err(|e| unreadable(e.body_text()))?;
        while let Some(field) = multipart
            .next_field()
            .await
            .map_err(|e| unreadable(e.body_text()))?
        {
            if field.name() != Some("file") {
                continue;
            }
            let original_name = field.file_name().unwrap_or("upload").to_string();
            let bytes = field.bytes().await.map_err(|e| unreadable(e.body_text()))?;
            return Ok(Self {
                original_name,
                bytes: bytes.to_vec(),
            });
        }
        Err(HttpError(UploadAssetRequestError::NoFile))
    }
}

impl From<UploadAssetRequest> for UploadAssetInput {
    fn from(request: UploadAssetRequest) -> Self {
        Self {
            original_name: request.original_name,
            bytes: request.bytes,
        }
    }
}

/// Where the API serves the stored image.
#[derive(Serialize)]
pub(crate) struct UploadAssetResponse {
    url: String,
}

impl From<UploadAssetOutput> for UploadAssetResponse {
    fn from(output: UploadAssetOutput) -> Self {
        Self {
            url: format!("/api/assets/{}", output.name),
        }
    }
}

async fn upload_asset(
    State(handlers): State<AssetClientHandlers>,
    request: UploadAssetRequest,
) -> Result<Json<UploadAssetResponse>, HttpError<UploadAssetError>> {
    let output = handlers.upload.execute(request.into()).await?;
    Ok(Json(output.into()))
}

async fn find_asset(
    State(handlers): State<AssetClientHandlers>,
    PathParam(asset): PathParam<Asset>,
) -> Result<Response, HttpError<AssetClientQueryError>> {
    let asset = handlers.query.find_asset(asset).await?;
    let mut headers = HeaderMap::new();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static(asset.media_type),
    );
    headers.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("public, max-age=31536000, immutable"),
    );
    Ok((headers, Body::from(asset.bytes)).into_response())
}
