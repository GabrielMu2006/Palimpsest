// Authored by opencode (AI coding agent) — P1-KERNEL-REPAIR.
//! CHRON-027–029 repair regression suites (KFIX-001..006, ADR-0024).
//!
//! These reproduce the reviewed findings F01–F06 against the shared repair
//! fixture and assert the repaired behavior. They are a separate fixture from
//! the ADR-0018 closed-loop and the generator goldens; they must not be edited
//! to dodge a collision.

mod common;

use palimpsest_sim_ai::Needs;
use palimpsest_sim_core::{
    ActionError, ActionState, EntityId, KernelError, Transition, TransitionReason, WorldKernel,
};

use common::repair_fixture::{
    RepairFixture, action_config_with_work, at, candidate, default_action_config, default_weights,
    zero_perturbation,
};

// ---------------------------------------------------------------------------
// KFIX-001 — rejection is side-effect-free (F02a / F02b and adjacent paths)
// ---------------------------------------------------------------------------

#[test]
fn kfix_001_blocked_start_at_nonzero_leaves_needs_and_state_unchanged() {
    // F02a: a Blocked start at a non-zero instant must not materialize Needs
    // or alter any prior state.
    let mut fixture = RepairFixture::new();
    let person = fixture.spawn();
    let work = fixture.work;
    let mut runtime = palimpsest_sim_core::ActionRuntime::default();
    let mut env = fixture.env();
    let result = runtime.start(
        person,
        candidate(palimpsest_sim_ai::ActionKind::Eat, Some(work)),
        &mut env,
        at(10),
    );
    assert_eq!(
        result,
        Err(ActionError::Blocked {
            kind: palimpsest_sim_ai::ActionKind::Eat,
            target: work,
        })
    );
    assert_eq!(
        env.persons.needs(person).expect("person exists"),
        Needs::default(),
        "a rejected start must not materialize Needs",
    );
    assert_eq!(runtime.current(person), None);
    assert_eq!(runtime.current_action(person), None);
    assert_eq!(runtime.metrics().scheduler.scheduled_entries, 0);
    assert!(runtime.next_due().is_none());
    assert_eq!(runtime.stats().started, 0);
}

#[test]
fn kfix_001_cancel_backwards_keeps_idle_and_token() {
    // F02b: cancelling with a reversed time must be a no-op that keeps the
    // active Idle action and its live continuation token.
    let mut fixture = RepairFixture::new();
    let person = fixture.spawn();
    let mut runtime = palimpsest_sim_core::ActionRuntime::default();
    {
        let mut env = fixture.env();
        runtime
            .start(
                person,
                candidate(palimpsest_sim_ai::ActionKind::Idle, None),
                &mut env,
                at(10),
            )
            .expect("start idle");
    }
    let before = runtime.metrics();
    {
        let mut env = fixture.env();
        let result = runtime.cancel(
            person,
            palimpsest_sim_core::CancelReason::External,
            at(0),
            &mut env,
        );
        assert_eq!(result, Err(ActionError::TimeOverflow { id: person }));
    }
    assert_eq!(
        runtime.metrics(),
        before,
        "a rejected cancel mutates no live token or queue"
    );
    assert_eq!(runtime.current(person), Some(ActionState::Idle));
    assert_eq!(
        runtime.current_action(person),
        Some((palimpsest_sim_ai::ActionKind::Idle, None))
    );
    assert_eq!(runtime.metrics().live_actions, 1);
    // The Idle wait (60s from t=10) still completes, proving the token is intact.
    let out = {
        let mut env = fixture.env();
        runtime
            .advance(at(70), &mut env)
            .expect("idle wait completes")
    };
    assert_eq!(
        out.transitions().last().map(Transition::reason),
        Some(TransitionReason::Completed),
    );
    assert_eq!(runtime.stats().idle_completions, 1);
}

