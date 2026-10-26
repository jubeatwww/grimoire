use async_trait::async_trait;
use sqlx::PgPool;

use crate::domain::metadata_source::MetadataProvider;
use crate::domain::work_type_rule::{
    WorkTypeRule, WorkTypeRuleRepository, WorkTypeRuleRepositoryError,
};

pub(crate) struct PgWorkTypeRuleRepository {
    pool: PgPool,
}

impl PgWorkTypeRuleRepository {
    pub(crate) fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct WorkTypeRuleRow {
    source: String,
    work_type: String,
    code: Option<String>,
    confident: bool,
}

fn backend(e: sqlx::Error) -> WorkTypeRuleRepositoryError {
    WorkTypeRuleRepositoryError::Backend(Box::new(e))
}

impl TryFrom<WorkTypeRuleRow> for WorkTypeRule {
    type Error = WorkTypeRuleRepositoryError;

    fn try_from(row: WorkTypeRuleRow) -> Result<Self, Self::Error> {
        let source = MetadataProvider::parse(&row.source).ok_or_else(|| {
            WorkTypeRuleRepositoryError::Backend(
                format!("work type rule: unknown source {:?}", row.source).into(),
            )
        })?;
        Ok(WorkTypeRule::from_persisted(
            source,
            row.work_type,
            row.code,
            row.confident,
        ))
    }
}

#[async_trait]
impl WorkTypeRuleRepository for PgWorkTypeRuleRepository {
    async fn find_by_source_and_work_type(
        &self,
        source: MetadataProvider,
        work_type: &str,
    ) -> Result<Option<WorkTypeRule>, WorkTypeRuleRepositoryError> {
        let row: Option<WorkTypeRuleRow> = sqlx::query_as(
            "SELECT source, work_type, code, confident FROM work_type_rules
              WHERE source = $1 AND work_type = $2",
        )
        .bind(source.name())
        .bind(work_type)
        .fetch_optional(&self.pool)
        .await
        .map_err(backend)?;
        row.map(WorkTypeRule::try_from).transpose()
    }

    async fn list_all(&self, limit: i64) -> Result<Vec<WorkTypeRule>, WorkTypeRuleRepositoryError> {
        let rows: Vec<WorkTypeRuleRow> = sqlx::query_as(
            "SELECT source, work_type, code, confident FROM work_type_rules
              ORDER BY source, work_type LIMIT $1",
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(backend)?;
        rows.into_iter().map(WorkTypeRule::try_from).collect()
    }

    async fn save(&self, rule: &WorkTypeRule) -> Result<(), WorkTypeRuleRepositoryError> {
        sqlx::query(
            "INSERT INTO work_type_rules (source, work_type, code, confident)
             VALUES ($1, $2, $3, $4)
             ON CONFLICT (source, work_type) DO UPDATE SET
                code = EXCLUDED.code, confident = EXCLUDED.confident",
        )
        .bind(rule.source().name())
        .bind(rule.work_type())
        .bind(rule.code())
        .bind(rule.confident())
        .execute(&self.pool)
        .await
        .map_err(backend)?;
        Ok(())
    }

    async fn delete(&self, rule: &WorkTypeRule) -> Result<(), WorkTypeRuleRepositoryError> {
        sqlx::query("DELETE FROM work_type_rules WHERE source = $1 AND work_type = $2")
            .bind(rule.source().name())
            .bind(rule.work_type())
            .execute(&self.pool)
            .await
            .map_err(backend)?;
        Ok(())
    }
}
