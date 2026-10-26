//! Builds every adapter, app service, use case and handler state, and merges the routes.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use axum::routing::get;
use axum::{Json, Router};
use serde::Serialize;
use sqlx::PgPool;
use tokio::sync::Mutex;

use crate::application::asset::client::{AssetClientQueryUseCase, UploadAssetUseCase};
use crate::application::catalog::client::{
    ConfirmMetadataCandidateUseCase, CreateManualGameWorkUseCase, EditGameWorkUseCase,
    GameWorkEnrichmentAppService, InventoryItemTriageUseCase, LinkGameWorkUseCase,
    RefreshGameWorkUseCase, SearchMetadataUseCase,
};
use crate::application::classification::client::{
    ClassificationClientQueryUseCase, ConfirmClassificationsUseCase, DefineKeywordRuleUseCase,
    DefineWorkTypeRuleUseCase, RemoveKeywordRuleUseCase, RemoveWorkTypeRuleUseCase,
};
use crate::application::library::client::{
    DeleteInventoryItemFileUseCase, DeleteInventoryItemUseCase, DeleteMissingInventoryItemsUseCase,
    LibraryClientQueryUseCase, ScanLibraryUseCase,
};
use crate::application::organization::client::{
    CreateInboxUseCase, ExecuteMovesUseCase, OrganizationClientQueryUseCase, UndoMoveBatchUseCase,
};
use crate::application::organization::job::ReconcileMoveBatchesUseCase;
use crate::application::organization::{
    MovePlanAppService, MoveReconciliationAppService, MoveSettlementAppService,
};
use crate::application::source_refetch::client::{
    SourceRefetchClientQueryUseCase, StartSourceRefetchUseCase,
};
use crate::application::vocabulary::client::{
    DefineGenreCodeUseCase, RemoveGenreCodeUseCase, VocabularyClientQueryUseCase,
};
use crate::domain::asset::AssetStore;
use crate::domain::catalog_recorder::CatalogRecorder;
use crate::domain::game_work::{ClassificationRecorder, GameWorkRepository};
use crate::domain::genre_code::GenreCodeRepository;
use crate::domain::inventory_item::InventoryItemRepository;
use crate::domain::keyword_rule::KeywordRuleRepository;
use crate::domain::library_files::LibraryFiles;
use crate::domain::library_folders::LibraryFolders;
use crate::domain::library_lock_registry::LibraryLockRegistry;
use crate::domain::metadata_candidate::MetadataCandidateRepository;
use crate::domain::metadata_source::{MetadataProvider, MetadataSource};
use crate::domain::move_batch::MoveBatchRepository;
use crate::domain::source_refetch::SourceRefetch;
use crate::domain::vocabulary::VocabularySource;
use crate::domain::work_source_data::WorkSourceDataRepository;
use crate::domain::work_type_rule::WorkTypeRuleRepository;
use crate::infrastructure::acl::dlsite::DlsiteMetadataSourceAcl;
use crate::infrastructure::acl::steam::SteamMetadataSourceAcl;
use crate::infrastructure::acl::vndb::VndbMetadataSourceAcl;
use crate::infrastructure::lock::InMemoryLibraryLockRegistry;
use crate::infrastructure::repository::{
    PgCatalogRecorder, PgClassificationClientQueryRepository, PgClassificationRecorder,
    PgGameWorkRepository, PgGenreCodeRepository, PgGenreCodeUsage, PgInventoryItemRepository,
    PgKeywordRuleRepository, PgLibraryClientQueryRepository, PgMetadataCandidateRepository,
    PgMoveBatchRepository, PgPlacementSource, PgVocabularySource, PgWorkSourceDataRepository,
    PgWorkTypeRuleRepository,
};
use crate::infrastructure::storage::{
    FsAssetStore, FsLibraryFiles, FsLibraryFolders, FsLibraryScanner,
};
use crate::kernel::{Clock, IdGenerator, SystemClock, UuidV7IdGenerator};
use crate::presentation::web::{
    AssetClientHandlers, CatalogClientHandlers, ClassificationClientHandlers,
    LibraryClientHandlers, OrganizationClientHandlers, SourceRefetchClientHandlers,
    VocabularyClientHandlers, asset_client_routes, catalog_client_routes,
    classification_client_routes, library_client_routes, organization_client_routes,
    source_refetch_client_routes, vocabulary_client_routes,
};

