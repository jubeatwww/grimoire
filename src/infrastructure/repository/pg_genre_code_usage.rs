use async_trait::async_trait;
use sqlx::PgPool;

use crate::domain::vocabulary::{GenreCodeUsage, GenreCodeUsageError};

pub(crate) struct PgGenreCodeUsage {
    pool: PgPool,
}

impl PgGenreCodeUsage {
    pub(crate) fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

fn backend(e: sqlx::Error) -> GenreCodeUsageError {
    GenreCodeUsageError::Backend(Box::new(e))
}

#[async_trait]
impl GenreCodeUsage for PgGenreCodeUsage {
    async fn codes_in_works(&self) -> Result<Vec<String>, GenreCodeUsageError> {
        sqlx::query_scalar(
            "SELECT DISTINCT jsonb_array_elements_text(genre_facets) FROM game_works",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(backend)
    }

    async fn codes_in_rules(&self) -> Result<Vec<String>, GenreCodeUsageError> {
        sqlx::query_scalar(
            "SELECT code FROM work_type_rules WHERE code IS NOT NULL
             UNION SELECT code FROM keyword_rules",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(backend)
    }
}
