use async_trait::async_trait;
use sqlx::PgPool;

use super::pg_game_work_repository::upsert;
use crate::domain::game_work::{ClassificationRecorder, ClassificationRecorderError, GameWork};

pub(crate) struct PgClassificationRecorder {
    pool: PgPool,
}

impl PgClassificationRecorder {
    pub(crate) fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

fn backend(e: sqlx::Error) -> ClassificationRecorderError {
    ClassificationRecorderError::Backend(Box::new(e))
}

#[async_trait]
impl ClassificationRecorder for PgClassificationRecorder {
    async fn record(&self, works: &[GameWork]) -> Result<(), ClassificationRecorderError> {
        let mut tx = self.pool.begin().await.map_err(backend)?;
        for work in works {
            upsert(&mut *tx, work).await.map_err(backend)?;
        }
        tx.commit().await.map_err(backend)
    }
}