/// The wired application: its routes, and the startup job.
pub(super) struct App {
    pub(super) router: Router,
    pub(super) reconcile: Arc<ReconcileMoveBatchesUseCase>,
}

#[derive(Serialize)]
struct Health {
    status: &'static str,
}

/// Liveness and readiness probe.
async fn health() -> Json<Health> {
    Json(Health { status: "ok" })
}

/// Repositories and other domain traits, shared by the use cases.
struct Adapters {
    inventory: Arc<dyn InventoryItemRepository>,
    works: Arc<dyn GameWorkRepository>,
    recorder: Arc<dyn ClassificationRecorder>,
    catalog: Arc<dyn CatalogRecorder>,
    source_data: Arc<dyn WorkSourceDataRepository>,
    candidates: Arc<dyn MetadataCandidateRepository>,
    vocabulary: Arc<dyn VocabularySource>,
    genre_codes: Arc<dyn GenreCodeRepository>,
    work_type_rules: Arc<dyn WorkTypeRuleRepository>,
    keyword_rules: Arc<dyn KeywordRuleRepository>,
    batches: Arc<dyn MoveBatchRepository>,
    files: Arc<dyn LibraryFiles>,
    folders: Arc<dyn LibraryFolders>,
    assets: Arc<dyn AssetStore>,
    lock: Arc<dyn LibraryLockRegistry>,
    sources: HashMap<MetadataProvider, Arc<dyn MetadataSource>>,
    clock: Arc<dyn Clock>,
    ids: Arc<dyn IdGenerator>,
}

pub(super) fn wire(pool: PgPool, library_root: PathBuf, asset_cache_root: PathBuf) -> App {
    let sources: [Arc<dyn MetadataSource>; 3] = [
        Arc::new(DlsiteMetadataSourceAcl::new()),
        Arc::new(VndbMetadataSourceAcl::new()),
        Arc::new(SteamMetadataSourceAcl::new()),
    ];
    let a = Adapters {
        inventory: Arc::new(PgInventoryItemRepository::new(pool.clone())),
        works: Arc::new(PgGameWorkRepository::new(pool.clone())),
        recorder: Arc::new(PgClassificationRecorder::new(pool.clone())),
        catalog: Arc::new(PgCatalogRecorder::new(pool.clone())),
        source_data: Arc::new(PgWorkSourceDataRepository::new(pool.clone())),
        candidates: Arc::new(PgMetadataCandidateRepository::new(pool.clone())),
        vocabulary: Arc::new(PgVocabularySource::new(pool.clone())),
        genre_codes: Arc::new(PgGenreCodeRepository::new(pool.clone())),
        work_type_rules: Arc::new(PgWorkTypeRuleRepository::new(pool.clone())),
        keyword_rules: Arc::new(PgKeywordRuleRepository::new(pool.clone())),
        batches: Arc::new(PgMoveBatchRepository::new(pool.clone())),
        files: Arc::new(FsLibraryFiles::new(library_root.clone())),
        folders: Arc::new(FsLibraryFolders::new(
            library_root.clone(),
            asset_cache_root.clone(),
        )),
        assets: Arc::new(FsAssetStore::new(asset_cache_root)),
        lock: Arc::new(InMemoryLibraryLockRegistry::default()),
        sources: sources.into_iter().map(|s| (s.provider(), s)).collect(),
        clock: Arc::new(SystemClock),
        ids: Arc::new(UuidV7IdGenerator),
    };

    let reconciliation = Arc::new(MoveReconciliationAppService::new(
        a.batches.clone(),
        a.inventory.clone(),
        a.files.clone(),
        a.clock.clone(),
    ));
    let router = Router::new()
        .route("/api/health", get(health))
        .merge(library_client_routes(library(&a, &pool, library_root)))
        .merge(catalog_client_routes(catalog(&a)))
        .merge(vocabulary_client_routes(vocabulary(&a, &pool)))
        .merge(classification_client_routes(classification(&a, &pool)))
        .merge(organization_client_routes(organization(
            &a,
            &pool,
            &reconciliation,
        )))
        .merge(source_refetch_client_routes(source_refetch(&a)))
        .merge(asset_client_routes(asset(&a)));
    App {
        router,
        reconcile: Arc::new(ReconcileMoveBatchesUseCase::new(
            a.lock.clone(),
            reconciliation,
        )),
    }
}

