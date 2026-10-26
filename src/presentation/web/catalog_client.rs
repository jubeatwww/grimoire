use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::post;
use axum::{Json, Router};

use crate::application::catalog::client::{
    ConfirmMetadataCandidateError, ConfirmMetadataCandidateInput, ConfirmMetadataCandidateUseCase,
    CreateManualGameWorkError, CreateManualGameWorkInput, CreateManualGameWorkUseCase,
    EditGameWorkError, EditGameWorkInput, EditGameWorkUseCase, InventoryItemTriageError,
    InventoryItemTriageUseCase, LinkGameWorkError, LinkGameWorkInput, LinkGameWorkOutput,
    LinkGameWorkUseCase, RefreshGameWorkError, RefreshGameWorkInput, RefreshGameWorkUseCase,
    SearchMetadataError, SearchMetadataInput, SearchMetadataOutput, SearchMetadataUseCase,
    TriageInventoryItemInput,
};
use crate::kernel::http::{HttpError, JsonBody};

#[derive(Clone)]
pub(crate) struct CatalogClientHandlers {
    pub(crate) search: Arc<SearchMetadataUseCase>,
    pub(crate) confirm: Arc<ConfirmMetadataCandidateUseCase>,
    pub(crate) link: Arc<LinkGameWorkUseCase>,
    pub(crate) refresh: Arc<RefreshGameWorkUseCase>,
    pub(crate) manual: Arc<CreateManualGameWorkUseCase>,
    pub(crate) edit: Arc<EditGameWorkUseCase>,
    pub(crate) triage: Arc<InventoryItemTriageUseCase>,
}

pub(crate) fn routes(handlers: CatalogClientHandlers) -> Router {
    Router::new()
        .route("/api/metadata/search", post(search_metadata))
        .route("/api/metadata/confirm", post(confirm_metadata_candidate))
        .route("/api/metadata/link", post(link_game_work))
        .route("/api/metadata/refresh", post(refresh_game_work))
        .route("/api/metadata/manual", post(create_manual_game_work))
        .route("/api/metadata/edit-work", post(edit_game_work))
        .route("/api/metadata/skip", post(skip_inventory_item))
        .route("/api/metadata/exclude", post(exclude_inventory_item))
        .route("/api/metadata/reset", post(reset_inventory_item))
        .with_state(handlers)
}

async fn search_metadata(
    State(handlers): State<CatalogClientHandlers>,
    JsonBody(input): JsonBody<SearchMetadataInput>,
) -> Result<Json<SearchMetadataOutput>, HttpError<SearchMetadataError>> {
    Ok(Json(handlers.search.execute(input).await?))
}

async fn confirm_metadata_candidate(
    State(handlers): State<CatalogClientHandlers>,
    JsonBody(input): JsonBody<ConfirmMetadataCandidateInput>,
) -> Result<Json<LinkGameWorkOutput>, HttpError<ConfirmMetadataCandidateError>> {
    Ok(Json(handlers.confirm.execute(input).await?))
}

async fn link_game_work(
    State(handlers): State<CatalogClientHandlers>,
    JsonBody(input): JsonBody<LinkGameWorkInput>,
) -> Result<Json<LinkGameWorkOutput>, HttpError<LinkGameWorkError>> {
    Ok(Json(handlers.link.execute(input).await?))
}

async fn refresh_game_work(
    State(handlers): State<CatalogClientHandlers>,
    JsonBody(input): JsonBody<RefreshGameWorkInput>,
) -> Result<StatusCode, HttpError<RefreshGameWorkError>> {
    handlers.refresh.execute(input).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn create_manual_game_work(
    State(handlers): State<CatalogClientHandlers>,
    JsonBody(input): JsonBody<CreateManualGameWorkInput>,
) -> Result<Json<LinkGameWorkOutput>, HttpError<CreateManualGameWorkError>> {
    Ok(Json(handlers.manual.execute(input).await?))
}

async fn edit_game_work(
    State(handlers): State<CatalogClientHandlers>,
    JsonBody(input): JsonBody<EditGameWorkInput>,
) -> Result<StatusCode, HttpError<EditGameWorkError>> {
    handlers.edit.execute(input).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn skip_inventory_item(
    State(handlers): State<CatalogClientHandlers>,
    JsonBody(input): JsonBody<TriageInventoryItemInput>,
) -> Result<StatusCode, HttpError<InventoryItemTriageError>> {
    handlers.triage.skip(input).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn exclude_inventory_item(
    State(handlers): State<CatalogClientHandlers>,
    JsonBody(input): JsonBody<TriageInventoryItemInput>,
) -> Result<StatusCode, HttpError<InventoryItemTriageError>> {
    handlers.triage.exclude(input).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn reset_inventory_item(
    State(handlers): State<CatalogClientHandlers>,
    JsonBody(input): JsonBody<TriageInventoryItemInput>,
) -> Result<StatusCode, HttpError<InventoryItemTriageError>> {
    handlers.triage.reset(input).await?;
    Ok(StatusCode::NO_CONTENT)
}