#[test]
fn kfix_001_unknown_and_duplicate_rejections_leave_state_unchanged() {
    let mut fixture = RepairFixture::new();
    let person = fixture.spawn();
    let work = fixture.work;
    let missing = EntityId::new(999).expect("non-zero");
    let mut runtime = palimpsest_sim_core::ActionRuntime::default();
    let before = runtime.metrics();
    {
        let mut env = fixture.env();
        assert_eq!(
            runtime.start(
                missing,
                candidate(palimpsest_sim_ai::ActionKind::Work, Some(work)),
                &mut env,
                at(0)
            ),
            Err(ActionError::UnknownPerson { id: missing }),
        );
    }
    assert_eq!(runtime.metrics(), before);

    // AlreadyExecuting: start Work, then a second start must be rejected and
    // the running action preserved.
    {
        let mut env = fixture.env();
        runtime
            .start(
                person,
                candidate(palimpsest_sim_ai::ActionKind::Work, Some(work)),
                &mut env,
                at(0),
            )
            .expect("start work");
    }
    {
        let mut env = fixture.env();
        assert_eq!(
            runtime.start(
                person,
                candidate(palimpsest_sim_ai::ActionKind::Idle, None),
                &mut env,
                at(0)
            ),
            Err(ActionError::AlreadyExecuting { id: person }),
        );
    }
    assert_eq!(
        runtime.current_action(person).map(|(kind, _)| kind),
        Some(palimpsest_sim_ai::ActionKind::Work),
    );
    assert_eq!(runtime.stats().started, 1);
}

#[test]
fn kfix_001_rejected_start_does_not_cancel_a_pending_retry() {
    // A pending retry token must survive a subsequent rejected start.
    let mut fixture = RepairFixture::new();
    let person = fixture.spawn();
    let work = fixture.work;
    let meal = fixture.meal;
    let mut runtime = palimpsest_sim_core::ActionRuntime::default();
    let mut empty = palimpsest_sim_world::ActivitySites::new(Vec::new()).expect("empty sites");
    {
        let mut env = fixture.env();
        runtime
            .start(
                person,
                candidate(palimpsest_sim_ai::ActionKind::Eat, Some(meal)),
                &mut env,
                at(0),
            )
            .expect("start eat");
        runtime.advance(at(1), &mut env).expect("step");
    }
    {
        let mut env = palimpsest_sim_core::ActionEnvironment {
            persons: &mut fixture.persons,
            map: &fixture.map,
            sites: &mut empty,
        };
        runtime.advance(at(2), &mut env).expect("arrival recheck");
    }
    assert_eq!(
        runtime.metrics().pending_retries,
        1,
        "a blocked arrival set a retry token"
    );
    // A rejected Eat(Work-site) start must not cancel that retry token.
    {
        let mut env = palimpsest_sim_core::ActionEnvironment {
            persons: &mut fixture.persons,
            map: &fixture.map,
            sites: &mut empty,
        };
        assert_eq!(
            runtime.start(
                person,
                candidate(palimpsest_sim_ai::ActionKind::Eat, Some(work)),
                &mut env,
                at(2)
            ),
            Err(ActionError::Blocked {
                kind: palimpsest_sim_ai::ActionKind::Eat,
                target: work,
            }),
        );
    }
    assert_eq!(
        runtime.metrics().pending_retries,
        1,
        "a rejected start must preserve a pending retry token",
    );
    // The retry still fires one second later.
    let out = {
        let mut env = palimpsest_sim_core::ActionEnvironment {
            persons: &mut fixture.persons,
            map: &fixture.map,
            sites: &mut empty,
        };
        runtime.advance(at(3), &mut env).expect("retry pop")
    };
    assert!(
        out.decision_requests()
            .iter()
            .any(|request| request.reason() == palimpsest_sim_core::DecisionReason::Retry),
        "the preserved retry token still surfaces a Retry decision",
    );
}

// ---------------------------------------------------------------------------
// KFIX-002 — one decision per (person, instant) (F01)
// ---------------------------------------------------------------------------