fn library(a: &Adapters, pool: &PgPool, library_root: PathBuf) -> LibraryClientHandlers {
    LibraryClientHandlers {
        query: Arc::new(LibraryClientQueryUseCase::new(
            Arc::new(PgLibraryClientQueryRepository::new(pool.clone())),
            a.inventory.clone(),
            a.files.clone(),
        )),
        scan: Arc::new(ScanLibraryUseCase::new(
            Arc::new(FsLibraryScanner::new(library_root)),
            a.inventory.clone(),
            a.lock.clone(),
            a.ids.clone(),
        )),
        delete_item: Arc::new(DeleteInventoryItemUseCase::new(a.inventory.clone())),
        delete_missing: Arc::new(DeleteMissingInventoryItemsUseCase::new(a.inventory.clone())),
        delete_file: Arc::new(DeleteInventoryItemFileUseCase::new(
            a.inventory.clone(),
            a.files.clone(),
            a.lock.clone(),
        )),
    }
}

fn catalog(a: &Adapters) -> CatalogClientHandlers {
    let enrichment = Arc::new(GameWorkEnrichmentAppService::new(
        a.sources.clone(),
        a.clock.clone(),
    ));
    CatalogClientHandlers {
        search: Arc::new(SearchMetadataUseCase::new(
            a.sources.clone(),
            a.candidates.clone(),
            a.ids.clone(),
        )),
        confirm: Arc::new(ConfirmMetadataCandidateUseCase::new(
            a.inventory.clone(),
            a.candidates.clone(),
            a.works.clone(),
            enrichment.clone(),
            a.catalog.clone(),
            a.ids.clone(),
        )),
        link: Arc::new(LinkGameWorkUseCase::new(
            a.inventory.clone(),
            a.works.clone(),
            enrichment.clone(),
            a.catalog.clone(),
            a.ids.clone(),
        )),
        refresh: Arc::new(RefreshGameWorkUseCase::new(
            a.inventory.clone(),
            a.works.clone(),
            enrichment,
            a.catalog.clone(),
        )),
        manual: Arc::new(CreateManualGameWorkUseCase::new(
            a.inventory.clone(),
            a.catalog.clone(),
            a.ids.clone(),
            a.clock.clone(),
        )),
        edit: Arc::new(EditGameWorkUseCase::new(
            a.inventory.clone(),
            a.works.clone(),
        )),
        triage: Arc::new(InventoryItemTriageUseCase::new(a.inventory.clone())),
    }
}

fn vocabulary(a: &Adapters, pool: &PgPool) -> VocabularyClientHandlers {
    VocabularyClientHandlers {
        query: Arc::new(VocabularyClientQueryUseCase::new(a.vocabulary.clone())),
        define: Arc::new(DefineGenreCodeUseCase::new(
            a.vocabulary.clone(),
            a.genre_codes.clone(),
        )),
        remove: Arc::new(RemoveGenreCodeUseCase::new(
            a.vocabulary.clone(),
            Arc::new(PgGenreCodeUsage::new(pool.clone())),
            a.genre_codes.clone(),
        )),
    }
}

