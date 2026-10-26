use async_trait::async_trait;
use chrono::{DateTime, NaiveDate, Utc};
use sqlx::types::Json;
use sqlx::{PgExecutor, PgPool};
use uuid::Uuid;

use crate::domain::game_work::{
    ExternalIds, GameWork, GameWorkId, GameWorkRepository, GameWorkRepositoryError,
    WorkClassification, WorkDetail, WorkIdentity,
};
use crate::domain::metadata_source::MetadataProvider;
use crate::kernel::Timestamp;

pub(crate) struct PgGameWorkRepository {
    pool: PgPool,
}

impl PgGameWorkRepository {
    pub(crate) fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct GameWorkRow {
    id: Uuid,
    canonical_title: String,
    display_title: String,
    circle: Option<String>,
    source_urls: Json<Vec<String>>,
    dlsite_work_id: Option<String>,
    vndb_id: Option<String>,
    steam_app_id: Option<String>,
    description: Option<String>,
    release_date: Option<NaiveDate>,
    series: Option<String>,
    source_tags: Json<Vec<String>>,
    cover_image_url: Option<String>,
    preview_image_urls: Json<Vec<String>>,
    file_type: Option<String>,
    file_size_bytes: Option<i64>,
    dl_count: Option<i32>,
    rate_average: Option<f32>,
    rate_count: Option<i32>,
    price_jpy: Option<i32>,
    work_type: Option<String>,
    work_type_label: Option<String>,
    enriched_at: Option<DateTime<Utc>>,
    primary_category: Option<String>,
    genre_facets: Json<Vec<String>>,
    category_confirmed_at: Option<DateTime<Utc>>,
}

impl From<GameWorkRow> for GameWork {
    fn from(row: GameWorkRow) -> Self {
        let detail = WorkDetail::default()
            .with_description(row.description)
            .with_release_date(row.release_date)
            .with_series(row.series)
            .with_tags(row.source_tags.0)
            .with_cover_image_url(row.cover_image_url)
            .with_preview_image_urls(row.preview_image_urls.0)
            .with_file(row.file_type, row.file_size_bytes)
            .with_sales(
                row.dl_count,
                row.rate_average,
                row.rate_count,
                row.price_jpy,
            )
            .with_work_type(row.work_type, row.work_type_label);
        GameWork::from_persisted(
            GameWorkId::from_raw(row.id),
            WorkIdentity::new(row.canonical_title, row.display_title, row.circle),
            row.source_urls.0,
            ExternalIds::new(row.dlsite_work_id, row.vndb_id, row.steam_app_id),
            detail,
            row.enriched_at.map(Timestamp::from_utc),
            WorkClassification::new(
                row.primary_category,
                row.genre_facets.0,
                row.category_confirmed_at.map(Timestamp::from_utc),
            ),
        )
    }
}

const COLUMNS: &str = "id, canonical_title, display_title, circle, source_urls, dlsite_work_id, vndb_id, \
     steam_app_id, description, release_date, series, source_tags, cover_image_url, \
     preview_image_urls, file_type, file_size_bytes, dl_count, rate_average, rate_count, price_jpy, \
     work_type, work_type_label, enriched_at, primary_category, genre_facets, category_confirmed_at";

fn backend(e: sqlx::Error) -> GameWorkRepositoryError {
    GameWorkRepositoryError::Backend(Box::new(e))
}

/// Shared with `PgClassificationRecorder`, which runs it inside its own transaction.
pub(super) async fn upsert<'e>(
    executor: impl PgExecutor<'e>,
    work: &GameWork,
) -> Result<(), sqlx::Error> {
    let identity = work.identity();
    let ids = work.external_ids();
    let detail = work.detail();
    let classification = work.classification();
    sqlx::query(
        "INSERT INTO game_works (
            id, canonical_title, display_title, circle, source_urls, dlsite_work_id, vndb_id,
            steam_app_id, description, release_date, series, source_tags, cover_image_url,
            preview_image_urls, file_type, file_size_bytes, dl_count, rate_average, rate_count,
            price_jpy, work_type, work_type_label, enriched_at, primary_category, genre_facets,
            category_confirmed_at
         ) VALUES (
            $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19,
            $20, $21, $22, $23, $24, $25, $26
         )
         ON CONFLICT (id) DO UPDATE SET
            canonical_title       = EXCLUDED.canonical_title,
            display_title         = EXCLUDED.display_title,
            circle                = EXCLUDED.circle,
            source_urls           = EXCLUDED.source_urls,
            dlsite_work_id        = EXCLUDED.dlsite_work_id,
            vndb_id               = EXCLUDED.vndb_id,
            steam_app_id          = EXCLUDED.steam_app_id,
            description           = EXCLUDED.description,
            release_date          = EXCLUDED.release_date,
            series                = EXCLUDED.series,
            source_tags           = EXCLUDED.source_tags,
            cover_image_url       = EXCLUDED.cover_image_url,
            preview_image_urls    = EXCLUDED.preview_image_urls,
            file_type             = EXCLUDED.file_type,
            file_size_bytes       = EXCLUDED.file_size_bytes,
            dl_count              = EXCLUDED.dl_count,
            rate_average          = EXCLUDED.rate_average,
            rate_count            = EXCLUDED.rate_count,
            price_jpy             = EXCLUDED.price_jpy,
            work_type             = EXCLUDED.work_type,
            work_type_label       = EXCLUDED.work_type_label,
            enriched_at           = EXCLUDED.enriched_at,
            primary_category      = EXCLUDED.primary_category,
            genre_facets          = EXCLUDED.genre_facets,
            category_confirmed_at = EXCLUDED.category_confirmed_at,
            updated_at            = now()",
    )
    .bind(work.id().raw())
    .bind(identity.canonical_title())
    .bind(identity.display_title())
    .bind(identity.circle())
    .bind(Json(work.source_urls()))
    .bind(ids.get(MetadataProvider::Dlsite))
    .bind(ids.get(MetadataProvider::Vndb))
    .bind(ids.get(MetadataProvider::Steam))
    .bind(detail.description())
    .bind(detail.release_date())
    .bind(detail.series())
    .bind(Json(detail.tags()))
    .bind(detail.cover_image_url())
    .bind(Json(detail.preview_image_urls()))
    .bind(detail.file_type())
    .bind(detail.file_size_bytes())
    .bind(detail.dl_count())
    .bind(detail.rate_average())
    .bind(detail.rate_count())
    .bind(detail.price_jpy())
    .bind(detail.work_type())
    .bind(detail.work_type_label())
    .bind(work.enriched_at().map(Timestamp::as_utc))
    .bind(classification.primary_category())
    .bind(Json(classification.genre_facets()))
    .bind(classification.confirmed_at().map(Timestamp::as_utc))
    .execute(executor)
    .await?;
    Ok(())
}

