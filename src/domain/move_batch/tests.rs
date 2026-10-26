use chrono::{TimeZone, Utc};

use super::*;
use crate::domain::file_placement::{FilePlacement, WorkPlacement};
use crate::domain::game_work::GameWorkId;
use crate::domain::inventory_item::InventoryItemId;
use crate::domain::library_files::MoveProbe;
use crate::domain::library_path::LibraryPath;
use crate::domain::move_plan::PlannedMove;

fn at(minute: u32) -> Timestamp {
    Timestamp::from_utc(Utc.with_ymd_and_hms(2026, 1, 1, 0, minute, 0).unwrap())
}

fn path(raw: &str) -> LibraryPath {
    LibraryPath::parse(raw).unwrap()
}

fn planned(n: u128) -> PlannedMove {
    let placement = FilePlacement::new(
        InventoryItemId::from_raw(uuid::Uuid::from_u128(n)),
        path(&format!("RPG/{n}.zip")),
        format!("{n}.zip"),
        WorkPlacement::new(
            GameWorkId::from_raw(uuid::Uuid::from_u128(100 + n)),
            "T".into(),
            None,
            "RPG".into(),
            "04 角色扮演(RPG)".into(),
            1,
        ),
    );
    PlannedMove::new(
        &placement,
        path(&format!("04 角色扮演(RPG)/未知社團/{n}.zip")),
    )
}

fn op_id(n: u128) -> MoveOpId {
    MoveOpId::from_raw(uuid::Uuid::from_u128(1000 + n))
}

fn started() -> MoveBatch {
    MoveBatch::start(MoveBatchId::from_raw(uuid::Uuid::from_u128(1)), at(0)).0
}

fn batch_with(outcomes: &[MoveOutcome]) -> MoveBatch {
    let mut batch = started();
    for (n, outcome) in outcomes.iter().enumerate() {
        let n = n as u128;
        let logged = batch.log_move(op_id(n), &planned(n)).unwrap();
        let _settled = batch.settle_move(logged.seq, outcome.clone()).unwrap();
    }
    batch
}

#[test]
fn finishing_counts_every_outcome() {
    let mut batch = batch_with(&[
        MoveOutcome::Moved,
        MoveOutcome::Skipped("target already exists".into()),
        MoveOutcome::Failed("boom".into()),
        MoveOutcome::Moved,
    ]);
    let _skip = batch
        .log_skip(
            op_id(9),
            &planned(9),
            "another file is planned for the same path".into(),
        )
        .unwrap();
    let _finished = batch.finish(vec!["RPG".into()], vec![], at(1)).unwrap();
    assert_eq!(batch.status(), MoveBatchStatus::Done);
    assert_eq!(batch.counts(), MoveCounts::new(2, 2, 1));
    assert_eq!(batch.times().finished_at(), Some(at(1)));
}

#[test]
fn a_finished_run_takes_no_more_moves() {
    let mut batch = batch_with(&[MoveOutcome::Moved]);
    let _finished = batch.finish(vec![], vec![], at(1)).unwrap();
    assert!(batch.log_move(op_id(5), &planned(5)).is_err());
    assert!(batch.finish(vec![], vec![], at(2)).is_err());
    assert!(batch.log_undo(0).is_err(), "an undo must be started first");
}

#[test]
fn prunes_the_folder_a_file_left_or_failed_to_fill() {
    let m = planned(1);
    assert_eq!(MoveOutcome::Moved.folder_to_prune(&m), Some(path("RPG")));
    assert_eq!(
        MoveOutcome::Failed("x".into()).folder_to_prune(&m),
        Some(path("04 角色扮演(RPG)/未知社團"))
    );
    assert_eq!(MoveOutcome::Skipped("x".into()).folder_to_prune(&m), None);
}

