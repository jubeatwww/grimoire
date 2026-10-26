use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{PgExecutor, PgPool};
use uuid::Uuid;

use crate::domain::game_work::GameWorkId;
use crate::domain::inventory_item::{
    InventoryItem, InventoryItemId, InventoryItemRepository, InventoryItemRepositoryError,
    OrganizationStatus, ScannedFile,
};
use crate::domain::library_path::LibraryPath;
use crate::kernel::Timestamp;

pub(crate) struct PgInventoryItemRepository {
    pool: PgPool,
}

impl PgInventoryItemRepository {
    pub(crate) fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct InventoryItemRow {
    id: Uuid,
    path: String,
    file_name: String,
    extension: Option<String>,
    file_size: i64,
    modified_at: DateTime<Utc>,
    legacy_location: Option<String>,
    game_work_id: Option<Uuid>,
    organization_status: String,
}

/// The scan source of every item: the NAS share mounted at the library root.
pub(super) const LOCAL_SOURCE: &str = "local";

fn organization_status(raw: &str) -> Option<OrganizationStatus> {
    match raw {
        "pending" => Some(OrganizationStatus::Pending),
        "matched" => Some(OrganizationStatus::Matched),
        "confirmed" => Some(OrganizationStatus::Confirmed),
        "no_match" => Some(OrganizationStatus::NoMatch),
        "ignored" => Some(OrganizationStatus::Ignored),
        _ => None,
    }
}

fn backend(e: sqlx::Error) -> InventoryItemRepositoryError {
    InventoryItemRepositoryError::Backend(Box::new(e))
}

fn corrupt(id: Uuid, what: impl std::fmt::Display) -> InventoryItemRepositoryError {
    InventoryItemRepositoryError::Backend(format!("inventory item {id}: {what}").into())
}

impl TryFrom<InventoryItemRow> for InventoryItem {
    type Error = InventoryItemRepositoryError;

    fn try_from(row: InventoryItemRow) -> Result<Self, Self::Error> {
        let path = LibraryPath::parse(&row.path).map_err(|e| corrupt(row.id, e))?;
        let status = organization_status(&row.organization_status).ok_or_else(|| {
            corrupt(
                row.id,
                format!("unknown status {:?}", row.organization_status),
            )
        })?;
        let file = ScannedFile::new(
            path,
            row.file_name,
            row.extension,
            u64::try_from(row.file_size).unwrap_or(0),
            Timestamp::from_utc(row.modified_at),
            row.legacy_location,
        );
        Ok(InventoryItem::from_persisted(
            InventoryItemId::from_raw(row.id),
            file,
            row.game_work_id.map(GameWorkId::from_raw),
            status,
        ))
    }
}

const COLUMNS: &str = "id, path, file_name, extension, file_size, modified_at, legacy_location, \
                       game_work_id, organization_status";

/// New rows start as unplayed files; the rest of the columns keep their defaults.
const INSERT: &str = "INSERT INTO inventory_items (
        id, source_id, path, file_name, extension, kind, file_size, modified_at,
        legacy_location, game_work_id, organization_status, play_status
    ) VALUES ($1, $2, $3, $4, $5, 'file', $6, $7, $8, $9, $10, 'not_played')";

fn bind_insert<'q>(
    query: sqlx::query::Query<'q, sqlx::Postgres, sqlx::postgres::PgArguments>,
    item: &'q InventoryItem,
) -> sqlx::query::Query<'q, sqlx::Postgres, sqlx::postgres::PgArguments> {
    let file = item.file();
    query
        .bind(item.id().raw())
        .bind(LOCAL_SOURCE)
        .bind(file.path().as_str())
        .bind(file.file_name())
        .bind(file.extension())
        .bind(i64::try_from(file.file_size()).unwrap_or(i64::MAX))
        .bind(file.modified_at().as_utc())
        .bind(file.legacy_location())
        .bind(item.work_id().map(GameWorkId::raw))
        .bind(item.status().as_str())
}

