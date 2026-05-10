use grimoire_domain::{
    asset::Asset, inventory::InventoryItem, metadata::MetadataCandidate,
    staging::StagingItem, work::GameWork,
};
use serde::{Deserialize, Serialize};

pub const EXPORT_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Serialize, Deserialize)]
pub struct ExportSnapshot {
    pub schema_version: u32,
    pub game_works: Vec<GameWork>,
    pub inventory_items: Vec<InventoryItem>,
    pub metadata_candidates: Vec<MetadataCandidate>,
    pub assets: Vec<Asset>,
    pub staging_items: Vec<StagingItem>,
    pub category_definitions: Vec<String>,
}

impl ExportSnapshot {
    pub fn empty() -> Self {
        Self {
            schema_version: EXPORT_SCHEMA_VERSION,
            game_works: Vec::new(),
            inventory_items: Vec::new(),
            metadata_candidates: Vec::new(),
            assets: Vec::new(),
            staging_items: Vec::new(),
            category_definitions: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_has_schema_version() {
        let snapshot = ExportSnapshot::empty();
        let json = serde_json::to_value(snapshot).unwrap();
        assert_eq!(json["schema_version"], 1);
    }
}