#[test]
fn undo_walks_moved_ops_newest_first_and_only_once() {
    let mut batch = batch_with(&[
        MoveOutcome::Moved,
        MoveOutcome::Failed("x".into()),
        MoveOutcome::Moved,
    ]);
    assert_eq!(
        batch.start_undo().unwrap_err(),
        MoveBatchError::NotFinished(batch.id())
    );
    let _finished = batch.finish(vec![], vec![], at(1)).unwrap();
    assert_eq!(batch.moved_ops_newest_first(), vec![2, 0]);
    let _undoing = batch.start_undo().unwrap();
    let _logged = batch.log_undo(2).unwrap();
    let _settled = batch
        .settle_undo(
            2,
            UndoOutcome::Skipped("the original location is taken".into()),
        )
        .unwrap();
    let _logged = batch.log_undo(0).unwrap();
    let _settled = batch.settle_undo(0, UndoOutcome::Undone).unwrap();
    let _undone = batch
        .finish_undo(vec!["04 角色扮演(RPG)/未知社團".into()], vec![], at(2))
        .unwrap();
    assert_eq!(batch.status(), MoveBatchStatus::Undone);
    assert_eq!(
        batch.notes().notes(),
        ["04 角色扮演(RPG)/未知社團/2.zip: the original location is taken"]
    );
    assert_eq!(
        batch.counts(),
        MoveCounts::new(2, 0, 1),
        "undo keeps the run's counts"
    );
    assert!(batch.start_undo().is_err());
}

#[test]
fn a_pending_move_follows_the_file() {
    let (from, to) = (path("RPG/0.zip"), path("04 角色扮演(RPG)/未知社團/0.zip"));
    let cases = [
        // (from, to, same) → status, link to remove, record moved along
        (
            (false, true, false),
            MoveOpStatus::Moved,
            None,
            Some((&from, &to)),
        ),
        (
            (true, true, true),
            MoveOpStatus::Failed,
            Some(&to),
            Some((&to, &from)),
        ),
        (
            (true, false, false),
            MoveOpStatus::Failed,
            None,
            Some((&to, &from)),
        ),
        ((false, false, false), MoveOpStatus::Failed, None, None),
    ];
    for ((source, target, same), status, remove, record_move) in cases {
        let mut batch = started();
        let _logged = batch.log_move(op_id(0), &planned(0)).unwrap();
        let (recovery, _reconciled) = batch
            .reconcile_op(0, MoveProbe::new(source, target, same))
            .unwrap();
        assert_eq!(batch.op(0).unwrap().status(), status);
        assert_eq!(recovery.remove_link(), remove);
        // The record still says `from`; it moves when the file is elsewhere.
        assert_eq!(
            recovery.record_move(&from),
            record_move.filter(|(_, t)| *t != &from)
        );
    }
}

#[test]
fn an_interrupted_undo_keeps_files_at_their_target_undoable() {
    let mut batch = batch_with(&[MoveOutcome::Moved, MoveOutcome::Moved]);
    let _finished = batch.finish(vec![], vec![], at(1)).unwrap();
    let _undoing = batch.start_undo().unwrap();
    let _logged = batch.log_undo(1).unwrap();
    assert_eq!(batch.interrupted_ops(), vec![1]);
    let (recovery, _reconciled) = batch
        .reconcile_op(1, MoveProbe::new(true, true, true))
        .unwrap();
    assert_eq!(recovery.remove_link(), Some(batch.op(1).unwrap().from()));
    assert_eq!(batch.op(1).unwrap().status(), MoveOpStatus::Moved);
    let _recovered = batch.recover(at(3));
    assert_eq!(batch.status(), MoveBatchStatus::Done);
    assert_eq!(batch.times().undone_at(), Some(at(3)));
}

#[test]
fn an_interrupted_run_is_counted_from_its_ops() {
    let mut batch = started();
    let first = batch.log_move(op_id(0), &planned(0)).unwrap();
    let _settled = batch.settle_move(first.seq, MoveOutcome::Moved).unwrap();
    let _second = batch.log_move(op_id(1), &planned(1)).unwrap();
    let (_recovery, _reconciled) = batch
        .reconcile_op(1, MoveProbe::new(false, true, false))
        .unwrap();
    let _recovered = batch.recover(at(5));
    assert_eq!(batch.status(), MoveBatchStatus::Done);
    assert_eq!(batch.counts(), MoveCounts::new(2, 0, 0));
    assert_eq!(batch.times().finished_at(), Some(at(5)));
    assert_eq!(
        batch.notes().notes(),
        ["the batch was interrupted and reconciled against the disk"]
    );
}
