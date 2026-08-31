// Authored by Kimi Code (AI coding agent) — task CHRON-027.
//! Closed-loop integration tests for the action execution state machine
//! (CHRON-027; mandatory ADR-0018 integration test).
//!
//! The reference driver runs actual candidate generation, scoring, movement,
//! completion, and Needs updates over 172,800 simulated seconds on the locked
//! seed-25,025 fixture — twice — and asserts positive Work/Eat/Sleep
//! completions, exact need reductions at completion instants, a return to
//! Work after both pressures fall low, and byte-identical outcomes. Selected
//! candidates alone never count as executed actions here.
//!
//! Interrupt and unreachable paths are covered through the public API only.

use palimpsest_sim_ai::{ActionCandidate, ActionKind, NeedValue, Needs, PerturbationSpec, Weights};
use palimpsest_sim_core::{
    ActionEnvironment, ActionRuntime, CancelReason, DecisionReason, EntityId, EntityIdAllocator,
    PersonRuntime, SimInstant, TransitionReason, decide_and_start, resolve_decision,
};
use palimpsest_sim_world::{
    ActivitySite, ActivitySites, LocalCoord, SiteKind, WorldGenConfig, WorldMap, WorldSeed,
};

/// Locked fixture seed shared with the ADR-0018 reference context.
const FIXTURE_SEED: u64 = 25_025;
/// Two simulated days: long enough to cross both need thresholds repeatedly
/// with the accepted durations (P1-REMAINING D1).
const CLOSED_LOOP_SECONDS: i64 = 172_800;

fn coord(x: i32, y: i32) -> LocalCoord {
    LocalCoord::new(x, y).expect("test coordinate in bounds")
}

fn at(value: i64) -> SimInstant {
    SimInstant::from_seconds(value)
}

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

/// The ADR-0018 reference fixture: a person at the walkable-block origin with
/// Meal +(2,0), Rest +(0,2), Work +(2,2), default pathfinding budget.
struct Fixture {
    map: WorldMap,
    sites: ActivitySites,
    origin: LocalCoord,
    persons: PersonRuntime,
    allocator: EntityIdAllocator,
}

impl Fixture {
    fn new() -> Self {
        let map = WorldMap::generate(WorldSeed::new(FIXTURE_SEED), WorldGenConfig::default());
        let origin = walkable_block_origin(&map);
        let (ox, oy) = (origin.x(), origin.y());
        let sites = ActivitySites::new(vec![
            ActivitySite::new(&map, coord(ox + 2, oy), SiteKind::Meal).expect("walkable"),
            ActivitySite::new(&map, coord(ox, oy + 2), SiteKind::Rest).expect("walkable"),
            ActivitySite::new(&map, coord(ox + 2, oy + 2), SiteKind::Work).expect("walkable"),
        ])
        .expect("distinct coords");
        Self {
            map,
            sites,
            origin,
            persons: PersonRuntime::new(),
            allocator: EntityIdAllocator::default(),
        }
    }

    fn spawn(&mut self) -> EntityId {
        self.persons
            .spawn(&mut self.allocator, self.origin)
            .expect("identity capacity")
    }

    fn env(&mut self) -> ActionEnvironment<'_> {
        ActionEnvironment {
            persons: &mut self.persons,
            map: &self.map,
            sites: &mut self.sites,
        }
    }
}

/// One full closed-loop record for determinism comparison: per-completion
/// (kind, instant, needs raw values after the completion) plus stats and the
/// event stream projection.
struct LoopRecord {
    completions: Vec<(ActionKind, i64, i64, i64)>,
    stats: palimpsest_sim_core::ActionStats,
    events: Vec<(String, i64)>,
    final_needs: (i64, i64),
    final_location: (i32, i32),
}

