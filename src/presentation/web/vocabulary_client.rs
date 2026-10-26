use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::{delete, get, post};
use axum::{Json, Router};

use crate::application::vocabulary::client::{
    DefineGenreCodeError, DefineGenreCodeInput, DefineGenreCodeUseCase, GenreCodeOutput,
    RemoveGenreCodeError, RemoveGenreCodeInput, RemoveGenreCodeUseCase, VocabularyClientQueryError,
    VocabularyClientQueryUseCase, VocabularyOutput,
};
use crate::kernel::http::{HttpError, JsonBody, PathParam};

#[derive(Clone)]
pub(crate) struct VocabularyClientHandlers {
    pub(crate) query: Arc<VocabularyClientQueryUseCase>,
    pub(crate) define: Arc<DefineGenreCodeUseCase>,
    pub(crate) remove: Arc<RemoveGenreCodeUseCase>,
}

pub(crate) fn routes(handlers: VocabularyClientHandlers) -> Router {
    Router::new()
        .route("/api/vocabulary", get(find_vocabulary))
        .route("/api/vocabulary/codes", post(define_genre_code))
        .route("/api/vocabulary/codes/{code}", delete(remove_genre_code))
        .with_state(handlers)
}

async fn find_vocabulary(
    State(handlers): State<VocabularyClientHandlers>,
) -> Result<Json<VocabularyOutput>, HttpError<VocabularyClientQueryError>> {
    Ok(Json(handlers.query.find_vocabulary().await?))
}

async fn define_genre_code(
    State(handlers): State<VocabularyClientHandlers>,
    JsonBody(input): JsonBody<DefineGenreCodeInput>,
) -> Result<Json<GenreCodeOutput>, HttpError<DefineGenreCodeError>> {
    Ok(Json(handlers.define.execute(input).await?))
}

async fn remove_genre_code(
    State(handlers): State<VocabularyClientHandlers>,
    PathParam(code): PathParam<String>,
) -> Result<StatusCode, HttpError<RemoveGenreCodeError>> {
    handlers
        .remove
        .execute(RemoveGenreCodeInput { code })
        .await?;
    Ok(StatusCode::NO_CONTENT)
}
