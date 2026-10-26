use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::{delete, get, post};
use axum::{Json, Router};

use crate::application::classification::client::{
    ClassificationClientQueryError, ClassificationClientQueryUseCase, ConfirmClassificationsError,
    ConfirmClassificationsInput, ConfirmClassificationsOutput, ConfirmClassificationsUseCase,
    DefineKeywordRuleError, DefineKeywordRuleInput, DefineKeywordRuleUseCase,
    DefineWorkTypeRuleError, DefineWorkTypeRuleInput, DefineWorkTypeRuleUseCase, KeywordRuleOutput,
    ListCategorySuggestionsOutput, ListClassificationRulesOutput, ListCommonTagsOutput,
    RemoveKeywordRuleError, RemoveKeywordRuleInput, RemoveKeywordRuleUseCase,
    RemoveWorkTypeRuleError, RemoveWorkTypeRuleInput, RemoveWorkTypeRuleUseCase,
    WorkTypeRuleOutput,
};
use crate::domain::keyword_rule::KeywordRuleId;
use crate::domain::metadata_source::MetadataProvider;
use crate::kernel::http::{HttpError, JsonBody, PathParam};

#[derive(Clone)]
pub(crate) struct ClassificationClientHandlers {
    pub(crate) query: Arc<ClassificationClientQueryUseCase>,
    pub(crate) confirm: Arc<ConfirmClassificationsUseCase>,
    pub(crate) define_work_type_rule: Arc<DefineWorkTypeRuleUseCase>,
    pub(crate) remove_work_type_rule: Arc<RemoveWorkTypeRuleUseCase>,
    pub(crate) define_keyword_rule: Arc<DefineKeywordRuleUseCase>,
    pub(crate) remove_keyword_rule: Arc<RemoveKeywordRuleUseCase>,
}

pub(crate) fn routes(handlers: ClassificationClientHandlers) -> Router {
    Router::new()
        .route("/api/classification/confirm", post(confirm_classifications))
        .route("/api/classification/suggestions", get(list_suggestions))
        .route("/api/classification/rules", get(list_rules))
        .route(
            "/api/classification/rules/work-type",
            post(define_work_type_rule),
        )
        .route(
            "/api/classification/rules/work-type/{source}/{work_type}",
            delete(remove_work_type_rule),
        )
        .route(
            "/api/classification/rules/keyword",
            post(define_keyword_rule),
        )
        .route(
            "/api/classification/rules/keyword/{id}",
            delete(remove_keyword_rule),
        )
        .route("/api/classification/common-tags", get(list_common_tags))
        .with_state(handlers)
}

async fn confirm_classifications(
    State(handlers): State<ClassificationClientHandlers>,
    JsonBody(input): JsonBody<ConfirmClassificationsInput>,
) -> Result<Json<ConfirmClassificationsOutput>, HttpError<ConfirmClassificationsError>> {
    Ok(Json(handlers.confirm.execute(input).await?))
}

async fn list_suggestions(
    State(handlers): State<ClassificationClientHandlers>,
) -> Result<Json<ListCategorySuggestionsOutput>, HttpError<ClassificationClientQueryError>> {
    Ok(Json(handlers.query.list_suggestions().await?))
}

async fn list_rules(
    State(handlers): State<ClassificationClientHandlers>,
) -> Result<Json<ListClassificationRulesOutput>, HttpError<ClassificationClientQueryError>> {
    Ok(Json(handlers.query.list_rules().await?))
}

async fn define_work_type_rule(
    State(handlers): State<ClassificationClientHandlers>,
    JsonBody(input): JsonBody<DefineWorkTypeRuleInput>,
) -> Result<Json<WorkTypeRuleOutput>, HttpError<DefineWorkTypeRuleError>> {
    Ok(Json(handlers.define_work_type_rule.execute(input).await?))
}

async fn remove_work_type_rule(
    State(handlers): State<ClassificationClientHandlers>,
    PathParam((source, work_type)): PathParam<(MetadataProvider, String)>,
) -> Result<StatusCode, HttpError<RemoveWorkTypeRuleError>> {
    handlers
        .remove_work_type_rule
        .execute(RemoveWorkTypeRuleInput { source, work_type })
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn define_keyword_rule(
    State(handlers): State<ClassificationClientHandlers>,
    JsonBody(input): JsonBody<DefineKeywordRuleInput>,
) -> Result<Json<KeywordRuleOutput>, HttpError<DefineKeywordRuleError>> {
    Ok(Json(handlers.define_keyword_rule.execute(input).await?))
}

async fn remove_keyword_rule(
    State(handlers): State<ClassificationClientHandlers>,
    PathParam(id): PathParam<KeywordRuleId>,
) -> Result<StatusCode, HttpError<RemoveKeywordRuleError>> {
    handlers
        .remove_keyword_rule
        .execute(RemoveKeywordRuleInput { id })
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn list_common_tags(
    State(handlers): State<ClassificationClientHandlers>,
) -> Result<Json<ListCommonTagsOutput>, HttpError<ClassificationClientQueryError>> {
    Ok(Json(handlers.query.list_common_tags().await?))
}
