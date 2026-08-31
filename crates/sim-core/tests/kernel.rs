// Authored by opencode (AI coding agent) — task CHRON-028.
//! Kernel orchestration integration tests (CHRON-028, ADR-0022).
//!
//! These exercise the [`WorldKernel`] as the single owner of time and
//! ordering: determinism across repeated runs, clock-regression and
//! equal-target behavior, budget/reached semantics, segmentation equivalence,
//! the deterministic no-full-scan cadence, per-person `EntityId`
//! addressability, decision-trace retention, and bounded event accounting.

use palimpsest_sim_core::{
    EntityId, KernelAdvance, KernelConfig, KernelError, KernelPersonView, SimInstant, WorldKernel,
};
use palimpsest_sim_world::{LocalCoord, WorldGenConfig, WorldMap, WorldSeed};

/// Locked fixture seed shared with the ADR-0018 reference context.
const FIXTURE_SEED: u64 = 25_025;
/// One simulation day in seconds; long enough to cross need thresholds.
const DAY: i64 = 86_400;
/// Two simulated days used by the determinism fixture.
const TWO_DAYS: i64 = 172_800;

fn at(value: i64) -> SimInstant {
    SimInstant::from_seconds(value)
}

/// Origin of a fully walkable 3×3 block inside the spawn clearing.
fn walkable_block_origin(map: &WorldMap) -> LocalCoord {
    map.local()
        .coords()
        .find(|origin| {
            (0..3).all(|dy| {
                (0..3).all(|dx| {
                    LocalCoord::new(origin.x() + dx, origin.y() + dy).is_some_and(|coord| {
                        map.local()
                            .get(coord.x(), coord.y())
                            .is_some_and(|kind| kind.is_walkable())
                    })
                })
            })
        })
        .expect("spawn clearing contains a 3x3 walkable block")
}

/// A kernel seeded with the locked reference world and one person spawned at
/// the walkable-block origin.
fn seeded_kernel(persons: usize) -> WorldKernel {
    let mut kernel = WorldKernel::from_world(WorldSeed::new(FIXTURE_SEED), KernelConfig::default());
    let map = WorldMap::generate(WorldSeed::new(FIXTURE_SEED), WorldGenConfig::default());
    let origin = walkable_block_origin(&map);
    for _ in 0..persons {
        kernel
            .spawn_person(origin)
            .expect("identity capacity for the fixture population");
    }
    kernel
}

/// Spins the kernel to `target` with the configured budget, asserting every
/// advance call succeeds and returns a consistent `committed_to`.
fn spin_to(kernel: &mut WorldKernel, target: SimInstant) -> KernelAdvance {
    loop {
        let advance = kernel.advance(target).expect("bounded advance succeeds");
        if advance.reached_target() {
            return advance;
        }
        assert!(
            advance.committed_to() < target,
            "yielded boundary is below target"
        );
    }
}

#[test]
fn clock_regression_is_rejected_without_mutation() {
    let mut kernel = seeded_kernel(1);
    kernel.start_world(at(0)).expect("start");
    let before = kernel.now();
    assert_eq!(
        kernel.advance_to(at(-1), 10),
        Err(KernelError::ClockRegression {
            current: at(0),
            requested: at(-1),
        })
    );
    assert_eq!(
        kernel.now(),
        before,
        "a rejected target never mutates the clock"
    );
}

#[test]
fn equal_target_advance_is_a_noop_after_committing() {
    let mut kernel = seeded_kernel(2);
    kernel.start_world(at(0)).expect("start");
    let first = spin_to(&mut kernel, at(10));
    assert!(first.reached_target());
    let now = kernel.now();
    assert_eq!(now, at(10));
    // Advancing to the already-committed instant processes nothing further.
    let second = kernel.advance_to(now, 10).expect("same target is allowed");
    assert!(second.reached_target());
    assert_eq!(second.committed_to(), now);
    assert_eq!(second.rounds(), 0);
    // A fresh, later step produces strictly more committed work.
    let third = spin_to(&mut kernel, at(30));
    assert_eq!(third.committed_to(), at(30));
    assert!(kernel.now() > now);
}

#[test]
fn budget_exhaustion_yields_and_is_resumable_to_the_same_truth() {
    // A one-round budget must yield before a multi-second horizon; resuming
    // with a fresh call reaches the same fully committed state.
    fn run(budget: usize) -> (Vec<String>, Vec<KernelPersonView>, u64) {
        let mut kernel = seeded_kernel(3);
        kernel.start_world(at(0)).expect("start");
        let mut produced = Vec::new();
        let target = at(DAY);
        loop {
            let advance = kernel.advance_to(target, budget).expect("bounded advance");
            produced.extend(
                kernel
                    .drain_events()
                    .iter()
                    .map(|event| format!("{event:?}")),
            );
            if advance.reached_target() {
                break;
            }
        }
        let views: Vec<KernelPersonView> = kernel.persons().expect("running read");
        (produced, views, kernel.metrics().events_total)
    }
    let (segmented, views, seg_events) = run(1);
    let (whole, whole_views, whole_events) = run(usize::MAX);
    assert_eq!(
        segmented, whole,
        "segmented advance diverges from one long advance"
    );
    assert_eq!(views, whole_views, "final visible state diverges");
    assert_eq!(seg_events, whole_events, "event totals diverge");
}