fn classification(a: &Adapters, pool: &PgPool) -> ClassificationClientHandlers {
    ClassificationClientHandlers {
        query: Arc::new(ClassificationClientQueryUseCase::new(
            Arc::new(PgClassificationClientQueryRepository::new(pool.clone())),
            a.vocabulary.clone(),
            a.work_type_rules.clone(),
            a.keyword_rules.clone(),
        )),
        confirm: Arc::new(ConfirmClassificationsUseCase::new(
            a.vocabulary.clone(),
            a.works.clone(),
            a.recorder.clone(),
            a.clock.clone(),
        )),
        define_work_type_rule: Arc::new(DefineWorkTypeRuleUseCase::new(
            a.vocabulary.clone(),
            a.work_type_rules.clone(),
        )),
        remove_work_type_rule: Arc::new(RemoveWorkTypeRuleUseCase::new(a.work_type_rules.clone())),
        define_keyword_rule: Arc::new(DefineKeywordRuleUseCase::new(
            a.vocabulary.clone(),
            a.keyword_rules.clone(),
            a.ids.clone(),
        )),
        remove_keyword_rule: Arc::new(RemoveKeywordRuleUseCase::new(a.keyword_rules.clone())),
    }
}

fn organization(
    a: &Adapters,
    pool: &PgPool,
    reconciliation: &Arc<MoveReconciliationAppService>,
) -> OrganizationClientHandlers {
    let plans = Arc::new(MovePlanAppService::new(
        Arc::new(PgPlacementSource::new(pool.clone())),
        a.files.clone(),
        a.vocabulary.clone(),
    ));
    let settlement = Arc::new(MoveSettlementAppService::new(
        a.inventory.clone(),
        a.files.clone(),
    ));
    OrganizationClientHandlers {
        query: Arc::new(OrganizationClientQueryUseCase::new(
            plans.clone(),
            a.batches.clone(),
            a.folders.clone(),
        )),
        execute: Arc::new(ExecuteMovesUseCase::new(
            a.lock.clone(),
            plans.clone(),
            settlement.clone(),
            reconciliation.clone(),
            a.batches.clone(),
            a.folders.clone(),
            a.ids.clone(),
            a.clock.clone(),
        )),
        undo: Arc::new(UndoMoveBatchUseCase::new(
            a.lock.clone(),
            plans,
            settlement,
            reconciliation.clone(),
            a.batches.clone(),
            a.folders.clone(),
            a.clock.clone(),
        )),
        create_inbox: Arc::new(CreateInboxUseCase::new(a.lock.clone(), a.folders.clone())),
    }
}

fn source_refetch(a: &Adapters) -> SourceRefetchClientHandlers {
    let progress = Arc::new(Mutex::new(SourceRefetch::idle()));
    SourceRefetchClientHandlers {
        query: Arc::new(SourceRefetchClientQueryUseCase::new(progress.clone())),
        start: Arc::new(StartSourceRefetchUseCase::new(
            a.works.clone(),
            a.sources.clone(),
            a.source_data.clone(),
            progress,
            a.clock.clone(),
        )),
    }
}

fn asset(a: &Adapters) -> AssetClientHandlers {
    AssetClientHandlers {
        query: Arc::new(AssetClientQueryUseCase::new(a.assets.clone())),
        upload: Arc::new(UploadAssetUseCase::new(a.assets.clone(), a.ids.clone())),
    }
}

#[cfg(test)]
mod tests {
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use sqlx::postgres::PgPoolOptions;
    use tower::ServiceExt;

    use super::*;

    fn test_router() -> Router {
        // Never connects: the routes are found before any handler needs the database.
        let pool = PgPoolOptions::new()
            .acquire_timeout(std::time::Duration::from_millis(50))
            .connect_lazy("postgres://postgres:postgres@localhost:1/grimoire")
            .unwrap();
        wire(
            pool,
            PathBuf::from("/mnt/games"),
            PathBuf::from("/tmp/grimoire-assets"),
        )
        .router
    }

    async fn status_of(uri: &str) -> StatusCode {
        let request = Request::builder().uri(uri).body(Body::empty()).unwrap();
        test_router().oneshot(request).await.unwrap().status()
    }

    #[tokio::test]
    async fn library_endpoint_accepts_both_spellings() {
        for uri in ["/api/library", "/api/library/"] {
            let status = status_of(uri).await;
            assert_ne!(status, StatusCode::NOT_FOUND, "{uri}");
            assert_ne!(status, StatusCode::METHOD_NOT_ALLOWED, "{uri}");
        }
    }

    #[tokio::test]
    async fn health_answers_without_the_database() {
        assert_eq!(status_of("/api/health").await, StatusCode::OK);
    }
}
