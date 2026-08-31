// Authored by Kimi Code (AI coding agent) — task CHRON-027.
//! Action-execution throughput benchmark (CHRON-027): the closed
//! decide → execute → advance loop at 100 and 1,000 persons over a fixed
//! simulated interval, release build, with two warm-ups and ten samples by
//! default. Fixture construction (world generation, spawn) is excluded from
//! the timed interval; correctness assertions stay enabled in every sample.
//!
//! CLI: `--persons N --seconds S --samples K [--warmups W]`.
//! The `memory_workload` adapter serves the REM-008A peak-RSS tool; it is
//! separate from the timing path.
#![allow(clippy::cast_precision_loss)]

use std::collections::VecDeque;
use std::hint::black_box;
use std::time::Instant;

use palimpsest_sim_ai::{PerturbationSpec, Weights};
use palimpsest_sim_core::{
    ActionEnvironment, ActionRuntime, ActionStats, EntityId, EntityIdAllocator, PersonRuntime,
    SimInstant, decide_and_start, resolve_decisions,
};
use palimpsest_sim_world::{
    ActivitySite, ActivitySites, LocalCoord, SiteKind, WorldGenConfig, WorldMap, WorldSeed,
};
use serde::Serialize;
#[path = "support/bench_protocol.rs"]
mod protocol;

/// Locked fixture seed shared with the ADR-0018 reference context.
const FIXTURE_SEED: u64 = 25_025;

struct Fixture {
    map: WorldMap,
    sites: ActivitySites,
    persons: PersonRuntime,
    roster: Vec<EntityId>,
}

fn coord(x: i32, y: i32) -> LocalCoord {
    LocalCoord::new(x, y).expect("coordinate in bounds")
}

/// Origin of the fully walkable 3×3 block guaranteed by the generator's
/// spawn clearing (same fixture as the test suites).
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

/// Deterministic BFS over the walkable region containing `origin`, in fixed
/// neighbor order, so every spawned person can reach the fixture sites.
fn walkable_region(map: &WorldMap, origin: LocalCoord) -> Vec<LocalCoord> {
    let mut visited = vec![false; palimpsest_sim_world::LOCAL_GRID_CELL_COUNT];
    let mut order = Vec::new();
    let mut queue = VecDeque::new();
    visited[origin.index()] = true;
    queue.push_back(origin);
    while let Some(cell) = queue.pop_front() {
        order.push(cell);
        for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
            let Some(next) = LocalCoord::new(cell.x() + dx, cell.y() + dy) else {
                continue;
            };
            if visited[next.index()] {
                continue;
            }
            let walkable = map
                .local()
                .get(next.x(), next.y())
                .is_some_and(|kind| kind.is_walkable());
            if walkable {
                visited[next.index()] = true;
                queue.push_back(next);
            }
        }
    }
    order
}

/// Builds the fixture outside the timed interval: the seeded map, the three
/// ADR-0018 reference sites, and `count` persons strided deterministically
/// across the sites' walkable region.
fn build_fixture(count: usize) -> Fixture {
    let map = WorldMap::generate(WorldSeed::new(FIXTURE_SEED), WorldGenConfig::default());
    let origin = walkable_block_origin(&map);
    let (ox, oy) = (origin.x(), origin.y());
    let sites = ActivitySites::new(vec![
        ActivitySite::new(&map, coord(ox + 2, oy), SiteKind::Meal).expect("walkable"),
        ActivitySite::new(&map, coord(ox, oy + 2), SiteKind::Rest).expect("walkable"),
        ActivitySite::new(&map, coord(ox + 2, oy + 2), SiteKind::Work).expect("walkable"),
    ])
    .expect("distinct coords");
    let region = walkable_region(&map, origin);
    assert!(
        region.len() >= count,
        "walkable region {} cannot host {count} persons",
        region.len()
    );
    let mut persons = PersonRuntime::new();
    let mut allocator = EntityIdAllocator::default();
    let mut roster = Vec::with_capacity(count);
    for index in 0..count {
        let location = region[index * region.len() / count];
        roster.push(
            persons
                .spawn(&mut allocator, location)
                .expect("identity capacity"),
        );
    }
    Fixture {
        map,
        sites,
        persons,
        roster,
    }
}

