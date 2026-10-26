use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::keyword_rule::{
    KeywordRule, KeywordRuleId, KeywordRuleRepository, KeywordRuleRepositoryError,
};
use crate::domain::metadata_source::MetadataProvider;

pub(crate) struct PgKeywordRuleRepository {
    pool: PgPool,
}

impl PgKeywordRuleRepository {
    pub(crate) fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct KeywordRuleRow {
    id: Uuid,
    source: Option<String>,
    keyword: String,
    code: String,
    contests_type: bool,
}

fn backend(e: sqlx::Error) -> KeywordRuleRepositoryError {
    KeywordRuleRepositoryError::Backend(Box::new(e))
}

impl TryFrom<KeywordRuleRow> for KeywordRule {
    type Error = KeywordRuleRepositoryError;

    fn try_from(row: KeywordRuleRow) -> Result<Self, Self::Error> {
        let source = match row.source.as_deref() {
            None => None,
            Some(name) => Some(MetadataProvider::parse(name).ok_or_else(|| {
                KeywordRuleRepositoryError::Backend(
                    format!("keyword rule {}: unknown source {name:?}", row.id).into(),
                )
            })?),
        };
        Ok(KeywordRule::from_persisted(
            KeywordRuleId::from_raw(row.id),
            source,
            row.keyword,
            row.code,
            row.contests_type,
        ))
    }
}

const COLUMNS: &str = "id, source, keyword, code, contests_type";

#[async_trait]
impl KeywordRuleRepository for PgKeywordRuleRepository {
    async fn find_by_id(
        &self,
        id: KeywordRuleId,
    ) -> Result<Option<KeywordRule>, KeywordRuleRepositoryError> {
        let row: Option<KeywordRuleRow> = sqlx::query_as(&format!(
            "SELECT {COLUMNS} FROM keyword_rules WHERE id = $1"
        ))
        .bind(id.raw())
        .fetch_optional(&self.pool)
        .await
        .map_err(backend)?;
        row.map(KeywordRule::try_from).transpose()
    }

    async fn list_all(&self, limit: i64) -> Result<Vec<KeywordRule>, KeywordRuleRepositoryError> {
        let rows: Vec<KeywordRuleRow> = sqlx::query_as(&format!(
            "SELECT {COLUMNS} FROM keyword_rules ORDER BY created_at, keyword LIMIT $1"
        ))
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(backend)?;
        rows.into_iter().map(KeywordRule::try_from).collect()
    }

    async fn save(&self, rule: &KeywordRule) -> Result<(), KeywordRuleRepositoryError> {
        let result = sqlx::query(
            "INSERT INTO keyword_rules (id, source, keyword, code, contests_type)
             VALUES ($1, $2, $3, $4, $5)
             ON CONFLICT (id) DO UPDATE SET
                source        = EXCLUDED.source,
                keyword       = EXCLUDED.keyword,
                code          = EXCLUDED.code,
                contests_type = EXCLUDED.contests_type",
        )
        .bind(rule.id().raw())
        .bind(rule.source().map(MetadataProvider::name))
        .bind(rule.keyword())
        .bind(rule.code())
        .bind(rule.contests_type())
        .execute(&self.pool)
        .await;
        match result {
            Ok(_) => Ok(()),
            Err(sqlx::Error::Database(db)) if db.is_unique_violation() => Err(
                KeywordRuleRepositoryError::KeywordTaken(rule.keyword().to_owned()),
            ),
            Err(e) => Err(backend(e)),
        }
    }

    async fn delete(&self, id: KeywordRuleId) -> Result<(), KeywordRuleRepositoryError> {
        sqlx::query("DELETE FROM keyword_rules WHERE id = $1")
            .bind(id.raw())
            .execute(&self.pool)
            .await
            .map_err(backend)?;
        Ok(())
    }
}
