use async_trait::async_trait;
use sqlx::PgPool;

use super::{
    pg_game_work_repository, pg_inventory_item_repository, pg_work_source_data_repository,
};
use crate::domain::catalog_recorder::{CatalogRecorder, CatalogRecorderError};
use crate::domain::game_work::GameWork;
use crate::domain::inventory_item::InventoryItem;
use crate::domain::work_source_data::WorkSourceData;

pub(crate) struct PgCatalogRecorder {
    pool: PgPool,
}

impl PgCatalogRecorder {
    pub(crate) fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

fn backend(e: sqlx::Error) -> CatalogRecorderError {
    CatalogRecorderError::Backend(Box::new(e))
}

#[async_trait]
impl CatalogRecorder for PgCatalogRecorder {
    async fn record(
        &self,
        work: &GameWork,
        source_data: Option<&WorkSourceData>,
        item: Option<&InventoryItem>,
    ) -> Result<(), CatalogRecorderError> {
        let mut tx = self.pool.begin().await.map_err(backend)?;
        // The work first: the source data and the file both point at it.
        pg_game_work_repository::upsert(&mut *tx, work)
            .await
            .map_err(backend)?;
        if let Some(data) = source_data {
            pg_work_source_data_repository::upsert(&mut *tx, data)
                .await
                .map_err(backend)?;
        }
        if let Some(item) = item {
            pg_inventory_item_repository::upsert(&mut *tx, item)
                .await
                .map_err(backend)?;
        }
        tx.commit().await.map_err(backend)
    }
}