#[test]
fn repeated_runs_are_deterministic() {
    fn run_once() -> (
        Vec<String>,
        Vec<KernelPersonView>,
        palimpsest_sim_core::KernelMetrics,
    ) {
        let mut kernel = seeded_kernel(4);
        kernel.start_world(at(0)).expect("start");
        spin_to(&mut kernel, at(TWO_DAYS));
        let events = kernel
            .drain_events()
            .iter()
            .map(|event| format!("{event:?}"))
            .collect();
        let views: Vec<KernelPersonView> = kernel.persons().expect("running read");
        (events, views, kernel.metrics())
    }
    let (first_events, first_views, first_metrics) = run_once();
    let (second_events, second_views, second_metrics) = run_once();
    assert_eq!(first_events, second_events, "event streams diverge");
    assert_eq!(first_views, second_views, "visible state diverges");
    assert_eq!(first_metrics, second_metrics, "metrics diverge");
    assert!(
        first_metrics.transitions_total > 0,
        "the fixture runs real work"
    );
}

#[test]
fn all_persons_are_addressable_by_stable_entity_id() {
    let mut kernel = seeded_kernel(100);
    kernel.start_world(at(0)).expect("start");
    spin_to(&mut kernel, at(DAY));
    assert_eq!(kernel.person_count(), 100);
    let ids: Vec<EntityId> = kernel
        .persons()
        .expect("running read")
        .into_iter()
        .map(|v| v.id())
        .collect();
    assert_eq!(ids.len(), 100);
    let unique: std::collections::HashSet<_> = ids.iter().copied().collect();
    assert_eq!(unique.len(), 100, "no identity is reused");
    for id in &ids {
        assert_eq!(
            kernel.person(*id).expect("running read").map(|v| v.id()),
            Some(*id)
        );
    }
    // Persons are iterated in stable ascending identity order.
    let mut sorted = ids.clone();
    sorted.sort();
    assert_eq!(ids, sorted, "render order is stable by EntityId");
}

#[test]
fn decisions_and_events_are_accounted_and_bounded() {
    let mut kernel = seeded_kernel(6);
    kernel.start_world(at(0)).expect("start");
    spin_to(&mut kernel, at(DAY));
    let metrics = kernel.metrics();
    assert!(metrics.decisions_total > 0, "decisions were resolved");
    assert!(
        metrics.events_total > 0,
        "high-level outcomes were accounted"
    );
    assert!(metrics.events_buffered <= palimpsest_sim_core::DEFAULT_EVENT_BUFFER_CAPACITY);
    let drained = kernel.drain_events();
    assert!(!drained.is_empty());
    for event in &drained {
        assert!(event.validate().is_ok(), "every accounted event is valid");
    }
    // The decision trace is retained per person (latest only).
    let ids: Vec<EntityId> = kernel
        .persons()
        .expect("running read")
        .into_iter()
        .map(|v| v.id())
        .collect();
    let count = ids
        .iter()
        .filter(|id| kernel.latest_trace(**id).expect("running read").is_some())
        .count();
    assert_eq!(
        count,
        ids.len(),
        "every surveyed person has a retained trace"
    );
}

#[test]
fn cadence_jumps_due_instants_and_never_accesses_every_second() {
    let mut kernel = seeded_kernel(2);
    kernel.start_world(at(0)).expect("start");
    // One simulated day is 86,400 seconds; the kernel must process far fewer
    // than that many advance rounds (it jumps between due instants, it does
    // not scan every second).
    let advance = spin_to(&mut kernel, at(DAY));
    let day = usize::try_from(DAY).expect("day fits usize");
    assert!(
        advance.rounds() < day / 10,
        "kernel ran {} rounds for {DAY} simulated seconds",
        advance.rounds()
    );
}

#[test]
fn empty_world_advances_to_the_target_directly() {
    let mut kernel = WorldKernel::from_world(WorldSeed::new(1), KernelConfig::default());
    assert_eq!(kernel.person_count(), 0);
    let advance = kernel
        .advance_to(at(50), 100)
        .expect("empty world advances");
    assert!(advance.reached_target());
    assert_eq!(advance.committed_to(), at(50));
    assert_eq!(advance.rounds(), 0);
    assert_eq!(advance.transitions(), 0);
    assert_eq!(kernel.now(), at(50));
}

#[test]
fn after_reaching_a_target_no_due_work_remains_at_or_before_it() {
    let mut kernel = seeded_kernel(2);
    kernel.start_world(at(0)).expect("start");
    let target = at(5_000);
    spin_to(&mut kernel, target);
    assert_eq!(kernel.now(), target);
    if let Some(due) = kernel.next_due().expect("complete-boundary queue") {
        assert!(
            due > target,
            "pending work must lie beyond the committed target"
        );
    }
    // Advancing again to the same target is a clean no-op.
    let again = kernel.advance_to(target, 1_000).expect("same target");
    assert!(again.reached_target());
    assert_eq!(again.rounds(), 0);
}
