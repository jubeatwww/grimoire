use std::sync::Arc;

use axum::extract::State;
use axum::routing::{get, post};
use axum::{Json, Router};

use crate::application::organization::client::{
    CreateInboxError, CreateInboxUseCase, ExecuteMovesError, ExecuteMovesInput,
    ExecuteMovesUseCase, InboxOutput, ListMoveBatchesOutput, ListOtherFilesOutput, MoveBatchOutput,
    MovePlanOutput, OrganizationClientQueryError, OrganizationClientQueryUseCase,
    PreviewMovesInput, PreviewMovesOutput, UndoMoveBatchError, UndoMoveBatchInput,
    UndoMoveBatchUseCase,
};
use crate::domain::move_batch::MoveBatchId;
use crate::kernel::http::{HttpError, JsonBody, PathParam};

#[derive(Clone)]
pub(crate) struct OrganizationClientHandlers {
    pub(crate) query: Arc<OrganizationClientQueryUseCase>,
    pub(crate) execute: Arc<ExecuteMovesUseCase>,
    pub(crate) undo: Arc<UndoMoveBatchUseCase>,
    pub(crate) create_inbox: Arc<CreateInboxUseCase>,
}

pub(crate) fn routes(handlers: OrganizationClientHandlers) -> Router {
    Router::new()
        .route("/api/organize/plan", get(find_plan))
        .route("/api/organize/preview", post(preview_moves))
        .route("/api/organize/execute", post(execute_moves))
        .route("/api/organize/batches", get(list_batches))
        .route("/api/organize/batches/{id}", get(find_batch))
        .route("/api/organize/batches/{id}/undo", post(undo_move_batch))
        .route("/api/organize/inbox", get(find_inbox).post(create_inbox))
        .route("/api/organize/other-files", get(list_other_files))
        .with_state(handlers)
}

async fn find_plan(
    State(handlers): State<OrganizationClientHandlers>,
) -> Result<Json<MovePlanOutput>, HttpError<OrganizationClientQueryError>> {
    Ok(Json(handlers.query.find_plan().await?))
}

async fn preview_moves(
    State(handlers): State<OrganizationClientHandlers>,
    JsonBody(input): JsonBody<PreviewMovesInput>,
) -> Result<Json<PreviewMovesOutput>, HttpError<OrganizationClientQueryError>> {
    Ok(Json(handlers.query.preview(input).await?))
}

async fn execute_moves(
    State(handlers): State<OrganizationClientHandlers>,
    JsonBody(input): JsonBody<ExecuteMovesInput>,
) -> Result<Json<MoveBatchOutput>, HttpError<ExecuteMovesError>> {
    Ok(Json(Arc::clone(&handlers.execute).execute(input).await?))
}

async fn list_batches(
    State(handlers): State<OrganizationClientHandlers>,
) -> Result<Json<ListMoveBatchesOutput>, HttpError<OrganizationClientQueryError>> {
    Ok(Json(handlers.query.list_batches().await?))
}

async fn find_batch(
    State(handlers): State<OrganizationClientHandlers>,
    PathParam(id): PathParam<MoveBatchId>,
) -> Result<Json<MoveBatchOutput>, HttpError<OrganizationClientQueryError>> {
    Ok(Json(handlers.query.find_batch(id).await?))
}

async fn undo_move_batch(
    State(handlers): State<OrganizationClientHandlers>,
    PathParam(id): PathParam<MoveBatchId>,
) -> Result<Json<MoveBatchOutput>, HttpError<UndoMoveBatchError>> {
    let input = UndoMoveBatchInput { batch_id: id };
    Ok(Json(Arc::clone(&handlers.undo).execute(input).await?))
}

async fn find_inbox(State(handlers): State<OrganizationClientHandlers>) -> Json<InboxOutput> {
    Json(handlers.query.find_inbox().await)
}

async fn create_inbox(
    State(handlers): State<OrganizationClientHandlers>,
) -> Result<Json<InboxOutput>, HttpError<CreateInboxError>> {
    Ok(Json(handlers.create_inbox.execute().await?))
}

async fn list_other_files(
    State(handlers): State<OrganizationClientHandlers>,
) -> Result<Json<ListOtherFilesOutput>, HttpError<OrganizationClientQueryError>> {
    Ok(Json(handlers.query.list_other_files().await?))
}
