use serde::Serialize;

use crate::domain::game_work::GameWorkId;
use crate::domain::inventory_item::InventoryItemId;
use crate::domain::move_plan::{MovePlan, PlannedMove};

#[derive(Debug, Serialize)]
pub(crate) struct MovePlanOutput {
    moves: Vec<PlannedMoveOutput>,
}

impl From<&MovePlan> for MovePlanOutput {
    fn from(plan: &MovePlan) -> Self {
        Self {
            moves: plan.moves().iter().map(PlannedMoveOutput::from).collect(),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PreviewMovesOutput {
    moves: Vec<PlannedMoveOutput>,
    removable_dirs: Vec<String>,
}

impl PreviewMovesOutput {
    /// The plan of the selected files and the folders the moves would empty.
    pub(crate) fn new(plan: &MovePlan, removable_dirs: Vec<String>) -> Self {
        Self {
            moves: plan.moves().iter().map(PlannedMoveOutput::from).collect(),
            removable_dirs,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PlannedMoveOutput {
    inventory_item_id: InventoryItemId,
    game_work_id: GameWorkId,
    title: String,
    category_id: String,
    from_path: String,
    to_path: String,
    /// Why this file cannot move right now, if it cannot.
    conflict: Option<&'static str>,
}

impl From<&PlannedMove> for PlannedMoveOutput {
    fn from(m: &PlannedMove) -> Self {
        Self {
            inventory_item_id: m.item_id(),
            game_work_id: m.work_id(),
            title: m.title().to_owned(),
            category_id: m.category_id().to_owned(),
            from_path: m.from().to_string(),
            to_path: m.to().to_string(),
            conflict: m.conflict().map(|c| c.message()),
        }
    }
}
