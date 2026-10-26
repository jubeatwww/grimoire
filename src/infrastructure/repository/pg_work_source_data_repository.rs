use async_trait::async_trait;
use sqlx::types::Json;
use sqlx::{PgExecutor, PgPool};

use crate::domain::work_source_data::{
    WorkSourceData, WorkSourceDataRepository, WorkSourceDataRepositoryError,
};

pub(crate) struct PgWorkSourceDataRepository {
    pool: PgPool,
}

impl PgWorkSourceDataRepository {
    pub(crate) fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

/// Shared with `PgCatalogRecorder`, which runs it inside its own transaction.
pub(super) async fn upsert<'e>(
    executor: impl PgExecutor<'e>,
    data: &WorkSourceData,
) -> Result<(), sqlx::Error> {
    sqlx::query(
            "INSERT INTO work_source_data (game_work_id, source, tags, work_type, work_type_label, fetched_at)
             VALUES ($1, $2, $3, $4, $5, $6)
             ON CONFLICT (game_work_id, source) DO UPDATE SET
                tags            = EXCLUDED.tags,
                work_type       = EXCLUDED.work_type,
                work_type_label = EXCLUDED.work_type_label,
                fetched_at      = EXCLUDED.fetched_at",
        )
        .bind(data.work_id().raw())
        .bind(data.provider().name())
        .bind(Json(data.tags()))
        .bind(data.work_type())
        .bind(data.work_type_label())
        .bind(data.fetched_at().as_utc())
        .execute(executor)
        .await?;
    Ok(())
}

#[async_trait]
impl WorkSourceDataRepository for PgWorkSourceDataRepository {
    async fn save(&self, data: &WorkSourceData) -> Result<(), WorkSourceDataRepositoryError> {
        upsert(&self.pool, data)
            .await
            .map_err(|e| WorkSourceDataRepositoryError::Backend(Box::new(e)))
    }
}