/// An existing item keeps its file facts; only its work link and status change. Shared with
/// `PgCatalogRecorder`, which runs it inside its own transaction.
pub(super) async fn upsert<'e>(
    executor: impl PgExecutor<'e>,
    item: &InventoryItem,
) -> Result<(), sqlx::Error> {
    let sql = format!(
        "{INSERT}
         ON CONFLICT (id) DO UPDATE SET
            game_work_id        = EXCLUDED.game_work_id,
            organization_status = EXCLUDED.organization_status,
            updated_at          = now()"
    );
    bind_insert(sqlx::query(&sql), item)
        .execute(executor)
        .await?;
    Ok(())
}

#[async_trait]
impl InventoryItemRepository for PgInventoryItemRepository {
    async fn find_by_id(
        &self,
        id: InventoryItemId,
    ) -> Result<Option<InventoryItem>, InventoryItemRepositoryError> {
        let row: Option<InventoryItemRow> = sqlx::query_as(&format!(
            "SELECT {COLUMNS} FROM inventory_items WHERE id = $1"
        ))
        .bind(id.raw())
        .fetch_optional(&self.pool)
        .await
        .map_err(backend)?;
        row.map(InventoryItem::try_from).transpose()
    }

    async fn save(&self, item: &InventoryItem) -> Result<(), InventoryItemRepositoryError> {
        upsert(&self.pool, item).await.map_err(backend)
    }

    async fn delete(&self, id: InventoryItemId) -> Result<(), InventoryItemRepositoryError> {
        sqlx::query("DELETE FROM inventory_items WHERE id = $1")
            .bind(id.raw())
            .execute(&self.pool)
            .await
            .map_err(backend)?;
        Ok(())
    }

    async fn record_scan(
        &self,
        found: &[InventoryItem],
    ) -> Result<u64, InventoryItemRepositoryError> {
        let sql = format!(
            "{INSERT}
             ON CONFLICT (source_id, path) DO UPDATE SET
                file_name   = EXCLUDED.file_name,
                extension   = EXCLUDED.extension,
                kind        = EXCLUDED.kind,
                file_size   = EXCLUDED.file_size,
                modified_at = EXCLUDED.modified_at,
                missing     = false,
                updated_at  = now()"
        );
        let mut tx = self.pool.begin().await.map_err(backend)?;
        for item in found {
            bind_insert(sqlx::query(&sql), item)
                .execute(&mut *tx)
                .await
                .map_err(backend)?;
        }
        // Anything previously visible under this source that did not show up in this scan is
        // missing. Reappearing paths were reset to false by the upsert above.
        let paths: Vec<&str> = found.iter().map(|i| i.path().as_str()).collect();
        let marked = sqlx::query(
            "UPDATE inventory_items
                SET missing = true, updated_at = now()
              WHERE source_id = $1
                AND missing = false
                AND NOT (path = ANY($2::text[]))",
        )
        .bind(LOCAL_SOURCE)
        .bind(&paths)
        .execute(&mut *tx)
        .await
        .map_err(backend)?;
        tx.commit().await.map_err(backend)?;
        Ok(marked.rows_affected())
    }

    async fn delete_missing(&self) -> Result<u64, InventoryItemRepositoryError> {
        let result = sqlx::query("DELETE FROM inventory_items WHERE missing = true")
            .execute(&self.pool)
            .await
            .map_err(backend)?;
        Ok(result.rows_affected())
    }

    async fn relocate(
        &self,
        id: InventoryItemId,
        from: &LibraryPath,
        to: &LibraryPath,
    ) -> Result<bool, InventoryItemRepositoryError> {
        let result = sqlx::query(
            "UPDATE inventory_items SET path = $1, updated_at = now() WHERE id = $2 AND path = $3",
        )
        .bind(to.as_str())
        .bind(id.raw())
        .bind(from.as_str())
        .execute(&self.pool)
        .await
        .map_err(backend)?;
        Ok(result.rows_affected() == 1)
    }
}