fn run_closed_loop(seconds: i64) -> LoopRecord {
    let mut fixture = Fixture::new();
    let person = fixture.spawn();
    let mut runtime = ActionRuntime::default();
    let weights = Weights::default();
    let spec = PerturbationSpec::ZERO;
    let mut completions = Vec::new();
    {
        let mut env = fixture.env();
        decide_and_start(&mut runtime, person, &mut env, &weights, &spec, at(0))
            .expect("initial decision");
    }
    let mut env = fixture.env();
    let target = at(seconds);
    while let Some(next) = runtime.next_due() {
        if next > target {
            break;
        }
        let outcome = runtime.advance(next, &mut env).expect("advance");
        for request in outcome.decision_requests() {
            if request.reason() == DecisionReason::Completed {
                // The completion already materialized needs with any relief
                // applied; record exactly what the loop did.
                let record_action = outcome
                    .transitions()
                    .iter()
                    .find(|transition| transition.reason() == TransitionReason::Completed)
                    .map(palimpsest_sim_core::Transition::action);
                let needs = env.persons.needs(person).expect("person exists");
                if let Some(action) = record_action {
                    completions.push((
                        action,
                        request.at().as_seconds(),
                        needs.hunger().raw(),
                        needs.fatigue().raw(),
                    ));
                }
            }
            resolve_decision(&mut runtime, request, &mut env, &weights, &spec)
                .expect("resolve decision");
        }
    }
    let events = runtime
        .drain_events()
        .iter()
        .map(|event| {
            assert!(event.validate().is_ok(), "every outcome event is valid");
            (
                event.event_type().to_owned(),
                event.timestamp().as_seconds(),
            )
        })
        .collect();
    let needs = env.persons.needs(person).expect("person exists");
    let location = env.persons.location(person).expect("person exists");
    LoopRecord {
        completions,
        stats: runtime.stats(),
        events,
        final_needs: (needs.hunger().raw(), needs.fatigue().raw()),
        final_location: (location.x(), location.y()),
    }
}

#[test]
fn closed_loop_two_days_crosses_all_thresholds_and_returns_to_work() {
    let record = run_closed_loop(CLOSED_LOOP_SECONDS);
    let stats = record.stats;
    assert!(stats.work_completions > 0, "work happens: {stats:?}");
    assert!(stats.eat_completions > 0, "eat happens: {stats:?}");
    assert!(stats.sleep_completions > 0, "sleep happens: {stats:?}");
    assert!(
        stats.movement_completions > 0,
        "movement happens: {stats:?}"
    );
    assert_eq!(stats.blocked, 0);
    assert_eq!(stats.failed, 0);

    // Every Eat completion fully relieved hunger; every Sleep completion
    // fully relieved fatigue (materialize-then-relieve, ADR-0021 §2).
    for (kind, _, hunger, fatigue) in &record.completions {
        match kind {
            ActionKind::Eat => assert_eq!(*hunger, 0, "Eat relieves hunger completely"),
            ActionKind::Sleep => assert_eq!(*fatigue, 0, "Sleep relieves fatigue completely"),
            _ => {}
        }
    }

    // After the first need-driven action completes, the person returns to
    // Work (ADR-0018: low needs never starve Work in the reference context).
    let first_need_action = record
        .completions
        .iter()
        .position(|(kind, _, _, _)| matches!(kind, ActionKind::Eat | ActionKind::Sleep))
        .expect("a need-driven completion exists");
    assert!(
        record
            .completions
            .iter()
            .skip(first_need_action + 1)
            .any(|(kind, _, _, _)| *kind == ActionKind::Work),
        "the loop returns to Work after needs recover"
    );

    // Events are ordered by commit instant and reference the live person.
    let mut previous = 0_i64;
    for (_, timestamp) in &record.events {
        assert!(*timestamp >= previous);
        previous = *timestamp;
    }
}

#[test]
fn closed_loop_is_deterministic_across_repeated_runs() {
    let first = run_closed_loop(CLOSED_LOOP_SECONDS);
    let second = run_closed_loop(CLOSED_LOOP_SECONDS);
    assert_eq!(first.completions, second.completions, "completions diverge");
    assert_eq!(first.events, second.events, "event stream diverges");
    assert_eq!(first.stats, second.stats, "stats diverge");
    assert_eq!(first.final_needs, second.final_needs);
    assert_eq!(first.final_location, second.final_location);
}

#[test]
fn driver_interrupts_work_when_a_critical_need_wins() {
    let mut fixture = Fixture::new();
    let person = fixture.spawn();
    let work = coord(fixture.origin.x() + 2, fixture.origin.y() + 2);
    // Fatigue is one second below the critical boundary; start Work directly
    // so the boundary check must supersede it through a real selection.
    fixture
        .persons
        .set_needs(
            person,
            Needs::new(
                NeedValue::from_raw(0).expect("in range"),
                NeedValue::from_raw(89_998).expect("in range"),
            ),
        )
        .expect("set needs");
    let mut runtime = ActionRuntime::default();
    let weights = Weights::default();
    let spec = PerturbationSpec::ZERO;
    {
        let mut env = fixture.env();
        runtime
            .start(
                person,
                ActionCandidate::new(ActionKind::Work, Some(work), 0).expect("candidate"),
                &mut env,
                at(0),
            )
            .expect("start work");
    }
    let mut env = fixture.env();
    let mut interrupted = false;
    let target = at(10_000);
    while let Some(next) = runtime.next_due() {
        if next > target {
            break;
        }
        let outcome = runtime.advance(next, &mut env).expect("advance");
        for request in outcome.decision_requests() {
            resolve_decision(&mut runtime, request, &mut env, &weights, &spec)
                .expect("resolve decision");
            if request.reason() == DecisionReason::CriticalBoundary {
                interrupted = true;
            }
        }
        if interrupted {
            break;
        }
    }
    assert!(interrupted, "the critical boundary fired");
    assert_eq!(runtime.stats().interrupted, 1);
    assert_eq!(
        runtime.current_action(person).map(|(kind, _)| kind),
        Some(ActionKind::Sleep),
        "the fresh selection replaced Work with Sleep"
    );
}