#[async_trait]
impl GameWorkRepository for PgGameWorkRepository {
    async fn find_by_id(
        &self,
        id: GameWorkId,
    ) -> Result<Option<GameWork>, GameWorkRepositoryError> {
        let row: Option<GameWorkRow> =
            sqlx::query_as(&format!("SELECT {COLUMNS} FROM game_works WHERE id = $1"))
                .bind(id.raw())
                .fetch_optional(&self.pool)
                .await
                .map_err(backend)?;
        Ok(row.map(GameWork::from))
    }

    async fn find_by_external_id(
        &self,
        provider: MetadataProvider,
        external_id: &str,
    ) -> Result<Option<GameWork>, GameWorkRepositoryError> {
        let column = match provider {
            MetadataProvider::Dlsite => "dlsite_work_id",
            MetadataProvider::Vndb => "vndb_id",
            MetadataProvider::Steam => "steam_app_id",
        };
        let row: Option<GameWorkRow> = sqlx::query_as(&format!(
            "SELECT {COLUMNS} FROM game_works WHERE {column} = $1 LIMIT 1"
        ))
        .bind(external_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(backend)?;
        Ok(row.map(GameWork::from))
    }

    async fn list_linked(&self, limit: i64) -> Result<Vec<GameWork>, GameWorkRepositoryError> {
        let rows: Vec<GameWorkRow> = sqlx::query_as(&format!(
            "SELECT {COLUMNS} FROM game_works
              WHERE dlsite_work_id IS NOT NULL OR vndb_id IS NOT NULL OR steam_app_id IS NOT NULL
              ORDER BY created_at
              LIMIT $1"
        ))
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(backend)?;
        Ok(rows.into_iter().map(GameWork::from).collect())
    }

    async fn save(&self, work: &GameWork) -> Result<(), GameWorkRepositoryError> {
        upsert(&self.pool, work).await.map_err(backend)
    }
}
