use std::collections::HashMap;

use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

use super::pg_inventory_item_repository::LOCAL_SOURCE;
use crate::domain::file_placement::{
    FilePlacement, PlacementSource, PlacementSourceError, WorkPlacement,
};
use crate::domain::game_work::GameWorkId;
use crate::domain::inventory_item::InventoryItemId;
use crate::domain::library_path::LibraryPath;

pub(crate) struct PgPlacementSource {
    pool: PgPool,
}

impl PgPlacementSource {
    pub(crate) fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct PlacementRow {
    id: Uuid,
    path: String,
    file_name: String,
    game_work_id: Uuid,
    display_title: String,
    circle: Option<String>,
    category_id: String,
    folder_name: String,
    work_files: i64,
}

fn backend(e: sqlx::Error) -> PlacementSourceError {
    PlacementSourceError::Backend(Box::new(e))
}

impl TryFrom<PlacementRow> for FilePlacement {
    type Error = PlacementSourceError;

    fn try_from(row: PlacementRow) -> Result<Self, Self::Error> {
        let path = LibraryPath::parse(&row.path).map_err(|e| {
            PlacementSourceError::Backend(format!("inventory item {}: {e}", row.id).into())
        })?;
        Ok(FilePlacement::new(
            InventoryItemId::from_raw(row.id),
            path,
            row.file_name,
            WorkPlacement::new(
                GameWorkId::from_raw(row.game_work_id),
                row.display_title,
                row.circle,
                row.category_id,
                row.folder_name,
                usize::try_from(row.work_files).unwrap_or(0),
            ),
        ))
    }
}

#[derive(sqlx::FromRow)]
struct OccupantRow {
    path: String,
    missing: bool,
}

#[async_trait]
impl PlacementSource for PgPlacementSource {
    async fn confirmed_placements(&self) -> Result<Vec<FilePlacement>, PlacementSourceError> {
        let rows: Vec<PlacementRow> = sqlx::query_as(
            "SELECT i.id, i.path, i.file_name, g.id AS game_work_id, g.display_title, g.circle,
                    c.id AS category_id, c.folder_name,
                    (SELECT count(*) FROM inventory_items j
                      WHERE j.game_work_id = g.id AND NOT j.missing
                        AND j.organization_status <> 'ignored') AS work_files
               FROM inventory_items i
               JOIN game_works g ON g.id = i.game_work_id
               JOIN categories c ON c.id = g.primary_category
              WHERE g.category_confirmed_at IS NOT NULL
                AND i.source_id = $1
                AND NOT i.missing
                AND i.organization_status <> 'ignored'
              ORDER BY c.sort_order, g.circle, g.display_title, i.file_name",
        )
        .bind(LOCAL_SOURCE)
        .fetch_all(&self.pool)
        .await
        .map_err(backend)?;
        rows.into_iter().map(FilePlacement::try_from).collect()
    }

    async fn occupants(
        &self,
        paths: &[LibraryPath],
    ) -> Result<HashMap<LibraryPath, bool>, PlacementSourceError> {
        let paths: Vec<&str> = paths.iter().map(LibraryPath::as_str).collect();
        let rows: Vec<OccupantRow> = sqlx::query_as(
            "SELECT path, missing FROM inventory_items WHERE source_id = $1 AND path = ANY($2)",
        )
        .bind(LOCAL_SOURCE)
        .bind(&paths)
        .fetch_all(&self.pool)
        .await
        .map_err(backend)?;
        Ok(rows
            .into_iter()
            .filter_map(|r| LibraryPath::parse(&r.path).ok().map(|p| (p, r.missing)))
            .collect())
    }
}
