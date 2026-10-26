use async_trait::async_trait;
use sqlx::PgPool;
use sqlx::types::Json;

use super::pg_vocabulary_source::{GENRE_CODE_COLUMNS, GenreCodeRow};
use crate::domain::genre_code::{GenreCode, GenreCodeRepository, GenreCodeRepositoryError};

pub(crate) struct PgGenreCodeRepository {
    pool: PgPool,
}

impl PgGenreCodeRepository {
    pub(crate) fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

fn backend(e: sqlx::Error) -> GenreCodeRepositoryError {
    GenreCodeRepositoryError::Backend(Box::new(e))
}

#[async_trait]
impl GenreCodeRepository for PgGenreCodeRepository {
    async fn find_by_code(
        &self,
        code: &str,
    ) -> Result<Option<GenreCode>, GenreCodeRepositoryError> {
        let row: Option<GenreCodeRow> = sqlx::query_as(&format!(
            "SELECT {GENRE_CODE_COLUMNS} FROM genre_codes WHERE code = $1"
        ))
        .bind(code)
        .fetch_optional(&self.pool)
        .await
        .map_err(backend)?;
        Ok(row.map(GenreCode::from))
    }

    async fn save(&self, code: &GenreCode) -> Result<(), GenreCodeRepositoryError> {
        sqlx::query(
            "INSERT INTO genre_codes (code, label, category_id, family, accepts_prefix, aliases, sort_order)
             VALUES ($1, $2, $3, $4, $5, $6, $7)
             ON CONFLICT (code) DO UPDATE SET
                label          = EXCLUDED.label,
                category_id    = EXCLUDED.category_id,
                family         = EXCLUDED.family,
                accepts_prefix = EXCLUDED.accepts_prefix,
                aliases        = EXCLUDED.aliases",
        )
        .bind(code.code())
        .bind(code.label())
        .bind(code.category_id())
        .bind(code.family())
        .bind(code.accepts_prefix())
        .bind(Json(code.aliases()))
        .bind(code.sort_order())
        .execute(&self.pool)
        .await
        .map_err(backend)?;
        Ok(())
    }

    async fn delete(&self, code: &str) -> Result<(), GenreCodeRepositoryError> {
        sqlx::query("DELETE FROM genre_codes WHERE code = $1")
            .bind(code)
            .execute(&self.pool)
            .await
            .map_err(backend)?;
        Ok(())
    }
}