#[test]
fn kfix_002_kernel_advances_past_a_work_completion_and_critical_check_same_instant() {
    // F01: a 45,000-second Work completion coincides with the critical-fatigue
    // check. The kernel must not fail with AlreadyExecuting; exactly one Sleep
    // is started and the clock commits to 45,000.
    let config = palimpsest_sim_core::KernelConfig::new(
        action_config_with_work(44_999),
        default_weights(),
        zero_perturbation(),
        palimpsest_sim_core::DEFAULT_WORK_BUDGET,
        palimpsest_sim_core::DEFAULT_EVENT_BUFFER_CAPACITY,
    )
    .expect("valid config");
    let fixture = RepairFixture::new();
    let mut kernel = WorldKernel::new(fixture.map, fixture.sites, config);
    let person = kernel.spawn_person(fixture.origin).expect("spawn");
    kernel.start_world(at(0)).expect("start world");
    let advance = kernel
        .advance_to(at(45_000), palimpsest_sim_core::DEFAULT_WORK_BUDGET)
        .expect("the kernel reaches 45,000 without an AlreadyExecuting error");
    assert!(advance.reached_target());
    assert_eq!(advance.committed_to(), at(45_000));
    let view = kernel
        .person(person)
        .expect("complete-boundary read")
        .expect("person exists");
    assert_eq!(
        view.action(),
        palimpsest_sim_ai::ActionKind::Sleep,
        "critical fatigue selects Sleep immediately after the Work completion",
    );
}

#[test]
fn kfix_002_shared_driver_completes_work_once_and_starts_sleep_without_interrupt() {
    // The shared batch driver yields exactly one Work completion, no fake
    // interrupted Work, and a subsequent Sleep start at the same instant.
    let mut fixture = RepairFixture::new();
    let person = fixture.spawn();
    let mut runtime = palimpsest_sim_core::ActionRuntime::new(action_config_with_work(44_999));
    let weights = default_weights();
    let spec = zero_perturbation();
    {
        let mut env = fixture.env();
        palimpsest_sim_core::decide_and_start(
            &mut runtime,
            person,
            &mut env,
            &weights,
            &spec,
            at(0),
        )
        .expect("initial decision");
    }
    let mut env = fixture.env();
    palimpsest_sim_core::run_until(&mut runtime, &mut env, at(45_000), &weights, &spec)
        .expect("the shared driver runs the loop");
    let stats = runtime.stats();
    assert_eq!(stats.work_completions, 1, "Work completed exactly once");
    assert_eq!(stats.interrupted, 0, "no action was falsely interrupted");
    assert_eq!(
        runtime.current_action(person).map(|(kind, _)| kind),
        Some(palimpsest_sim_ai::ActionKind::Sleep),
        "the merged decision starts Sleep",
    );
    assert!(
        runtime.current(person).is_some(),
        "Sleep is now executing (movement phase at the same instant)"
    );
}

// ---------------------------------------------------------------------------
// KFIX-003 — lifecycle, Result read API, and full-boundary guards
// ---------------------------------------------------------------------------

#[test]
fn kfix_003_non_empty_setup_forward_advance_is_rejected() {
    let fixture = RepairFixture::new();
    let mut kernel = WorldKernel::new(
        fixture.map,
        fixture.sites,
        palimpsest_sim_core::KernelConfig::default(),
    );
    kernel.spawn_person(fixture.origin).expect("spawn");
    // A forward advance before start_world is a recoverable NotStarted rejection.
    assert_eq!(
        kernel.advance_to(at(100), palimpsest_sim_core::DEFAULT_WORK_BUDGET),
        Err(KernelError::NotStarted),
    );
    // An equal-target no-op stays in Setup and mutates nothing.
    kernel
        .advance_to(at(0), palimpsest_sim_core::DEFAULT_WORK_BUDGET)
        .expect("equal target is a no-op");
    assert_eq!(kernel.state(), palimpsest_sim_core::KernelState::Setup);
}

#[test]
fn kfix_003_zero_budget_is_rejected() {
    let fixture = RepairFixture::new();
    let mut kernel = WorldKernel::new(
        fixture.map,
        fixture.sites,
        palimpsest_sim_core::KernelConfig::default(),
    );
    kernel.spawn_person(fixture.origin).expect("spawn");
    assert_eq!(kernel.advance_to(at(1), 0), Err(KernelError::InvalidBudget),);
}