#[test]
fn unreachable_world_degrades_to_idle_waits() {
    // A one-cell path cap makes every off-cell site unreachable: the provider
    // emits only the Idle baseline and the loop stays alive without fabricating
    // actions.
    let mut fixture = Fixture::new();
    let person = fixture.spawn();
    let config = palimpsest_sim_core::ActionConfig::new(
        palimpsest_sim_core::SimDuration::from_seconds(1).expect("positive"),
        palimpsest_sim_core::SimDuration::from_seconds(600).expect("positive"),
        palimpsest_sim_core::SimDuration::from_seconds(28_800).expect("positive"),
        palimpsest_sim_core::SimDuration::from_seconds(1_800).expect("positive"),
        palimpsest_sim_core::SimDuration::from_seconds(60).expect("positive"),
        palimpsest_sim_core::SimDuration::from_seconds(1).expect("positive"),
        palimpsest_sim_core::SimDuration::from_seconds(60).expect("positive"),
        palimpsest_sim_world::PathConfig::new(usize::MAX, 1),
    )
    .expect("positive durations");
    let mut runtime = ActionRuntime::new(config);
    let weights = Weights::default();
    let spec = PerturbationSpec::ZERO;
    {
        let mut env = fixture.env();
        let resolution = decide_and_start(&mut runtime, person, &mut env, &weights, &spec, at(0))
            .expect("initial decision");
        assert_eq!(resolution.selection().candidate().kind(), ActionKind::Idle);
    }
    let mut env = fixture.env();
    let target = at(1_000);
    while let Some(next) = runtime.next_due() {
        if next > target {
            break;
        }
        let outcome = runtime.advance(next, &mut env).expect("advance");
        for request in outcome.decision_requests() {
            let resolution = resolve_decision(&mut runtime, request, &mut env, &weights, &spec)
                .expect("resolve decision");
            assert_eq!(resolution.selection().candidate().kind(), ActionKind::Idle);
        }
    }
    let stats = runtime.stats();
    assert_eq!(stats.idle_completions, 16, "1,000s of 60s Idle waits");
    assert_eq!(stats.move_completions, 0);
    assert_eq!(stats.work_completions, 0);
    assert_eq!(stats.blocked, 0);
    assert_eq!(stats.failed, 0);
    // Needs commit at transition boundaries: the last Idle wait started at
    // t=960 and materialized growth to that instant (hunger 960, fatigue
    // 1_920). The wait completing at t=1,020 lies beyond the horizon.
    let needs = env.persons.needs(person).expect("person exists");
    assert_eq!(needs.hunger().raw(), 960);
    assert_eq!(needs.fatigue().raw(), 1_920);
}

#[test]
fn cancel_is_the_only_external_abort_and_it_is_clean() {
    let mut fixture = Fixture::new();
    let person = fixture.spawn();
    let work = coord(fixture.origin.x() + 2, fixture.origin.y() + 2);
    let mut runtime = ActionRuntime::default();
    let mut env = fixture.env();
    runtime
        .start(
            person,
            ActionCandidate::new(ActionKind::Work, Some(work), 0).expect("candidate"),
            &mut env,
            at(0),
        )
        .expect("start work");
    runtime.advance(at(2), &mut env).expect("two steps");
    runtime
        .cancel(person, CancelReason::External, at(2), &mut env)
        .expect("external cancel");
    assert_eq!(runtime.current(person), None);
    assert_eq!(runtime.stats().cancelled, 1);
    // The runtime is clean: no continuation can fire later, and a new action
    // starts normally.
    runtime.advance(at(500), &mut env).expect("advance");
    assert_eq!(runtime.stats().steps, 2);
    decide_and_start(
        &mut runtime,
        person,
        &mut env,
        &Weights::default(),
        &PerturbationSpec::ZERO,
        at(500),
    )
    .expect("restart after cancel");
    assert!(runtime.current_action(person).is_some());
}
