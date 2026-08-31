//! Focused R2-01 regression probes.  The broader closed-loop and kernel
//! suites remain the compatibility coverage; these tests pin the repaired
//! action-boundary invariants independently.

mod common;

use common::repair_fixture::{at, candidate};
use palimpsest_sim_ai::ActionKind;
use palimpsest_sim_core::{ActionError, ActionRuntime, CancelReason};

#[test]
fn start_before_epoch_does_not_create_a_runtime_entry() {
    let mut fixture = common::repair_fixture::RepairFixture::new();
    let person = fixture.spawn();
    let mut runtime = ActionRuntime::default();
    let action = candidate(ActionKind::Move, Some(fixture.meal));
    let result = runtime.start(person, action, &mut fixture.env(), at(-1));
    assert_eq!(result, Err(ActionError::TimeOverflow { id: person }));
    assert_eq!(runtime.current_action(person), None);
    assert_eq!(runtime.metrics().scheduler.scheduled_entries, 0);
}

#[test]
fn cancel_rejects_a_time_watermark_overflow_without_side_effects() {
    let mut fixture = common::repair_fixture::RepairFixture::new();
    let person = fixture.spawn();
    let mut runtime = ActionRuntime::default();
    let action = candidate(ActionKind::Idle, None);
    runtime
        .start(person, action, &mut fixture.env(), at(0))
        .expect("idle starts");
    let before = runtime.metrics();
    let result = runtime.cancel(
        person,
        CancelReason::External,
        at(i64::MAX),
        &mut fixture.env(),
    );
    assert_eq!(result, Err(ActionError::TimeOverflow { id: person }));
    assert_eq!(
        runtime.current_action(person),
        Some((ActionKind::Idle, None))
    );
    assert_eq!(runtime.metrics(), before);
    runtime
        .advance(at(60), &mut fixture.env())
        .expect("original Idle token survives");
    assert_eq!(runtime.stats().idle_completions, 1);
    assert!(runtime.drain_events().is_empty());
    assert_eq!(fixture.persons.needs(person).unwrap().fatigue().raw(), 120);
}

#[test]
fn movement_step_records_a_commit_watermark() {
    let mut fixture = common::repair_fixture::RepairFixture::new();
    let person = fixture.spawn();
    let mut runtime = ActionRuntime::default();
    let action = candidate(ActionKind::Move, Some(fixture.meal));
    runtime
        .start(person, action, &mut fixture.env(), at(0))
        .expect("move starts");
    runtime
        .advance(at(1), &mut fixture.env())
        .expect("first step");
    let result = runtime.cancel(person, CancelReason::External, at(0), &mut fixture.env());
    assert_eq!(result, Err(ActionError::TimeOverflow { id: person }));
    runtime
        .advance(at(2), &mut fixture.env())
        .expect("original movement continues");
    assert_eq!(runtime.stats().move_completions, 1);
    assert_eq!(fixture.persons.location(person), Some(fixture.meal));
    assert_eq!(runtime.drain_events().len(), 1);
}

#[test]
fn arrival_also_advances_the_commit_watermark_without_growing_needs_twice() {
    let mut fixture = common::repair_fixture::RepairFixture::new();
    let person = fixture.spawn();
    let mut runtime = ActionRuntime::default();
    runtime
        .start(
            person,
            candidate(ActionKind::Work, Some(fixture.work)),
            &mut fixture.env(),
            at(0),
        )
        .unwrap();
    runtime.advance(at(1), &mut fixture.env()).unwrap();
    let before = runtime.metrics();
    assert_eq!(
        runtime.cancel(person, CancelReason::External, at(0), &mut fixture.env()),
        Err(ActionError::TimeOverflow { id: person })
    );
    assert_eq!(runtime.metrics(), before);
    runtime.advance(at(1801), &mut fixture.env()).unwrap();
    assert_eq!(runtime.stats().work_completions, 1);
    assert_eq!(fixture.persons.needs(person).unwrap().hunger().raw(), 1801);
}

#[test]
fn upper_bound_start_rejects_before_any_boundary_mutation() {
    let mut fixture = common::repair_fixture::RepairFixture::new();
    let person = fixture.spawn();
    let mut runtime = ActionRuntime::default();
    let before_needs = fixture.persons.needs(person).expect("needs");
    let before_stats = runtime.stats();
    let before_metrics = runtime.metrics();
    let result = runtime.start(
        person,
        candidate(ActionKind::Move, Some(fixture.meal)),
        &mut fixture.env(),
        at(i64::MAX - 1),
    );
    assert_eq!(result, Err(ActionError::TimeOverflow { id: person }));
    assert_eq!(fixture.persons.needs(person), Some(before_needs));
    assert_eq!(runtime.stats(), before_stats);
    assert_eq!(runtime.metrics(), before_metrics);
    assert_eq!(runtime.current_action(person), None);
    assert!(runtime.drain_events().is_empty());
}

#[test]
fn rejected_earlier_start_leaves_pending_retry_to_fire_once() {
    let mut fixture = common::repair_fixture::RepairFixture::new();
    let person = fixture.spawn();
    let mut runtime = ActionRuntime::default();
    runtime
        .start(
            person,
            candidate(ActionKind::Eat, Some(fixture.meal)),
            &mut fixture.env(),
            at(0),
        )
        .expect("start");
    runtime.advance(at(1), &mut fixture.env()).expect("step");
    let mut empty = palimpsest_sim_world::ActivitySites::new(Vec::new()).expect("empty sites");
    runtime
        .advance(
            at(2),
            &mut palimpsest_sim_core::ActionEnvironment {
                persons: &mut fixture.persons,
                map: &fixture.map,
                sites: &mut empty,
            },
        )
        .expect("blocked arrival");
    let rejected = runtime.start(
        person,
        candidate(ActionKind::Idle, None),
        &mut palimpsest_sim_core::ActionEnvironment {
            persons: &mut fixture.persons,
            map: &fixture.map,
            sites: &mut empty,
        },
        at(1),
    );
    assert_eq!(rejected, Err(ActionError::TimeOverflow { id: person }));
    assert_eq!(runtime.metrics().pending_retries, 1);
    let retry = runtime
        .advance(
            at(3),
            &mut palimpsest_sim_core::ActionEnvironment {
                persons: &mut fixture.persons,
                map: &fixture.map,
                sites: &mut empty,
            },
        )
        .expect("retry");
    assert_eq!(retry.decision_requests().len(), 1);
    assert_eq!(runtime.metrics().pending_retries, 0);
}
