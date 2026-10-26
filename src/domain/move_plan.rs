//! Every file of a confirmed work that is not where it belongs, with what stops it moving.

mod move_conflict;
mod move_selection;
mod planned_move;

use std::collections::{HashMap, HashSet};

use crate::domain::file_placement::FilePlacement;
use crate::domain::game_work::GameWorkId;
use crate::domain::inventory_item::InventoryItemId;
use crate::domain::library_path::LibraryPath;

pub(crate) use move_conflict::MoveConflict;
pub(crate) use move_selection::MoveSelection;
pub(crate) use planned_move::PlannedMove;

#[derive(Debug, Clone, Default)]
pub(crate) struct MovePlan {
    moves: Vec<PlannedMove>,
    /// Work folders and the works that end up inside each one, counting files already in
    /// place. Two works sharing a folder name must not be mixed.
    folder_owners: HashMap<String, HashSet<GameWorkId>>,
}

impl MovePlan {
    /// The moves the placements call for, conflicts not checked yet. A target that does not
    /// form a valid library path is left out.
    pub(crate) fn draft(placements: &[FilePlacement]) -> Self {
        let mut plan = Self::default();
        for placement in placements {
            let target = placement.target();
            let in_work = placement.work_folder_target();
            if target.as_deref().unwrap_or(placement.path().as_str()) == in_work
                && let Some((dir, _)) = in_work.rsplit_once('/')
            {
                plan.folder_owners
                    .entry(dir.to_string())
                    .or_default()
                    .insert(placement.work().work_id());
            }
            if let Some(to) = target.and_then(|t| LibraryPath::parse(&t).ok()) {
                plan.moves.push(PlannedMove::new(placement, to));
            }
        }
        plan
    }

    /// Every target, in plan order.
    pub(crate) fn targets(&self) -> Vec<LibraryPath> {
        self.moves.iter().map(|m| m.to().clone()).collect()
    }

    /// `occupants`: records already at a target, with whether each is missing. `on_disk`:
    /// whether something exists at each target, in `targets()` order.
    pub(crate) fn mark_conflicts(
        &mut self,
        occupants: &HashMap<LibraryPath, bool>,
        on_disk: &[bool],
    ) {
        let mut target_count: HashMap<&LibraryPath, usize> = HashMap::new();
        for m in &self.moves {
            *target_count.entry(m.to()).or_default() += 1;
        }
        let duplicated: HashSet<LibraryPath> = target_count
            .into_iter()
            .filter(|(_, n)| *n > 1)
            .map(|(p, _)| p.clone())
            .collect();

        for (m, exists) in self.moves.iter_mut().zip(on_disk) {
            let conflict = if duplicated.contains(m.to()) {
                Some(MoveConflict::SameTarget)
            } else if let Some(missing) = occupants.get(m.to()) {
                Some(if *missing {
                    MoveConflict::HeldByMissingRecord
                } else {
                    MoveConflict::HeldByRecord
                })
            } else if *exists {
                Some(MoveConflict::TargetExists)
            } else {
                None
            };
            m.block(conflict);
        }

        for m in self.moves.iter_mut().filter(|m| m.conflict().is_none()) {
            let shared = m
                .to()
                .folder()
                .and_then(|dir| self.folder_owners.get(dir))
                .is_some_and(|owners| owners.len() > 1);
            if shared {
                m.block(Some(MoveConflict::SharedWorkFolder));
            }
        }
    }

    /// Keeps only the moves of these files.
    pub(crate) fn keep_only(&mut self, items: &[InventoryItemId]) {
        let wanted: HashSet<&InventoryItemId> = items.iter().collect();
        self.moves.retain(|m| wanted.contains(&m.item_id()));
    }

    pub(crate) fn moves(&self) -> &[PlannedMove] {
        &self.moves
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.moves.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::file_placement::WorkPlacement;

    fn file(
        item: u128,
        work: u128,
        title: &str,
        path: &str,
        name: &str,
        count: usize,
    ) -> FilePlacement {
        FilePlacement::new(
            InventoryItemId::from_raw(uuid::Uuid::from_u128(item)),
            LibraryPath::parse(path).unwrap(),
            name.into(),
            WorkPlacement::new(
                GameWorkId::from_raw(uuid::Uuid::from_u128(work)),
                title.into(),
                Some("Same".into()),
                "RPG".into(),
                "04 角色扮演(RPG)".into(),
                count,
            ),
        )
    }

    #[test]
    fn titles_sanitizing_to_one_folder_block_both_works() {
        let mut plan = MovePlan::draft(&[
            file(1, 10, "Title?", "RPG/a.zip", "a.zip", 2),
            file(2, 10, "Title?", "RPG/b.zip", "b.zip", 2),
            file(3, 20, "Title:", "RPG/c.zip", "c.zip", 2),
            file(4, 20, "Title:", "RPG/d.zip", "d.zip", 2),
        ]);
        let on_disk = vec![false; plan.targets().len()];
        plan.mark_conflicts(&HashMap::new(), &on_disk);
        assert!(
            plan.moves()
                .iter()
                .all(|m| m.conflict() == Some(MoveConflict::SharedWorkFolder))
        );
    }

    #[test]
    fn conflicts_are_checked_in_order() {
        let mut plan = MovePlan::draft(&[
            file(1, 10, "A", "RPG/a.zip", "a.zip", 1),
            file(2, 11, "B", "ADV/a.zip", "a.zip", 1),
            file(3, 12, "C", "RPG/c.zip", "c.zip", 1),
            file(4, 13, "D", "RPG/d.zip", "d.zip", 1),
            file(5, 14, "E", "RPG/e.zip", "e.zip", 1),
        ]);
        let held: HashMap<LibraryPath, bool> = [
            (
                LibraryPath::parse("04 角色扮演(RPG)/Same/c.zip").unwrap(),
                true,
            ),
            (
                LibraryPath::parse("04 角色扮演(RPG)/Same/d.zip").unwrap(),
                false,
            ),
        ]
        .into();
        plan.mark_conflicts(&held, &[false, false, false, false, true]);
        let conflicts: Vec<Option<MoveConflict>> =
            plan.moves().iter().map(|m| m.conflict()).collect();
        assert_eq!(
            conflicts,
            vec![
                Some(MoveConflict::SameTarget),
                Some(MoveConflict::SameTarget),
                Some(MoveConflict::HeldByMissingRecord),
                Some(MoveConflict::HeldByRecord),
                Some(MoveConflict::TargetExists),
            ]
        );
    }

    #[test]
    fn keep_only_filters_after_planning() {
        let mut plan = MovePlan::draft(&[
            file(1, 10, "A", "RPG/a.zip", "a.zip", 1),
            file(2, 11, "B", "RPG/b.zip", "b.zip", 1),
        ]);
        plan.keep_only(&[InventoryItemId::from_raw(uuid::Uuid::from_u128(2))]);
        assert_eq!(plan.moves().len(), 1);
        assert_eq!(plan.moves()[0].from().as_str(), "RPG/b.zip");
    }
}
