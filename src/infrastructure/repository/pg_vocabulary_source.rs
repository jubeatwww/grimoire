use async_trait::async_trait;
use sqlx::PgPool;
use sqlx::types::Json;

use crate::domain::genre_code::GenreCode;
use crate::domain::vocabulary::{Category, Vocabulary, VocabularySource, VocabularySourceError};

pub(crate) struct PgVocabularySource {
    pool: PgPool,
}

impl PgVocabularySource {
    pub(crate) fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct CategoryRow {
    id: String,
    label: String,
    folder_name: String,
    sort_order: i32,
}

/// Shared with `PgGenreCodeRepository`.
#[derive(sqlx::FromRow)]
pub(super) struct GenreCodeRow {
    code: String,
    label: String,
    category_id: String,
    family: String,
    accepts_prefix: bool,
    aliases: Json<Vec<String>>,
    sort_order: i32,
}

impl From<GenreCodeRow> for GenreCode {
    fn from(row: GenreCodeRow) -> Self {
        GenreCode::from_persisted(
            row.code,
            row.label,
            row.category_id,
            row.family,
            row.accepts_prefix,
            row.aliases.0,
            row.sort_order,
        )
    }
}

pub(super) const GENRE_CODE_COLUMNS: &str =
    "code, label, category_id, family, accepts_prefix, aliases, sort_order";

#[async_trait]
impl VocabularySource for PgVocabularySource {
    async fn current(&self) -> Result<Vocabulary, VocabularySourceError> {
        let backend = |e: sqlx::Error| VocabularySourceError::Backend(Box::new(e));
        let categories: Vec<CategoryRow> = sqlx::query_as(
            "SELECT id, label, folder_name, sort_order FROM categories ORDER BY sort_order, id",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(backend)?;
        let codes: Vec<GenreCodeRow> = sqlx::query_as(&format!(
            "SELECT {GENRE_CODE_COLUMNS} FROM genre_codes ORDER BY sort_order, code"
        ))
        .fetch_all(&self.pool)
        .await
        .map_err(backend)?;
        Ok(Vocabulary::new(
            categories
                .into_iter()
                .map(|c| Category::new(c.id, c.label, c.folder_name, c.sort_order))
                .collect(),
            codes.into_iter().map(GenreCode::from).collect(),
        ))
    }
}
