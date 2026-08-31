// Authored by opencode (AI coding agent) — KFIX-001..006 shared fixture.
//! Shared, independent repair fixture (P1-KERNEL-REPAIR plan §2).
//!
//! This is a separate fixture from the ADR-0018 closed-loop fixture and the
//! generator golden: it must not be edited to dodge a collision. It reproduces
//! the reviewed findings F01–F06 deterministically. Some helpers are consumed
//! by a specific KFIX suite only.
#![allow(dead_code)]

use palimpsest_sim_ai::{ActionCandidate, ActionKind, PerturbationSpec, Weights};
use palimpsest_sim_core::{
    ActionConfig, ActionEnvironment, ActionRuntime, EntityId, EntityIdAllocator, PersonRuntime,
    SimDuration, SimInstant,
};
use palimpsest_sim_world::{
    ActivitySite, ActivitySites, LocalCoord, PathConfig, SiteKind, WorldGenConfig, WorldMap,
    WorldSeed,
};

/// Locked fixture seed (same reference world as ADR-0018).
pub const FIXTURE_SEED: u64 = 25_025;

/// The repaired fixture world.
pub struct RepairFixture {
    pub map: WorldMap,
    pub sites: ActivitySites,
    pub origin: LocalCoord,
    pub work: LocalCoord,
    pub meal: LocalCoord,
    pub rest: LocalCoord,
    pub persons: PersonRuntime,
    pub allocator: EntityIdAllocator,
}

impl RepairFixture {
    /// Builds the fixture: Work at the 3×3 origin, Meal to the east, Rest to
    /// the south, the person spawn point at the origin.
    pub fn new() -> Self {
        let map = WorldMap::generate(WorldSeed::new(FIXTURE_SEED), WorldGenConfig::default());
        let origin = walkable_block_origin(&map);
        let (ox, oy) = (origin.x(), origin.y());
        let work = origin;
        let meal = coord(ox + 2, oy);
        let rest = coord(ox, oy + 2);
        let sites = ActivitySites::new(vec![
            ActivitySite::new(&map, work, SiteKind::Work).expect("walkable"),
            ActivitySite::new(&map, meal, SiteKind::Meal).expect("walkable"),
            ActivitySite::new(&map, rest, SiteKind::Rest).expect("walkable"),
        ])
        .expect("distinct coords");
        Self {
            map,
            sites,
            origin,
            work,
            meal,
            rest,
            persons: PersonRuntime::new(),
            allocator: EntityIdAllocator::default(),
        }
    }

    /// Spawns a person at the origin with default (zero) needs.
    pub fn spawn(&mut self) -> EntityId {
        self.persons
            .spawn(&mut self.allocator, self.origin)
            .expect("identity capacity")
    }

    /// A fresh action environment borrowing this fixture.
    pub fn env(&mut self) -> ActionEnvironment<'_> {
        ActionEnvironment {
            persons: &mut self.persons,
            map: &self.map,
            sites: &mut self.sites,
        }
    }
}

impl Default for RepairFixture {
    fn default() -> Self {
        Self::new()
    }
}

/// The zero-perturbation perturbation spec.
pub fn zero_perturbation() -> PerturbationSpec {
    PerturbationSpec::ZERO
}

/// The default Phase 1 weights table.
pub fn default_weights() -> Weights {
    Weights::default()
}

/// The default pathfinding budget.
pub fn default_path() -> PathConfig {
    PathConfig::default()
}

/// An `ActionConfig` with the given Work duration (all other timings default).
pub fn action_config_with_work(work_seconds: i64) -> ActionConfig {
    ActionConfig::new(
        seconds(1),
        seconds(600),
        seconds(28_800),
        SimDuration::from_seconds(work_seconds).expect("non-negative"),
        seconds(60),
        seconds(1),
        seconds(60),
        PathConfig::default(),
    )
    .expect("positive durations")
}

/// A default action configuration.
pub fn default_action_config() -> ActionConfig {
    ActionConfig::default()
}

/// A validated test candidate.
pub fn candidate(kind: ActionKind, target: Option<LocalCoord>) -> ActionCandidate {
    ActionCandidate::new(kind, target, 0).expect("valid test candidate")
}

/// An action runtime with the given configuration.
pub fn runtime(config: ActionConfig) -> ActionRuntime {
    ActionRuntime::new(config)
}

pub fn seconds(value: i64) -> SimDuration {
    SimDuration::from_seconds(value).expect("non-negative duration")
}

pub fn at(value: i64) -> SimInstant {
    SimInstant::from_seconds(value)
}

pub fn coord(x: i32, y: i32) -> LocalCoord {
    LocalCoord::new(x, y).expect("test coordinate in bounds")
}

/// Origin of the first fully-walkable (8-connected check) 3×3 block, scanned
/// in row-major order over the map's coordinates.
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

/// Returns whether the map cell at `coord` is walkable.
pub fn is_walkable(map: &WorldMap, coord: LocalCoord) -> bool {
    map.local()
        .get(coord.x(), coord.y())
        .is_some_and(|kind| kind.is_walkable())
}