#[test]
fn kfix_003_read_api_returns_result_shapes() {
    let fixture = RepairFixture::new();
    let mut kernel = WorldKernel::new(
        fixture.map,
        fixture.sites,
        palimpsest_sim_core::KernelConfig::default(),
    );
    let person = kernel.spawn_person(fixture.origin).expect("spawn");
    kernel.start_world(at(0)).expect("start");
    kernel
        .advance_to(at(10), palimpsest_sim_core::DEFAULT_WORK_BUDGET)
        .expect("advance");
    // A known person is Ok(Some(view)); an unknown id is Ok(None).
    assert!(kernel.person(person).expect("running read").is_some());
    assert!(
        kernel
            .person(palimpsest_sim_core::EntityId::new(999_999).expect("non-zero"))
            .expect("running read")
            .is_none()
    );
    // persons() returns a Vec (empty only for an empty world).
    let views = kernel.persons().expect("running read");
    assert_eq!(views.len(), 1);
    // latest_trace returns a Result with an Option inside.
    let _ = kernel.latest_trace(person).expect("running read");
}

// ---------------------------------------------------------------------------
// KFIX-004 — current-instant Needs projection (F03)
// ---------------------------------------------------------------------------

#[test]
fn kfix_004_kernel_person_view_projects_needs_to_now() {
    // F03: a real Kernel using the repair fixture (Work at origin, reachable)
    // projects Needs to the kernel's committed instant: 100 / 200 at t=100.
    let fixture = RepairFixture::new();
    let mut kernel = WorldKernel::new(
        fixture.map,
        fixture.sites,
        palimpsest_sim_core::KernelConfig::default(),
    );
    let person = kernel.spawn_person(fixture.origin).expect("spawn");
    kernel.start_world(at(0)).expect("start");
    kernel
        .advance_to(at(100), palimpsest_sim_core::DEFAULT_WORK_BUDGET)
        .expect("advance to 100");
    let due_before = kernel.next_due();
    let view = kernel
        .person(person)
        .expect("running read")
        .expect("person exists");
    assert_eq!(view.needs().hunger().raw(), 100);
    assert_eq!(view.needs().fatigue().raw(), 200);
    // Repeated reads are identical and never mutate scheduling or truth.
    for _ in 0..100 {
        let again = kernel.person(person).expect("read").expect("person");
        assert_eq!(again.needs().hunger().raw(), 100);
        assert_eq!(again.needs().fatigue().raw(), 200);
    }
    assert_eq!(
        kernel.next_due(),
        due_before,
        "reads must not schedule or change work"
    );
    // A forward step continues from the same single baseline (no double-count).
    kernel
        .advance_to(at(101), palimpsest_sim_core::DEFAULT_WORK_BUDGET)
        .expect("advance to 101");
    let view = kernel.person(person).expect("read").expect("person");
    assert_eq!(view.needs().hunger().raw(), 101);
    assert_eq!(view.needs().fatigue().raw(), 202);
}

// ---------------------------------------------------------------------------
// KFIX-005 — cumulative event total/digest and two-level rotation (F04)
// ---------------------------------------------------------------------------

#[test]
fn kfix_005_4097_events_count_total_retained_and_rotated_exactly() {
    // F04: 4,097 same-instant completions produce 4,097 events; the default
    // retention buffer keeps 4,096 and one is dropped; total never regresses.
    // (Work duration 1s so the completions land at t=2; the accounting is
    // independent of the exact completion instant.)
    let config = palimpsest_sim_core::KernelConfig::new(
        action_config_with_work(1),
        default_weights(),
        zero_perturbation(),
        palimpsest_sim_core::DEFAULT_WORK_BUDGET,
        palimpsest_sim_core::DEFAULT_EVENT_BUFFER_CAPACITY,
    )
    .expect("config");
    let fixture = RepairFixture::new();
    let mut kernel = WorldKernel::new(fixture.map, fixture.sites, config);
    for _ in 0..4_097 {
        kernel.spawn_person(fixture.origin).expect("spawn");
    }
    kernel.start_world(at(0)).expect("start");
    kernel
        .advance_to(at(2), palimpsest_sim_core::DEFAULT_WORK_BUDGET)
        .expect("advance");
    let metrics = kernel.metrics();
    assert_eq!(
        metrics.events_total, 4_097,
        "total counts every produced event"
    );
    assert_eq!(
        metrics.events_buffered, 4_096,
        "the retained buffer holds its capacity"
    );
    assert_eq!(
        metrics.events_rotated, 1,
        "exactly one event is lost across both buffers"
    );
    // The accounting identity holds: delivered + retained + rotated == total.
    assert_eq!(
        metrics.events_total,
        metrics.events_buffered as u64 + metrics.events_rotated
    );
    let drained = kernel.drain_events();
    assert_eq!(drained.len(), 4_096);
    for event in &drained {
        assert!(event.validate().is_ok());
    }
}

