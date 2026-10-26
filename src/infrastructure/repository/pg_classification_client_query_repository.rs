use std::collections::HashMap;

use async_trait::async_trait;
use sqlx::PgPool;
use sqlx::types::Json;
use uuid::Uuid;

use crate::application::classification::client::{
    ClassificationClientQueryRepository, ClassificationClientQueryRepositoryError,
    CommonTagListItemOutput,
};
use crate::domain::category_suggestion::{SourceFacts, WorkFacts};
use crate::domain::game_work::GameWorkId;
use crate::domain::metadata_source::MetadataProvider;

pub(crate) struct PgClassificationClientQueryRepository {
    pool: PgPool,
}

impl PgClassificationClientQueryRepository {
    pub(crate) fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

fn backend(e: sqlx::Error) -> ClassificationClientQueryRepositoryError {
    ClassificationClientQueryRepositoryError::Backend(Box::new(e))
}

#[derive(sqlx::FromRow)]
struct SourceFactsRow {
    game_work_id: Uuid,
    source: String,
    tags: Json<Vec<String>>,
    work_type: Option<String>,
}

#[derive(sqlx::FromRow)]
struct FileHintRow {
    game_work_id: Uuid,
    legacy_location: Option<String>,
    primary_category: Option<String>,
}

#[derive(sqlx::FromRow)]
struct CommonTagRow {
    label: String,
    source: String,
    n: i64,
}

impl From<CommonTagRow> for CommonTagListItemOutput {
    fn from(row: CommonTagRow) -> Self {
        Self {
            label: row.label,
            source: row.source,
            count: row.n,
        }
    }
}

/// Facts of one work while they are being gathered.
#[derive(Default)]
struct FactsInProgress {
    sources: Vec<SourceFacts>,
    legacy_locations: Vec<String>,
    old_primaries: Vec<String>,
}

#[async_trait]
impl ClassificationClientQueryRepository for PgClassificationClientQueryRepository {
    async fn list_unclassified_facts(
        &self,
        limit: i64,
    ) -> Result<Vec<(GameWorkId, WorkFacts)>, ClassificationClientQueryRepositoryError> {
        let ids: Vec<Uuid> = sqlx::query_scalar(
            "SELECT g.id
               FROM game_works g
              WHERE g.category_confirmed_at IS NULL
                AND EXISTS (SELECT 1 FROM inventory_items i
                             WHERE i.game_work_id = g.id
                               AND NOT i.missing
                               AND i.organization_status <> 'ignored')
              ORDER BY g.display_title
              LIMIT $1",
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(backend)?;

        let mut facts: HashMap<Uuid, FactsInProgress> = ids
            .iter()
            .map(|id| (*id, FactsInProgress::default()))
            .collect();

        let sources: Vec<SourceFactsRow> = sqlx::query_as(
            "SELECT game_work_id, source, tags, work_type
               FROM work_source_data
              WHERE game_work_id = ANY($1)
              ORDER BY source",
        )
        .bind(&ids)
        .fetch_all(&self.pool)
        .await
        .map_err(backend)?;
        for row in sources {
            let (Some(f), Some(provider)) = (
                facts.get_mut(&row.game_work_id),
                MetadataProvider::parse(&row.source),
            ) else {
                continue;
            };
            f.sources
                .push(SourceFacts::new(provider, row.work_type, row.tags.0));
        }

        let hints: Vec<FileHintRow> = sqlx::query_as(
            "SELECT game_work_id, legacy_location, primary_category
               FROM inventory_items
              WHERE game_work_id = ANY($1) AND NOT missing",
        )
        .bind(&ids)
        .fetch_all(&self.pool)
        .await
        .map_err(backend)?;
        for row in hints {
            if let Some(f) = facts.get_mut(&row.game_work_id) {
                f.legacy_locations.extend(row.legacy_location);
                f.old_primaries.extend(row.primary_category);
            }
        }

        Ok(ids
            .into_iter()
            .map(|id| {
                let f = facts.remove(&id).unwrap_or_default();
                let work_facts = WorkFacts::new(f.sources, f.legacy_locations, f.old_primaries);
                (GameWorkId::from_raw(id), work_facts)
            })
            .collect())
    }

    async fn list_common_tags(
        &self,
        limit: i64,
    ) -> Result<Vec<CommonTagListItemOutput>, ClassificationClientQueryRepositoryError> {
        let rows: Vec<CommonTagRow> = sqlx::query_as(
            "SELECT t.label, s.source, count(*) AS n
               FROM work_source_data s
               JOIN game_works g ON g.id = s.game_work_id
         CROSS JOIN jsonb_array_elements_text(s.tags) AS t(label)
              WHERE g.category_confirmed_at IS NULL
           GROUP BY t.label, s.source
           ORDER BY n DESC, t.label
              LIMIT $1",
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(backend)?;
        Ok(rows
            .into_iter()
            .map(CommonTagListItemOutput::from)
            .collect())
    }
}
