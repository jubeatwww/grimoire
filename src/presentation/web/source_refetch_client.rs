use std::sync::Arc;

use axum::extract::State;
use axum::routing::get;
use axum::{Json, Router};

use crate::application::source_refetch::client::{
    SourceRefetchClientQueryUseCase, SourceRefetchOutput, StartSourceRefetchError,
    StartSourceRefetchUseCase,
};
use crate::kernel::http::HttpError;

#[derive(Clone)]
pub(crate) struct SourceRefetchClientHandlers {
    pub(crate) query: Arc<SourceRefetchClientQueryUseCase>,
    pub(crate) start: Arc<StartSourceRefetchUseCase>,
}

pub(crate) fn routes(handlers: SourceRefetchClientHandlers) -> Router {
    Router::new()
        .route(
            "/api/sources/refetch",
            get(find_progress).post(start_source_refetch),
        )
        .with_state(handlers)
}

async fn find_progress(
    State(handlers): State<SourceRefetchClientHandlers>,
) -> Json<SourceRefetchOutput> {
    Json(handlers.query.find_progress().await)
}

async fn start_source_refetch(
    State(handlers): State<SourceRefetchClientHandlers>,
) -> Result<Json<SourceRefetchOutput>, HttpError<StartSourceRefetchError>> {
    Ok(Json(Arc::clone(&handlers.start).execute().await?))
}