/// One timed closed-loop run: initial decisions at the epoch, then the
/// reference driver to `seconds`. Returns (transitions committed, stats,
/// checksum over final truth).
fn run_closed_loop(
    fixture: &mut Fixture,
    seconds: i64,
) -> (
    u64,
    ActionStats,
    u64,
    palimpsest_sim_core::ActionRuntimeMetrics,
) {
    let mut runtime = ActionRuntime::default();
    let weights = Weights::default();
    let spec = PerturbationSpec::ZERO;
    let mut transitions = 0_u64;
    {
        let mut env = ActionEnvironment {
            persons: &mut fixture.persons,
            map: &fixture.map,
            sites: &mut fixture.sites,
        };
        for &person in &fixture.roster {
            let resolution = decide_and_start(
                &mut runtime,
                person,
                &mut env,
                &weights,
                &spec,
                SimInstant::EPOCH,
            )
            .expect("initial decision");
            transitions += resolution.transitions().len() as u64;
        }
    }
    let mut env = ActionEnvironment {
        persons: &mut fixture.persons,
        map: &fixture.map,
        sites: &mut fixture.sites,
    };
    let target = SimInstant::from_seconds(seconds);
    while let Some(next) = runtime.next_due() {
        if next > target {
            break;
        }
        let outcome = runtime.advance(next, &mut env).expect("advance");
        transitions += outcome.transitions().len() as u64;
        let resolutions = resolve_decisions(
            &mut runtime,
            outcome.decision_requests(),
            &mut env,
            &weights,
            &spec,
        )
        .expect("resolve decisions");
        for person_resolution in &resolutions {
            transitions += person_resolution.resolution().transitions().len() as u64;
        }
    }
    let stats = runtime.stats();
    // Correctness assertions stay enabled: the loop really moved, ate, slept,
    // and worked, and never blocked or failed on this fixture.
    assert!(stats.movement_completions > 0);
    assert!(stats.work_completions > 0);
    assert!(stats.eat_completions > 0);
    assert!(stats.sleep_completions > 0);
    assert_eq!(stats.blocked, 0);
    assert_eq!(stats.failed, 0);
    let checksum = truth_checksum(env.persons, &fixture.roster, stats);
    (transitions, stats, checksum, runtime.metrics())
}

fn truth_checksum(persons: &PersonRuntime, roster: &[EntityId], stats: ActionStats) -> u64 {
    let mut checksum = 0_u64;
    for &id in roster {
        let view = persons.get(id).expect("spawned person exists");
        let needs = persons.needs(id).expect("spawned person has needs");
        checksum = checksum
            .wrapping_mul(1_000_003)
            .wrapping_add(u64::try_from(view.location().x()).expect("coordinates are non-negative"))
            .wrapping_mul(1_000_003)
            .wrapping_add(u64::try_from(view.location().y()).expect("coordinates are non-negative"))
            .wrapping_mul(1_000_003)
            .wrapping_add(u64::try_from(needs.hunger().raw()).expect("needs are non-negative"))
            .wrapping_mul(1_000_003)
            .wrapping_add(u64::try_from(needs.fatigue().raw()).expect("needs are non-negative"));
    }
    checksum = checksum
        .wrapping_mul(1_000_003)
        .wrapping_add(stats.move_completions)
        .wrapping_mul(1_000_003)
        .wrapping_add(stats.eat_completions)
        .wrapping_mul(1_000_003)
        .wrapping_add(stats.sleep_completions)
        .wrapping_mul(1_000_003)
        .wrapping_add(stats.work_completions)
        .wrapping_mul(1_000_003)
        .wrapping_add(stats.idle_completions);
    checksum
}

#[derive(Serialize, PartialEq, Debug)]
struct StatsOut {
    move_completions: u64,
    eat_completions: u64,
    sleep_completions: u64,
    work_completions: u64,
    idle_completions: u64,
    blocked: u64,
    failed: u64,
}
#[derive(Serialize)]
struct Sample {
    index: usize,
    wall_ns: u128,
    wall_seconds: f64,
    transitions: u64,
    stats: StatsOut,
    checksum: u64,
    events_total: u64,
    events_digest: u64,
    queue_depth: usize,
    stale_nodes: usize,
    transitions_per_wall_second: f64,
}

#[derive(Serialize)]
struct Report {
    fixture: &'static str,
    seed: u64,
    spawn_layout: &'static str,
    units: &'static str,
    config: serde_json::Value,
    persons: usize,
    seconds: usize,
    samples: usize,
    warmups: usize,
    min_ns: u128,
    median_ns: u128,
    max_ns: u128,
    transitions: u64,
    transitions_per_sim_second: f64,
    sim_seconds_per_wall_second: f64,
    checksum: u64,
    samples_series: Vec<Sample>,
}

fn verify_series(series: &[Sample]) {
    let first = &series[0];
    for s in &series[1..] {
        assert_eq!(
            (s.transitions, s.checksum, s.stale_nodes),
            (first.transitions, first.checksum, first.stale_nodes),
            "nondeterministic truth"
        );
        assert_eq!(
            (&s.stats, s.events_total, s.events_digest, s.queue_depth),
            (
                &first.stats,
                first.events_total,
                first.events_digest,
                first.queue_depth
            )
        );
    }
}

