use async_trait::async_trait;
use sqlx::PgPool;
use sqlx::types::Json;
use uuid::Uuid;

use crate::domain::metadata_candidate::{
    MetadataCandidate, MetadataCandidateId, MetadataCandidateRepository,
    MetadataCandidateRepositoryError,
};
use crate::domain::metadata_source::{MetadataProvider, SourceMatch};

pub(crate) struct PgMetadataCandidateRepository {
    pool: PgPool,
}

impl PgMetadataCandidateRepository {
    pub(crate) fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

/// What a candidate keeps of the hit beyond its columns.
#[derive(serde::Serialize, serde::Deserialize, Default)]
struct CandidatePayload {
    title: Option<String>,
    circle: Option<String>,
    work_type: Option<String>,
    work_type_label: Option<String>,
    intro_s: Option<String>,
}

#[derive(sqlx::FromRow)]
struct MetadataCandidateRow {
    id: Uuid,
    source_name: String,
    source_work_id: String,
    source_url: String,
    query_used: String,
    rank: i32,
    title: String,
    circle: Option<String>,
    cover_url: Option<String>,
    normalized_payload: Json<serde_json::Value>,
}

fn backend(e: sqlx::Error) -> MetadataCandidateRepositoryError {
    MetadataCandidateRepositoryError::Backend(Box::new(e))
}

impl TryFrom<MetadataCandidateRow> for MetadataCandidate {
    type Error = MetadataCandidateRepositoryError;

    fn try_from(row: MetadataCandidateRow) -> Result<Self, Self::Error> {
        let provider = MetadataProvider::parse(&row.source_name).ok_or_else(|| {
            MetadataCandidateRepositoryError::Backend(
                format!(
                    "metadata candidate {}: unknown source {:?}",
                    row.id, row.source_name
                )
                .into(),
            )
        })?;
        // Older rows carry other keys; read what is there.
        let payload: CandidatePayload =
            serde_json::from_value(row.normalized_payload.0).unwrap_or_default();
        let found = SourceMatch::new(row.source_work_id, row.rank, row.title)
            .with_circle(row.circle)
            .with_cover_url(row.cover_url)
            .with_work_type(payload.work_type, payload.work_type_label)
            .with_intro(payload.intro_s);
        Ok(MetadataCandidate::from_persisted(
            MetadataCandidateId::from_raw(row.id),
            provider,
            row.source_url,
            row.query_used,
            found,
        ))
    }
}

#[async_trait]
impl MetadataCandidateRepository for PgMetadataCandidateRepository {
    async fn find_by_id(
        &self,
        id: MetadataCandidateId,
    ) -> Result<Option<MetadataCandidate>, MetadataCandidateRepositoryError> {
        let row: Option<MetadataCandidateRow> = sqlx::query_as(
            "SELECT id, source_name, source_work_id, source_url, query_used, rank, title, circle,
                    cover_url, normalized_payload
               FROM metadata_candidates
              WHERE id = $1",
        )
        .bind(id.raw())
        .fetch_optional(&self.pool)
        .await
        .map_err(backend)?;
        row.map(MetadataCandidate::try_from).transpose()
    }

    async fn save(
        &self,
        candidate: &MetadataCandidate,
    ) -> Result<(), MetadataCandidateRepositoryError> {
        let found = candidate.source_match();
        let payload = CandidatePayload {
            title: Some(found.title().to_owned()),
            circle: found.circle().map(str::to_owned),
            work_type: found.work_type().map(str::to_owned),
            work_type_label: found.work_type_label().map(str::to_owned),
            intro_s: found.intro().map(str::to_owned),
        };
        sqlx::query(
            "INSERT INTO metadata_candidates (
                id, source_name, source_work_id, source_url, query_used,
                rank, title, circle, cover_url, normalized_payload
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            ON CONFLICT DO NOTHING",
        )
        .bind(candidate.id().raw())
        .bind(candidate.provider().name())
        .bind(found.source_work_id())
        .bind(candidate.source_url())
        .bind(candidate.query_used())
        .bind(found.rank())
        .bind(found.title())
        .bind(found.circle())
        .bind(found.cover_url())
        .bind(Json(&payload))
        .execute(&self.pool)
        .await
        .map_err(backend)?;
        Ok(())
    }
}