#[test]
fn kfix_005_event_digest_is_segmentation_invariant() {
    // The stream digest must be identical whether the same advance is done in
    // one call or split into budget-1 calls (ADR-0024 D5).
    fn run(budget: usize) -> u64 {
        let config = palimpsest_sim_core::KernelConfig::new(
            default_action_config(),
            default_weights(),
            zero_perturbation(),
            budget,
            palimpsest_sim_core::DEFAULT_EVENT_BUFFER_CAPACITY,
        )
        .expect("config");
        let fixture = RepairFixture::new();
        let mut kernel = WorldKernel::new(fixture.map, fixture.sites, config);
        kernel.spawn_person(fixture.origin).expect("spawn");
        kernel.start_world(at(0)).expect("start");
        let target = at(86_400);
        loop {
            let advance = kernel.advance_to(target, budget).expect("advance");
            kernel.drain_events();
            if advance.reached_target() {
                break;
            }
        }
        kernel.metrics().events_digest
    }
    let one = run(usize::MAX);
    let segmented = run(1);
    assert_eq!(
        one, segmented,
        "digest must not depend on advance segmentation"
    );
    assert_ne!(
        one,
        crate_digest_offset(),
        "a non-empty digest differs from the offset basis"
    );
}

fn crate_digest_offset() -> u64 {
    14_695_981_039_346_656_037
}

// ---------------------------------------------------------------------------
// KFIX-006 — schema 2, ActivitySite/Needs, and full DTO validation (F06/G01)
// ---------------------------------------------------------------------------

fn running_kernel() -> (WorldKernel, palimpsest_sim_core::EntityId) {
    let fixture = RepairFixture::new();
    let mut kernel = WorldKernel::new(
        fixture.map,
        fixture.sites,
        palimpsest_sim_core::KernelConfig::default(),
    );
    let person = kernel.spawn_person(fixture.origin).expect("spawn");
    kernel.start_world(at(0)).expect("start");
    kernel
        .advance_to(at(10), palimpsest_sim_core::DEFAULT_WORK_BUDGET)
        .expect("advance");
    (kernel, person)
}

#[test]
fn kfix_006_snapshot_includes_real_sites_and_projected_needs() {
    let (kernel, _) = running_kernel();
    let snapshot = palimpsest_sim_core::RenderSnapshot::from_kernel(&kernel)
        .expect("snapshot from a complete boundary");
    assert_eq!(snapshot.schema_version(), 2);
    assert_eq!(
        snapshot.sites().len(),
        3,
        "the repair fixture has three sites"
    );
    for site in snapshot.sites() {
        assert!(site.coord().x() < 128 && site.coord().y() < 128);
        // Every site coord is walkable on the batched terrain.
        let index = site.coord().index();
        assert!(snapshot.terrain().cells()[index].is_walkable());
    }
    // A person carries projected Needs at the snapshot instant.
    let person = snapshot.persons().first().expect("a person");
    assert!(person.person_id().get() != 0);
    // At t=10, the projected Needs are hunger 10 / fatigue 20.
    assert_eq!(person.needs().hunger().raw(), 10);
    assert_eq!(person.needs().fatigue().raw(), 20);
}

#[test]
fn kfix_006_schema_one_and_bad_dimensions_are_rejected() {
    let (kernel, _) = running_kernel();
    let snapshot = palimpsest_sim_core::RenderSnapshot::from_kernel(&kernel).expect("snapshot");
    let base = serde_json::to_value(&snapshot).expect("encode");

    let mut bad = base.clone();
    bad["schema_version"] = serde_json::json!(1);
    assert!(serde_json::from_value::<palimpsest_sim_core::RenderSnapshot>(bad).is_err());
    let mut bad = base.clone();
    bad["schema_version"] = serde_json::json!(99);
    assert!(serde_json::from_value::<palimpsest_sim_core::RenderSnapshot>(bad).is_err());

    for (field, value) in [
        ("width", 0_u64),
        ("width", 127_u64),
        ("width", 129_u64),
        ("height", 0_u64),
        ("height", 128_u64 - 1),
        ("height", 129_u64),
    ] {
        let mut bad = base.clone();
        bad["terrain"][field] = serde_json::json!(value);
        assert!(
            serde_json::from_value::<palimpsest_sim_core::RenderSnapshot>(bad).is_err(),
            "terrain {field}={value} must be rejected"
        );
    }
    // Wrong cell count, even with correct dimensions, is rejected.
    let mut bad = base.clone();
    bad["terrain"]["cells"] = serde_json::json!([0]);
    assert!(serde_json::from_value::<palimpsest_sim_core::RenderSnapshot>(bad).is_err());
}

