use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};

use crate::application::library::client::{
    DeleteInventoryItemError, DeleteInventoryItemFileError, DeleteInventoryItemFileInput,
    DeleteInventoryItemFileOutput, DeleteInventoryItemFileUseCase, DeleteInventoryItemInput,
    DeleteInventoryItemUseCase, DeleteMissingInventoryItemsError,
    DeleteMissingInventoryItemsOutput, DeleteMissingInventoryItemsUseCase, LibraryClientQueryError,
    LibraryClientQueryUseCase, ListInventoryItemsOutput, ScanLibraryError, ScanLibraryOutput,
    ScanLibraryUseCase,
};
use crate::domain::inventory_item::InventoryItemId;
use crate::kernel::http::{FileDownload, HttpError, JsonBody, PathParam};

#[derive(Clone)]
pub(crate) struct LibraryClientHandlers {
    pub(crate) query: Arc<LibraryClientQueryUseCase>,
    pub(crate) scan: Arc<ScanLibraryUseCase>,
    pub(crate) delete_item: Arc<DeleteInventoryItemUseCase>,
    pub(crate) delete_missing: Arc<DeleteMissingInventoryItemsUseCase>,
    pub(crate) delete_file: Arc<DeleteInventoryItemFileUseCase>,
}

pub(crate) fn routes(handlers: LibraryClientHandlers) -> Router {
    Router::new()
        .route("/api/library", get(list_library))
        .route("/api/library/", get(list_library))
        .route("/api/scan", post(scan_library))
        .route("/api/downloads/{id}", get(download_file))
        .route("/api/metadata/delete-item", post(delete_inventory_item))
        .route(
            "/api/metadata/delete-missing",
            post(delete_missing_inventory_items),
        )
        .route(
            "/api/metadata/delete-item-and-file",
            post(delete_inventory_item_file),
        )
        .with_state(handlers)
}

async fn list_library(
    State(handlers): State<LibraryClientHandlers>,
) -> Result<Json<ListInventoryItemsOutput>, HttpError<LibraryClientQueryError>> {
    Ok(Json(handlers.query.list_items().await?))
}

async fn scan_library(
    State(handlers): State<LibraryClientHandlers>,
) -> Result<Json<ScanLibraryOutput>, HttpError<ScanLibraryError>> {
    Ok(Json(handlers.scan.execute().await?))
}

async fn download_file(
    State(handlers): State<LibraryClientHandlers>,
    PathParam(id): PathParam<InventoryItemId>,
) -> Result<FileDownload, HttpError<LibraryClientQueryError>> {
    let download = handlers.query.find_download(id).await?;
    Ok(FileDownload::new(download.path, download.file_name))
}

async fn delete_inventory_item(
    State(handlers): State<LibraryClientHandlers>,
    JsonBody(input): JsonBody<DeleteInventoryItemInput>,
) -> Result<StatusCode, HttpError<DeleteInventoryItemError>> {
    handlers.delete_item.execute(input).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn delete_missing_inventory_items(
    State(handlers): State<LibraryClientHandlers>,
) -> Result<Json<DeleteMissingInventoryItemsOutput>, HttpError<DeleteMissingInventoryItemsError>> {
    Ok(Json(handlers.delete_missing.execute().await?))
}

async fn delete_inventory_item_file(
    State(handlers): State<LibraryClientHandlers>,
    JsonBody(input): JsonBody<DeleteInventoryItemFileInput>,
) -> Result<Json<DeleteInventoryItemFileOutput>, HttpError<DeleteInventoryItemFileError>> {
    Ok(Json(handlers.delete_file.execute(input).await?))
}