fn main() {
    let mut defaults = protocol::defaults();
    defaults.seconds = 172_800;
    let args = match protocol::parse_for(
        std::env::args().skip(1),
        defaults,
        &["--persons", "--seconds", "--samples", "--warmups", "--json"],
    ) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("action_execution_bench: {e}");
            std::process::exit(2);
        }
    };
    let (persons, seconds, samples, warmups, json) = (
        args.persons,
        usize::try_from(args.seconds).expect("positive seconds fit usize"),
        args.samples,
        args.warmups,
        args.json,
    );
    if persons == 0 {
        eprintln!("action_execution_bench: persons must be positive");
        std::process::exit(2);
    }

    for index in 0..warmups {
        eprintln!("action warmup {}/{} persons={persons}", index + 1, warmups);
        let mut fixture = build_fixture(persons);
        black_box(run_closed_loop(
            &mut fixture,
            i64::try_from(seconds).expect("seconds fit i64"),
        ));
    }

    let mut series = Vec::with_capacity(samples);
    for index in 0..samples {
        eprintln!("action sample {}/{} persons={persons}", index + 1, samples);
        let mut fixture = build_fixture(persons);
        let started = Instant::now();
        let result = run_closed_loop(
            &mut fixture,
            i64::try_from(seconds).expect("seconds fit i64"),
        );
        let elapsed = started.elapsed();
        let s = result.1;
        series.push(Sample {
            index,
            wall_ns: elapsed.as_nanos(),
            wall_seconds: elapsed.as_secs_f64(),
            transitions: result.0,
            stats: StatsOut {
                move_completions: s.move_completions,
                eat_completions: s.eat_completions,
                sleep_completions: s.sleep_completions,
                work_completions: s.work_completions,
                idle_completions: s.idle_completions,
                blocked: s.blocked,
                failed: s.failed,
            },
            checksum: result.2,
            events_total: result.3.events_total,
            events_digest: result.3.events_digest,
            queue_depth: result.3.scheduler.scheduled_entries,
            stale_nodes: result.3.scheduler.stale_nodes,
            transitions_per_wall_second: result.0 as f64 / elapsed.as_secs_f64(),
        });
    }
    let mut walls: Vec<u128> = series.iter().map(|s| s.wall_ns).collect();
    let median_ns = protocol::median(&mut walls);
    let first = &series[0];
    verify_series(&series);
    let report = Report {
        fixture: "action-reference-sites",
        seed: FIXTURE_SEED,
        spawn_layout: "strided_bfs_connected_region",
        units: "wall_ns=nanoseconds; wall_seconds=seconds; rates=per_wall_second",
        config: protocol::configuration(),
        persons,
        seconds,
        samples,
        warmups,
        min_ns: *walls.first().unwrap(),
        median_ns,
        max_ns: *walls.last().unwrap(),
        transitions: first.transitions,
        transitions_per_sim_second: first.transitions as f64 / seconds as f64,
        sim_seconds_per_wall_second: seconds as f64 / (median_ns as f64 / 1e9),
        checksum: first.checksum,
        samples_series: series,
    };
    let text = serde_json::to_string(&report).expect("serialize report");
    if json {
        println!("{text}");
    } else {
        eprintln!("{text}");
    }
}

/// Retains one closed-loop run for the memory benchmark adapter. The callback
/// marks the boundary around the measured run; the returned checksum is a
/// golden value asserted by the adapter tests.
///
/// # Panics
///
/// Panics when `case` is not `"100"` or `"1000"`.
pub fn memory_workload(case: &str, observe: &mut dyn FnMut()) -> u64 {
    let count = match case {
        "100" => 100,
        "1000" => 1_000,
        other => panic!("invalid action memory workload selector: {other}"),
    };
    let mut fixture = build_fixture(count);
    observe();
    let (_, _, checksum, _) = run_closed_loop(&mut fixture, 86_400);
    observe();
    let expected = match count {
        100 => 4_716_271_126_859_177_484,
        1_000 => 9_948_480_634_061_406_840,
        _ => unreachable!(),
    };
    assert_eq!(checksum, expected, "memory workload golden checksum");
    black_box(&fixture);
    checksum
}

#[cfg(test)]
mod tests {
    use super::memory_workload;

    #[test]
    fn memory_adapter_observes_twice_and_matches_golden() {
        let mut callbacks = 0;
        let checksum = memory_workload("100", &mut || callbacks += 1);
        assert_eq!(callbacks, 2);
        assert_eq!(checksum, 4_716_271_126_859_177_484);
    }

    #[test]
    fn memory_adapter_matches_1000_golden() {
        let mut callbacks = 0;
        let checksum = memory_workload("1000", &mut || callbacks += 1);
        assert_eq!(callbacks, 2);
        assert_eq!(checksum, 9_948_480_634_061_406_840);
    }

    #[test]
    #[should_panic(expected = "invalid action memory workload selector")]
    fn memory_adapter_rejects_invalid_selector() {
        let _ = memory_workload("bad", &mut || {});
    }
}
