//! HTTP edge helpers. Presentation code only: the architecture test denies `crate::kernel::http`
//! in domain and application.

use axum::Json;
use axum::extract::rejection::JsonRejection;
use axum::extract::{FromRequest, FromRequestParts};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Serialize;

use crate::kernel::{ErrorCode, ErrorKind};

#[derive(Serialize)]
struct ErrorBody {
    code: &'static str,
    message: String,
}

fn error_response(status: StatusCode, code: &'static str, message: String) -> Response {
    (status, Json(ErrorBody { code, message })).into_response()
}

/// Lets a handler return `Result<Json<Output>, HttpError<E>>` and use `?` on a use-case error.
/// A wrapper is required: the orphan rule forbids `impl<E: ErrorCode> IntoResponse for E`.
#[derive(Debug)]
pub(crate) struct HttpError<E>(pub(crate) E);

impl<E: ErrorCode> From<E> for HttpError<E> {
    fn from(error: E) -> Self {
        Self(error)
    }
}

impl<E: ErrorCode> IntoResponse for HttpError<E> {
    fn into_response(self) -> Response {
        let kind = self.0.kind();
        let status = match kind {
            ErrorKind::Invalid => StatusCode::BAD_REQUEST,
            ErrorKind::NotFound => StatusCode::NOT_FOUND,
            ErrorKind::Conflict => StatusCode::CONFLICT,
            ErrorKind::Rejected => StatusCode::UNPROCESSABLE_ENTITY,
            ErrorKind::Unavailable => StatusCode::SERVICE_UNAVAILABLE,
            ErrorKind::Internal => StatusCode::INTERNAL_SERVER_ERROR,
        };
        let message = match kind {
            ErrorKind::Internal | ErrorKind::Unavailable => {
                tracing::error!(error = %error_chain(&self.0), code = self.0.code(), "request failed");
                "the service could not complete the request".to_owned()
            }
            _ => self.0.to_string(),
        };
        error_response(status, self.0.code(), message)
    }
}

/// The error and every source below it, for the server log.
fn error_chain(error: &dyn std::error::Error) -> String {
    let mut out = error.to_string();
    let mut source = error.source();
    while let Some(e) = source {
        out.push_str(": ");
        out.push_str(&e.to_string());
        source = e.source();
    }
    out
}

/// `axum::Json` with the same `{code, message}` envelope on rejection. Because `*Input` fields are
/// value objects deserialized through their smart constructors, a malformed field is rejected here.
#[derive(FromRequest)]
#[from_request(via(axum::Json), rejection(BodyRejection))]
pub(crate) struct JsonBody<T>(pub(crate) T);

pub(crate) struct BodyRejection(JsonRejection);

impl From<JsonRejection> for BodyRejection {
    fn from(rejection: JsonRejection) -> Self {
        Self(rejection)
    }
}

impl IntoResponse for BodyRejection {
    fn into_response(self) -> Response {
        error_response(
            StatusCode::BAD_REQUEST,
            "request.invalid",
            self.0.body_text(),
        )
    }
}

/// `axum::extract::Path` with the same `{code, message}` envelope on rejection. Path values are
/// value objects or ids, so a malformed segment is rejected here.
#[derive(FromRequestParts)]
#[from_request(via(axum::extract::Path), rejection(PathRejection))]
pub(crate) struct PathParam<T>(pub(crate) T);

pub(crate) struct PathRejection(axum::extract::rejection::PathRejection);

impl From<axum::extract::rejection::PathRejection> for PathRejection {
    fn from(rejection: axum::extract::rejection::PathRejection) -> Self {
        Self(rejection)
    }
}

impl IntoResponse for PathRejection {
    fn into_response(self) -> Response {
        error_response(
            StatusCode::BAD_REQUEST,
            "request.invalid",
            self.0.body_text(),
        )
    }
}

/// Streams a file as an attachment. A file that cannot be opened (it went away after the use
/// case found it) answers 404 in the error envelope.
pub(crate) struct FileDownload {
    path: std::path::PathBuf,
    file_name: String,
}

impl FileDownload {
    pub(crate) fn new(path: std::path::PathBuf, file_name: String) -> Self {
        Self { path, file_name }
    }
}

impl IntoResponse for FileDownload {
    fn into_response(self) -> Response {
        let file = match std::fs::File::open(&self.path) {
            Ok(file) => tokio::fs::File::from_std(file),
            Err(e) => return error_response(StatusCode::NOT_FOUND, "file.missing", e.to_string()),
        };
        let body = axum::body::Body::from_stream(tokio_util::io::ReaderStream::new(file));
        let content_disposition = format!("attachment; filename=\"{}\"", self.file_name);
        (
            [
                (
                    axum::http::header::CONTENT_TYPE,
                    "application/octet-stream".to_string(),
                ),
                (axum::http::header::CONTENT_DISPOSITION, content_disposition),
            ],
            body,
        )
            .into_response()
    }
}