#[test]
fn kfix_006_action_state_target_correlation_is_enforced() {
    let (kernel, _) = running_kernel();
    let snapshot = palimpsest_sim_core::RenderSnapshot::from_kernel(&kernel).expect("snapshot");
    let base = serde_json::to_value(&snapshot).expect("encode");

    // Eating but action Idle / no target -> reject.
    let mut bad = base.clone();
    bad["persons"][0]["action_state"] = serde_json::json!("Eating");
    bad["persons"][0]["action"] = serde_json::json!("Idle");
    assert!(serde_json::from_value::<palimpsest_sim_core::RenderSnapshot>(bad).is_err());

    // Idle state with a target -> reject.
    let mut bad = base.clone();
    bad["persons"][0]["action_state"] = serde_json::json!("Idle");
    bad["persons"][0]["action"] = serde_json::json!("Idle");
    bad["persons"][0]["action_target"] = serde_json::json!({"x": 1, "y": 1});
    assert!(serde_json::from_value::<palimpsest_sim_core::RenderSnapshot>(bad).is_err());
}

#[test]
fn kfix_006_site_duplicate_and_unwalkable_are_rejected() {
    let (kernel, _) = running_kernel();
    let snapshot = palimpsest_sim_core::RenderSnapshot::from_kernel(&kernel).expect("snapshot");
    let base = serde_json::to_value(&snapshot).expect("encode");

    // Duplicate site coordinate.
    let mut bad = base.clone();
    let first = bad["sites"][0].clone();
    bad["sites"][1] = first;
    assert!(serde_json::from_value::<palimpsest_sim_core::RenderSnapshot>(bad).is_err());

    // An unwalkable site coordinate (a non-walkable terrain cell) is rejected.
    let unwalkable = kernel
        .map()
        .local()
        .coords()
        .find(|coord| {
            !kernel
                .map()
                .local()
                .get(coord.x(), coord.y())
                .expect("in bounds")
                .is_walkable()
        })
        .expect("a non-walkable cell exists");
    let mut bad = base.clone();
    bad["sites"][0]["coord"] = serde_json::json!({"x": unwalkable.x(), "y": unwalkable.y()});
    assert!(serde_json::from_value::<palimpsest_sim_core::RenderSnapshot>(bad).is_err());
}

#[test]
fn kfix_006_repeated_build_is_immutable_and_empty_world_is_valid() {
    let (kernel, _) = running_kernel();
    let before = kernel.now();
    let first = palimpsest_sim_core::RenderSnapshot::from_kernel(&kernel).expect("snapshot");
    let second = palimpsest_sim_core::RenderSnapshot::from_kernel(&kernel).expect("snapshot");
    assert_eq!(first, second);
    assert_eq!(kernel.now(), before, "building a snapshot mutates nothing");

    // An empty (no persons) running world still yields a valid snapshot.
    let empty_fixture = RepairFixture::new();
    let mut empty_kernel = WorldKernel::new(
        empty_fixture.map,
        empty_fixture.sites,
        palimpsest_sim_core::KernelConfig::default(),
    );
    // advance an empty kernel to Running first (allowed).
    empty_kernel
        .advance_to(at(5), palimpsest_sim_core::DEFAULT_WORK_BUDGET)
        .expect("empty advance");
    let snap =
        palimpsest_sim_core::RenderSnapshot::from_kernel(&empty_kernel).expect("empty snapshot");
    assert_eq!(snap.person_count(), 0);
    assert!(snap.validate().is_ok());
}
