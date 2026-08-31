// Authored by opencode (AI coding agent) — task CHRON-032.
//! Headless 10-year chaos runner (CHRON-032, ADR-0027).
//!
//! Drives the deterministic [`WorldKernel`] continuously over the Phase 1
//! horizon (10 simulated years = `315_360_000` s from the epoch) for a
//! fixed-seed 100-person world, proving the Master Spec's "continuous 10 years
//! without crash" gate. It is an **outer driver only**: it never mutates world
//! state directly, never teleports a person, and relies on the kernel for every
//! committed boundary. Its invariant/detector instrument lives in the Core (not
//! Godot) and returns a typed [`ChaosError`] (non-zero exit) on any violation.
//!
//! Determinism: the same `seed`/config/input produces the same canonical truth
//! hash, per-sample bounds, per-person completion counts, and event sequence.
//! Wall-clock time and RSS are measurement fields within [`ChaosReport`] and are
//! excluded from the truth hash and from cross-run equality.
//!
//! ```no_run
//! use palimpsest_sim_core::run_chaos;
//! // A 10-year run is the Phase 1 gate; a doctest only compiles this example.
//! let report = run_chaos(&Default::default(), false).expect("ten-year run succeeds");
//! assert!(report.person_count == 100);
//! ```
// The casts below are deliberate: coordinates, seconds, and hash folding use
// values that are non-negative and small (128×128 grid, i64 seconds), so the
// sign-loss and wrap lints do not apply.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap
)]

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::error::Error;
use std::fmt;

use palimpsest_sim_ai::{ActionKind, NEED_MAX, Needs};
use palimpsest_sim_entity::EntityId;
use palimpsest_sim_world::{
    ActivitySite, ActivitySites, LocalCoord, SiteKind, WorldGenConfig, WorldMap, WorldSeed,
};
use serde::{Deserialize, Serialize};

use crate::{ActionState, KernelConfig, KernelError, KernelMetrics, KernelPersonView, WorldKernel};

/// Simulated seconds per day (D4).
pub const SECONDS_PER_DAY: i64 = 86_400;
/// Days per calendar year (D4; not new calendar lore).
pub const DAYS_PER_YEAR: i64 = 365;
/// Simulated seconds per year.
pub const SECONDS_PER_YEAR: i64 = SECONDS_PER_DAY * DAYS_PER_YEAR;
/// The authoritative Phase 1 horizon: 10 simulated years.
pub const TEN_YEARS_SECONDS: i64 = SECONDS_PER_YEAR * 10;
/// Stalled `advance_to` calls before the liveness guard reports non-termination.
pub const MAX_STALLED_ADVANCE_CALLS: usize = 1_024;
/// Hard cap on `advance_to` calls within one simulated-day window.
pub const MAX_ADVANCE_CALLS_PER_DAY: usize = 1_048_576;
/// How many distinct id/path bytes an injected reference may use in tests.
const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;

/// The configuration for one chaos run (D4 defaults).
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ChaosConfig {
    /// The world-generation seed (deterministic fixture).
    pub seed: u64,
    /// The number of persons to spawn (Phase 1 default 100).
    pub person_count: usize,
    /// The number of simulated years to advance.
    pub years: u64,
    /// The simulated seconds per year (defaults to the D4 365-day year).
    pub sim_seconds_per_year: i64,
}

/// Observable lifecycle boundaries for supervised chaos runs.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChaosCheckpoint {
    Prepared,
    Advance,
    Day,
    Complete,
}

/// Builds and starts the deterministic chaos fixture without advancing it.
///
/// # Errors
/// Rejects invalid config, unreachable fixtures, or kernel initialization errors.
pub fn build_chaos_kernel(config: &ChaosConfig) -> Result<WorldKernel, ChaosError> {
    if config.person_count == 0 {
        return Err(ChaosError::Config("person_count must be positive".into()));
    }
    if config.years == 0 {
        return Err(ChaosError::Config("years must be positive".into()));
    }
    if config.sim_seconds_per_year <= 0 {
        return Err(ChaosError::Config(
            "sim_seconds_per_year must be positive".into(),
        ));
    }
    let target = config.target_seconds()?;
    if target <= 0 {
        return Err(ChaosError::Config("horizon must be positive".into()));
    }
    let map = WorldMap::generate(WorldSeed::new(config.seed), WorldGenConfig::default());
    let sites = ActivitySites::place_defaults(&map);
    let spawns = resolve_spawns(&map, &sites, config.person_count)?;
    let mut kernel = WorldKernel::new(map, sites, KernelConfig::default());
    for coord in spawns {
        kernel.spawn_person(coord)?;
    }
    let started = kernel.start_world(crate::SimInstant::EPOCH)?;
    if started != config.person_count {
        return Err(ChaosError::Population {
            expected: config.person_count,
            actual: started,
        });
    }
    Ok(kernel)
}

impl Default for ChaosConfig {
    fn default() -> Self {
        Self {
            seed: 42,
            person_count: 100,
            years: 10,
            sim_seconds_per_year: SECONDS_PER_YEAR,
        }
    }
}

impl ChaosConfig {
    /// The target simulated instant, in seconds from the epoch.
    ///
    /// # Errors
    /// Returns a [`ChaosError::Config`] when the horizon overflows `i64`.
    pub fn target_seconds(&self) -> Result<i64, ChaosError> {
        let years = i64::try_from(self.years)
            .map_err(|_| ChaosError::Config("years does not fit an i64 second budget".into()))?;
        years
            .checked_mul(self.sim_seconds_per_year)
            .ok_or_else(|| ChaosError::Config("horizon overflows i64 seconds".into()))
    }
}

/// A typed failure that aborts a chaos run (non-zero exit from the bin).
#[derive(Debug)]
pub enum ChaosError {
    /// A kernel error (a recoverable rejection or a real execution fault).
    Kernel(KernelError),
    /// An integer quantity left a documented bound (the integer stand-in for a
    /// NaN/Inf because every Phase 1 quantity is an integer).
    NonFinite {
        /// The offending person.
        person: EntityId,
        /// The quantity label.
        field: &'static str,
        /// The observed value.
        value: i64,
        /// The inclusive lower bound.
        min: i64,
        /// The inclusive upper bound.
        max: i64,
    },
    /// The scheduler queue exceeded a documented correctness bound.
    QueueGrowth {
        /// The diagnostic key (`scheduled_entries` / `queue_nodes`).
        key: &'static str,
        /// The observed size.
        value: usize,
        /// The documented bound.
        limit: usize,
    },
    /// A drained event referenced an identity that does not resolve.
    DanglingReference {
        /// Human-readable detail.
        detail: String,
    },
    /// Any other documented invariant was violated.
    Invariant {
        /// The rule label.
        rule: &'static str,
        /// Human-readable detail.
        detail: String,
    },
    /// The committed instant stalled (no progress) across many calls — an
    /// infinite-loop / non-termination report, not a fake recovery.
    NonTerminating {
        /// The stalled committed instant, in seconds.
        committed_to_seconds: i64,
        /// How many consecutive calls made no progress.
        stalled_calls: usize,
    },
    /// The live population count changed (Phase 1 never removes a person).
    Population {
        /// Expected count.
        expected: usize,
        /// Observed count.
        actual: usize,
    },
    /// The configuration is invalid (e.g. zero person count or horizon).
    Config(String),
    /// No walkable component contains a Meal, a Rest, **and** a Work site.
    NoReachableFixture {
        /// Why the fixture could not be resolved.
        reason: String,
    },
    /// A per-person hard condition was not reached by the end.
    UnmetCompletion {
        /// Human-readable detail (which person / which required completion).
        detail: String,
    },
}

impl fmt::Display for ChaosError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Kernel(error) => write!(f, "kernel error: {error}"),
            Self::NonFinite {
                person,
                field,
                value,
                min,
                max,
            } => write!(
                f,
                "non-finite/out-of-bounds {field} for person {}: {value} not in [{min}, {max}]",
                person.get()
            ),
            Self::QueueGrowth { key, value, limit } => {
                write!(f, "unbounded queue growth: {key} = {value} > {limit}")
            }
            Self::DanglingReference { detail } => write!(f, "dangling reference: {detail}"),
            Self::Invariant { rule, detail } => write!(f, "invariant '{rule}' violated: {detail}"),
            Self::NonTerminating {
                committed_to_seconds,
                stalled_calls,
            } => write!(
                f,
                "non-terminating: committed to {committed_to_seconds}s after {stalled_calls} stalled calls"
            ),
            Self::Population { expected, actual } => {
                write!(
                    f,
                    "population changed: expected {expected}, actual {actual}"
                )
            }
            Self::Config(error) => write!(f, "invalid chaos config: {error}"),
            Self::NoReachableFixture { reason } => {
                write!(f, "no reachable Meal/Rest/Work fixture: {reason}")
            }
            Self::UnmetCompletion { detail } => write!(f, "unmet completion: {detail}"),
        }
    }
}

impl Error for ChaosError {}

impl From<KernelError> for ChaosError {
    fn from(value: KernelError) -> Self {
        Self::Kernel(value)
    }
}

/// Per-person committed action completions, derived from real committed
/// outcome events (never from a selection).
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize)]
pub struct PerPersonCompletions {
    /// Completed standalone `Move` actions.
    pub moves: u64,
    /// Completed `Eat` actions.
    pub eats: u64,
    /// Completed `Sleep` actions.
    pub sleeps: u64,
    /// Completed `Work` actions.
    pub works: u64,
    /// Completed nonzero-distance movement phases observed at committed arrival.
    pub movement_phases: u64,
}

impl PerPersonCompletions {
    /// The number of the four required Phase 1 observations for a person:
    /// Eat, Sleep, Work, and a real (non-teleport) movement phase. A standalone
    /// top-level `Move` is not required: persons reach an activity site through
    /// each activity's own movement phase, which is what this counts.
    #[must_use]
    pub const fn kinds_completed(&self) -> u32 {
        (self.eats > 0) as u32
            + (self.sleeps > 0) as u32
            + (self.works > 0) as u32
            + (self.movement_phases > 0) as u32
    }
}

/// One per-person completion row (sorted by stable id) for the report.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub struct PerPersonCompletionRow {
    /// The person's stable identity.
    pub id: u64,
    /// The completion counts.
    pub completions: PerPersonCompletions,
    /// Whether the person was ever observed in the Idle state.
    pub ever_idle: bool,
}

/// State distribution of the population at one sampled boundary.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize)]
pub struct ActionCounts {
    /// Persons in the Idle state.
    pub idle: u64,
    /// Persons in a Moving phase.
    pub moving: u64,
    /// Persons actively Eating.
    pub eating: u64,
    /// Persons actively Sleeping.
    pub sleeping: u64,
    /// Persons actively Working.
    pub working: u64,
}

/// A per-simulated-day checkpoint sample (bounded; not a full event/trace log).
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub struct DaySample {
    /// Day index (1-based).
    pub day: u64,
    /// Committed instant at the day boundary, in seconds.
    pub seconds: i64,
    /// Population at the boundary.
    pub population: usize,
    /// State distribution.
    pub action_counts: ActionCounts,
    /// Live scheduler entries at the boundary.
    pub queue_depth: usize,
    /// Live + lazily-invalidated scheduler nodes at the boundary.
    pub queue_nodes: usize,
    /// Buffered (undrained) outcome events at the boundary.
    pub buffered_events: usize,
}

/// Timing/memory measurement fields — excluded from determinism equality.
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub struct ChaosMeasurement {
    /// Total wall-clock seconds.
    pub wall_seconds: f64,
    /// Sim-seconds per wall-second.
    pub sim_seconds_per_wall: f64,
    /// Committed outcome events per wall-second.
    pub events_per_wall: f64,
    /// Native peak RSS delta, or None when this run did not measure RSS.
    pub peak_rss_delta_bytes: Option<u64>,
}

/// The structured, deterministic chaos result (plus optional measurement).
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ChaosReport {
    /// Report/hash schema; v2 corrects movement and full-report comparison.
    pub schema_version: u32,
    /// Next due instant after completing all work at the final target.
    pub final_next_due_seconds: Option<i64>,
    /// Queue statistics sampled at each successful advance return, not heap peaks.
    pub queue_observations: crate::KernelObservations,
    /// The configuration that produced this report.
    pub config: ChaosConfig,
    /// The final committed instant, in seconds.
    pub final_instant_seconds: i64,
    /// The starting + final population (Phase 1 never removes a person).
    pub person_count: usize,
    /// Number of simulated-day samples retained.
    pub total_days: u64,
    /// Per-day checkpoint samples.
    pub day_samples: Vec<DaySample>,
    /// Aggregate completion counts summed across all persons.
    pub aggregate_completions: PerPersonCompletions,
    /// Per-person completion rows, ascending by id.
    pub per_person_completions: Vec<PerPersonCompletionRow>,
    /// How many persons completed Move, Eat, Sleep, and Work at least once.
    pub persons_completed_all_kinds: usize,
    /// Cumulative number of Idle-state observations across all samples.
    pub idle_observed_total: u64,
    /// Cumulative number of persons ever observed in the Idle state.
    pub idle_observed_persons: usize,
    /// Total committed outcome events (buffer-independent).
    pub events_total: u64,
    /// Cumulative FNV-1a-64 stream digest of every committed event.
    pub events_digest: u64,
    /// Per-person completion stream digest (drained order).
    pub per_person_digest: u64,
    /// Total decisions resolved.
    pub decisions_total: u64,
    /// Total action transitions committed.
    pub transitions_total: u64,
    /// Total advance rounds processed.
    pub rounds_total: u64,
    /// Observed max live scheduler entries.
    pub queue_depth_max: usize,
    /// Observed max scheduler nodes (live + stale).
    pub queue_nodes_max: usize,
    /// The canonical deterministic truth hash (measurement excluded).
    pub truth_hash: u64,
    /// Timing/memory measurement, if the bin filled it in.
    pub measurement: Option<ChaosMeasurement>,
    /// Documented invariant violations (empty on success).
    pub violated_invariants: Vec<String>,
    /// Death statistics marker.
    pub death_stats: &'static str,
    /// Event Store / database durability marker.
    pub database_consistency: &'static str,
}

impl ChaosReport {
    /// The deterministic-equality identity: everything except measurement.
    #[must_use]
    pub fn deterministic_id(&self) -> u64 {
        self.truth_hash
    }

    /// Compares all deterministic fields while ignoring wall-clock/RSS measurements.
    #[must_use]
    pub fn deterministic_eq(&self, other: &Self) -> bool {
        let mut left = self.clone();
        let mut right = other.clone();
        left.measurement = None;
        right.measurement = None;
        left == right
    }
}

/// Incrementally folds a stream of bytes into a deterministic 64-bit hash.
struct Fnv(u64);

impl Fnv {
    fn new() -> Self {
        Self(FNV_OFFSET)
    }

    fn byte(&mut self, value: u8) {
        self.0 ^= u64::from(value);
        self.0 = self.0.wrapping_mul(0x0100_0000_01b3);
    }

    fn u64(&mut self, value: u64) {
        for byte in value.to_le_bytes() {
            self.byte(byte);
        }
    }

    fn str(&mut self, value: &str) {
        for byte in value.as_bytes() {
            self.byte(*byte);
        }
    }

    fn finish(self) -> u64 {
        self.0
    }
}

/// A single frozen value view for a pass to the finite/bounds invariant check.
#[derive(Clone, Copy)]
struct BoundedValue {
    label: &'static str,
    raw: i64,
    min: i64,
    max: i64,
}

/// Returns a [`ChaosError::NonFinite`] when `value.raw` is outside the
/// documented inclusive bounds. This is the integer stand-in for a NaN/Inf
/// check because every Phase 1 quantity is an integer, so the honest detector
/// enforces the declared interval.
fn value_in_bounds(person: EntityId, value: BoundedValue) -> Option<ChaosError> {
    if value.raw < value.min || value.raw > value.max {
        Some(ChaosError::NonFinite {
            person,
            field: value.label,
            value: value.raw,
            min: value.min,
            max: value.max,
        })
    } else {
        None
    }
}

/// Returns a [`ChaosError::QueueGrowth`] when `value` exceeds `limit`.
#[must_use]
pub fn queue_bounded(key: &'static str, value: usize, limit: usize) -> Option<ChaosError> {
    (value > limit).then_some(ChaosError::QueueGrowth { key, value, limit })
}

/// Returns a [`ChaosError::DanglingReference`] when `actor` is not a member of
/// `population`. Stands in for the general "an id must resolve" check.
#[must_use]
pub fn actor_resolves(actor: EntityId, population: &BTreeSet<EntityId>) -> Option<ChaosError> {
    (!population.contains(&actor)).then_some(ChaosError::DanglingReference {
        detail: format!("event actor {} is not a live population id", actor.get()),
    })
}

/// The bounded per-person drive check (needs in `[0, NEED_MAX]`).
#[must_use]
pub fn needs_in_bounds(person: EntityId, needs: Needs) -> Option<ChaosError> {
    value_in_bounds(
        person,
        BoundedValue {
            label: "hunger",
            raw: needs.hunger().raw(),
            min: 0,
            max: NEED_MAX,
        },
    )
    .or_else(|| {
        value_in_bounds(
            person,
            BoundedValue {
                label: "fatigue",
                raw: needs.fatigue().raw(),
                min: 0,
                max: NEED_MAX,
            },
        )
    })
}

/// The documented live-entry and heap-size queue bounds for `person_count`.
#[must_use]
pub fn queue_limits(person_count: usize) -> (usize, usize) {
    // D2: at most two live schedule items per person; the heap includes lazily
    // invalidated nodes bounded by compaction (generous correctness slack).
    (2 * person_count, 8 * person_count)
}

/// Renders the invariant-check section of a person's view as a folded value.
#[must_use]
pub fn person_view_action_kind(view: &KernelPersonView) -> &'static str {
    match view.state() {
        ActionState::Idle => "Idle",
        ActionState::Moving { .. } => "Moving",
        ActionState::Eating => "Eating",
        ActionState::Sleeping => "Sleeping",
        ActionState::Working => "Working",
    }
}

/// Resolves a deterministic spawn layout: distinct walkable cells that share a
/// connected component containing a Meal, a Rest, **and** a Work site.
///
/// # Errors
/// Returns [`ChaosError::NoReachableFixture`] when no such component exists for
/// `count` cells, or [`ChaosError::Config`] when `count` is zero.
pub fn resolve_spawns(
    map: &WorldMap,
    sites: &ActivitySites,
    count: usize,
) -> Result<Vec<LocalCoord>, ChaosError> {
    if count == 0 {
        return Err(ChaosError::Config("person_count must be positive".into()));
    }
    let mut component: BTreeMap<LocalCoord, usize> = BTreeMap::new();
    let mut components: Vec<(usize, Vec<LocalCoord>)> = Vec::new();
    let mut next = 0_usize;

    for coord in map.local().coords() {
        if component.contains_key(&coord) {
            continue;
        }
        let Some(kind) = map.local().get(coord.x(), coord.y()) else {
            continue;
        };
        if !kind.is_walkable() {
            continue;
        }
        // BFS this connected walkable component (deterministic neighbor order).
        let id = next;
        next += 1;
        let mut cells = Vec::new();
        let mut queue = VecDeque::new();
        queue.push_back(coord);
        component.insert(coord, id);
        while let Some(current) = queue.pop_front() {
            cells.push(current);
            let (x, y) = (current.x(), current.y());
            for (dx, dy) in [(1_i32, 0_i32), (-1, 0), (0, 1), (0, -1)] {
                let Some(nbr) = LocalCoord::new(x + dx, y + dy) else {
                    continue;
                };
                if component.contains_key(&nbr) {
                    continue;
                }
                let Some(nk) = map.local().get(nbr.x(), nbr.y()) else {
                    continue;
                };
                if !nk.is_walkable() {
                    continue;
                }
                component.insert(nbr, id);
                queue.push_back(nbr);
            }
        }
        // Cells may not be in row-major order; sort for a deterministic pick.
        cells.sort_by_key(|cell| (cell.y(), cell.x()));
        components.push((id, cells));
    }

    // Tag each component with the site kinds it contains.
    let mut kinds: BTreeMap<usize, BTreeSet<SiteKind>> = BTreeMap::new();
    for site in all_sites(sites) {
        if let Some(&id) = component.get(&site.coord()) {
            kinds.entry(id).or_default().insert(site.kind());
        }
    }

    let mut candidates = Vec::new();
    for (id, cells) in &components {
        let Some(set) = kinds.get(id) else {
            continue;
        };
        if !(set.contains(&SiteKind::Meal)
            && set.contains(&SiteKind::Rest)
            && set.contains(&SiteKind::Work))
        {
            continue;
        }
        for cell in cells {
            candidates.push(*cell);
            if candidates.len() == count {
                return Ok(candidates);
            }
        }
    }

    Err(ChaosError::NoReachableFixture {
        reason: format!(
            "only {} reachable cells across components containing all site kinds",
            candidates.len()
        ),
    })
}

fn all_sites(sites: &ActivitySites) -> Vec<&ActivitySite> {
    let mut out = Vec::new();
    for kind in [SiteKind::Meal, SiteKind::Rest, SiteKind::Work] {
        out.extend(sites.sites_of(kind));
    }
    out
}

/// Determines whether a per-simulated-day checkpoint is clean.
fn check_person_views(
    kernel: &WorldKernel,
    population: &BTreeSet<EntityId>,
) -> Result<(), ChaosError> {
    let views = kernel
        .persons()
        .map_err(|_| ChaosError::Kernel(KernelError::KernelFaulted))?;
    if views.len() != population.len() {
        return Err(ChaosError::Population {
            expected: population.len(),
            actual: views.len(),
        });
    }
    for view in &views {
        if !population.contains(&view.id()) {
            return Err(ChaosError::Population {
                expected: population.len(),
                actual: views.len(),
            });
        }
        if let Some(error) = needs_in_bounds(view.id(), view.needs()) {
            return Err(error);
        }
        let Some(cell) = kernel
            .map()
            .local()
            .get(view.location().x(), view.location().y())
        else {
            return Err(ChaosError::Invariant {
                rule: "walkable location",
                detail: format!("person {} is outside map", view.id().get()),
            });
        };
        if !cell.is_walkable() {
            return Err(ChaosError::Invariant {
                rule: "walkable location",
                detail: format!("person {} is on blocked cell", view.id().get()),
            });
        }
        let valid = match view.state() {
            ActionState::Idle => {
                view.action() == palimpsest_sim_ai::ActionKind::Idle
                    && view.action_target().is_none()
            }
            ActionState::Moving { action } => {
                action != ActionKind::Idle
                    && action == view.action()
                    && view.action_target().is_some()
            }
            ActionState::Eating => {
                view.action() == palimpsest_sim_ai::ActionKind::Eat
                    && view.action_target().is_some()
            }
            ActionState::Sleeping => {
                view.action() == palimpsest_sim_ai::ActionKind::Sleep
                    && view.action_target().is_some()
            }
            ActionState::Working => {
                view.action() == palimpsest_sim_ai::ActionKind::Work
                    && view.action_target().is_some()
            }
        };
        let at_activity = matches!(
            view.state(),
            ActionState::Eating | ActionState::Sleeping | ActionState::Working
        );
        let target_valid = view.action_target().is_none_or(|target| {
            kernel
                .map()
                .local()
                .get(target.x(), target.y())
                .is_some_and(|cell| cell.is_walkable())
                && (!at_activity || target == view.location())
        });
        if !valid || !target_valid {
            return Err(ChaosError::Invariant {
                rule: "action/state/target consistency",
                detail: format!("person {} mismatch", view.id().get()),
            });
        }
    }
    Ok(())
}

/// Checks the queue-growth and buffer identity bounds at a committed boundary.
fn check_queue_and_buffer(
    metrics: KernelMetrics,
    delivered: u64,
    person_count: usize,
) -> Result<(), ChaosError> {
    let (live_limit, node_limit) = queue_limits(person_count);
    if let Some(error) = queue_bounded(
        "scheduled_entries",
        metrics.scheduler_queue_depth,
        live_limit,
    ) {
        return Err(error);
    }
    let nodes = metrics
        .scheduler_queue_depth
        .saturating_add(metrics.scheduler_stale_nodes);
    if let Some(error) = queue_bounded("queue_nodes", nodes, node_limit) {
        return Err(error);
    }
    // Buffer identity: total = delivered + buffered + rotated (buffer-independent).
    let accounted = delivered
        .saturating_add(metrics.events_buffered as u64)
        .saturating_add(metrics.events_rotated);
    if accounted != metrics.events_total {
        return Err(ChaosError::Invariant {
            rule: "event_total = delivered + buffered + rotated",
            detail: format!(
                "accounted {accounted} != total {} (buffered {}, rotated {})",
                metrics.events_total, metrics.events_buffered, metrics.events_rotated
            ),
        });
    }
    Ok(())
}

fn observe_idle(
    kernel: &WorldKernel,
    ever_idle: &mut BTreeSet<EntityId>,
    idle_observations: &mut u64,
) -> Result<(), ChaosError> {
    let views = kernel
        .persons()
        .map_err(|_| ChaosError::Kernel(KernelError::KernelFaulted))?;
    for view in &views {
        if view.state() == ActionState::Idle {
            ever_idle.insert(view.id());
            *idle_observations = idle_observations.saturating_add(1);
        }
    }
    Ok(())
}

fn validate_event_references(
    record: &crate::EventRecord,
    population: &BTreeSet<EntityId>,
) -> Result<(), ChaosError> {
    record.validate().map_err(|error| ChaosError::Invariant {
        rule: "event validation",
        detail: error.to_string(),
    })?;
    for actor in record.actors() {
        if let Some(error) = actor_resolves(*actor, population) {
            return Err(error);
        }
    }
    for target in record.targets() {
        if let Some(error) = actor_resolves(*target, population) {
            return Err(error);
        }
    }
    Ok(())
}

/// Drains a kernel's outcome buffer, hashing the observed completion stream.
///
/// Returns the number of events drained this call and a continuous per-person
/// completion digest. Also rejects a dangling actor reference.
fn drain_and_count(
    kernel: &mut WorldKernel,
    digest: &mut Fnv,
    population: &BTreeSet<EntityId>,
) -> Result<u64, ChaosError> {
    let records = kernel.drain_events();
    let count = records.len() as u64;
    for record in &records {
        validate_event_references(record, population)?;
        if record.event_type() == "action.completed" {
            let kind = record
                .metadata()
                .get("action_kind")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default()
                .to_owned();
            for actor in record.actors() {
                digest.u64(actor.get());
                digest.str(if kind.is_empty() { "" } else { &kind });
            }
        }
    }
    Ok(count)
}

/// Runs the chaos simulation to completion, returning a deterministic report.
///
/// `require_all_kinds` forces the Phase 1 completion requirement: every person
/// must have completed an Eat, a Sleep, a Work, and a real (non-teleport)
/// movement phase. A top-level standalone `Move` is *not* required — persons
/// reach an activity site through each activity's own movement phase, which is
/// what is counted — so the runner never has to manufacture a selection. The
/// CLI passes `false` so the completed report is independent of the gate; the
/// acceptance harness passes `true`.
///
/// # Errors
/// Returns a [`ChaosError`] on any config problem, kernel error, or first
/// invariant/liveness violation.
#[allow(clippy::too_many_lines)]
pub fn run_chaos(config: &ChaosConfig, require_all_kinds: bool) -> Result<ChaosReport, ChaosError> {
    run_chaos_internal(config, require_all_kinds, None)
}

type ChaosObserver<'a> = &'a mut dyn FnMut(ChaosCheckpoint, &WorldKernel);

#[allow(clippy::too_many_lines)]
fn run_chaos_internal(
    config: &ChaosConfig,
    require_all_kinds: bool,
    mut observer: Option<ChaosObserver<'_>>,
) -> Result<ChaosReport, ChaosError> {
    let target_seconds = config.target_seconds()?;
    let mut kernel = build_chaos_kernel(config)?;
    let population: BTreeSet<EntityId> = kernel
        .persons()
        .map_err(|_| ChaosError::Kernel(KernelError::KernelFaulted))?
        .iter()
        .map(KernelPersonView::id)
        .collect();
    if let Some(callback) = observer.as_deref_mut() {
        callback(ChaosCheckpoint::Prepared, &kernel);
    }

    // Drive day-by-day so checkpoints are exact simulated-day boundaries.
    let mut day = 0_u64;
    let mut now_seconds = 0_i64;
    let mut samples = Vec::new();
    let mut per_person: BTreeMap<EntityId, PerPersonCompletions> = population
        .iter()
        .copied()
        .map(|id| (id, PerPersonCompletions::default()))
        .collect();
    let mut ever_idle: BTreeSet<EntityId> = BTreeSet::new();
    let mut idle_observations = 0_u64;
    let mut digest = Fnv::new();
    let mut delivered = 0_u64;
    let mut stalled = 0_usize;

    while now_seconds < target_seconds {
        let day_target = now_seconds
            .saturating_add(SECONDS_PER_DAY)
            .min(target_seconds);
        let target = crate::SimInstant::from_seconds(day_target);
        let mut day_calls = 0_usize;
        loop {
            let last_committed = kernel.now().as_seconds();
            let advance = kernel.advance(target)?;
            if let Some(callback) = observer.as_deref_mut() {
                callback(ChaosCheckpoint::Advance, &kernel);
            }
            delivered =
                delivered.saturating_add(drain_and_count(&mut kernel, &mut digest, &population)?);
            observe_idle(&kernel, &mut ever_idle, &mut idle_observations)?;
            check_queue_and_buffer(kernel.metrics(), delivered, config.person_count)?;
            check_person_views(&kernel, &population)?;
            let current = kernel.now().as_seconds();
            if current <= last_committed {
                stalled += 1;
                if stalled >= MAX_STALLED_ADVANCE_CALLS {
                    return Err(ChaosError::NonTerminating {
                        committed_to_seconds: current,
                        stalled_calls: stalled,
                    });
                }
            } else {
                stalled = 0;
            }
            day_calls += 1;
            if day_calls > MAX_ADVANCE_CALLS_PER_DAY {
                return Err(ChaosError::NonTerminating {
                    committed_to_seconds: current,
                    stalled_calls: day_calls,
                });
            }
            if advance.reached_target() {
                break;
            }
        }
        now_seconds = kernel.now().as_seconds();
        day += 1;

        let metrics = kernel.metrics();
        check_queue_and_buffer(metrics, delivered, config.person_count)?;
        check_person_views(&kernel, &population)?;

        let views = kernel
            .persons()
            .map_err(|_| ChaosError::Kernel(KernelError::KernelFaulted))?;
        let mut counts = ActionCounts::default();
        for view in &views {
            match person_view_action_kind(view) {
                "Idle" => counts.idle += 1,
                "Moving" => counts.moving += 1,
                "Eating" => counts.eating += 1,
                "Sleeping" => counts.sleeping += 1,
                "Working" => counts.working += 1,
                _ => {}
            }
        }
        let nodes = metrics
            .scheduler_queue_depth
            .saturating_add(metrics.scheduler_stale_nodes);
        samples.push(DaySample {
            day,
            seconds: now_seconds,
            population: views.len(),
            action_counts: counts,
            queue_depth: metrics.scheduler_queue_depth,
            queue_nodes: nodes,
            buffered_events: metrics.events_buffered,
        });
        if let Some(callback) = observer.as_deref_mut() {
            callback(ChaosCheckpoint::Day, &kernel);
        }
    }

    // Final population preservation is a hard invariant (no Phase 1 removal).
    let metrics = kernel.metrics();
    if let Some(next_due) = kernel
        .next_due()
        .map_err(|_| ChaosError::Kernel(KernelError::KernelFaulted))?
        && next_due <= kernel.now()
    {
        return Err(ChaosError::Invariant {
            rule: "final next_due strictly future",
            detail: format!(
                "next due {} at now {}",
                next_due.as_seconds(),
                kernel.now().as_seconds()
            ),
        });
    }
    if metrics.person_count != config.person_count {
        return Err(ChaosError::Population {
            expected: config.person_count,
            actual: metrics.person_count,
        });
    }

    let final_next_due_seconds = kernel
        .next_due()
        .map_err(|_| ChaosError::Kernel(KernelError::KernelFaulted))?
        .map(crate::SimInstant::as_seconds);
    // Kernel observations are the authoritative completion source: event
    // retention is bounded and therefore cannot be used for this accounting.
    let observations = kernel
        .observations()
        .map_err(|_| ChaosError::Kernel(KernelError::KernelFaulted))?;
    for (id, observed) in &observations.persons {
        let row = per_person.entry(*id).or_default();
        row.moves = observed.moves;
        row.eats = observed.eats;
        row.sleeps = observed.sleeps;
        row.works = observed.works;
        row.movement_phases = observed.movement_phases;
    }
    let aggregate =
        per_person
            .values()
            .copied()
            .fold(PerPersonCompletions::default(), |mut acc, item| {
                acc.moves = acc.moves.saturating_add(item.moves);
                acc.eats = acc.eats.saturating_add(item.eats);
                acc.sleeps = acc.sleeps.saturating_add(item.sleeps);
                acc.works = acc.works.saturating_add(item.works);
                acc.movement_phases = acc.movement_phases.saturating_add(item.movement_phases);
                acc
            });
    let persons_completed_all_kinds = per_person
        .values()
        .filter(|row| row.kinds_completed() == 4)
        .count();
    let per_person_rows: Vec<PerPersonCompletionRow> = per_person
        .iter()
        .map(|(id, completions)| PerPersonCompletionRow {
            id: id.get(),
            completions: *completions,
            ever_idle: ever_idle.contains(id),
        })
        .collect();
    if require_all_kinds {
        for row in &per_person_rows {
            if row.completions.kinds_completed() != 4 {
                return Err(ChaosError::UnmetCompletion {
                    detail: format!(
                        "person {} completed {} of 4 action kinds",
                        row.id,
                        row.completions.kinds_completed()
                    ),
                });
            }
        }
    }
    let truth_hash = compute_truth_hash(config, &kernel, &per_person_rows);
    let mut report = ChaosReport {
        schema_version: 2,
        final_next_due_seconds,
        queue_observations: observations.clone(),
        config: *config,
        final_instant_seconds: now_seconds,
        person_count: config.person_count,
        total_days: day,
        day_samples: samples,
        aggregate_completions: aggregate,
        per_person_completions: per_person_rows,
        persons_completed_all_kinds,
        idle_observed_total: idle_observations,
        idle_observed_persons: ever_idle.len(),
        events_total: metrics.events_total,
        events_digest: metrics.events_digest,
        per_person_digest: digest.finish(),
        decisions_total: metrics.decisions_total,
        transitions_total: metrics.transitions_total,
        rounds_total: metrics.rounds_total,
        queue_depth_max: observations.queue_depth_max,
        queue_nodes_max: observations.queue_nodes_max,
        truth_hash,
        measurement: None,
        violated_invariants: Vec::new(),
        death_stats: "NotApplicable",
        database_consistency: "NotApplicable",
    };

    // Fold every deterministic report field, including daily samples and next_due.
    let mut full_hash = Fnv::new();
    for byte in serde_json::to_vec(&report).expect("integer report serializes") {
        full_hash.byte(byte);
    }
    report.truth_hash = full_hash.finish();
    if let Some(callback) = observer {
        callback(ChaosCheckpoint::Complete, &kernel);
    }

    Ok(report)
}

/// Runs chaos while exposing read-only lifecycle callbacks to a supervisor.
///
/// # Errors
/// Returns the same config, kernel, or invariant errors as [`run_chaos`].
pub fn run_chaos_observed(
    config: &ChaosConfig,
    require_all_kinds: bool,
    observer: &mut dyn FnMut(ChaosCheckpoint, &WorldKernel),
) -> Result<ChaosReport, ChaosError> {
    run_chaos_internal(config, require_all_kinds, Some(observer))
}

/// The canonical deterministic truth hash (measurement fields excluded).
fn compute_truth_hash(
    config: &ChaosConfig,
    kernel: &WorldKernel,
    rows: &[PerPersonCompletionRow],
) -> u64 {
    let mut fnv = Fnv::new();
    fnv.str("chaos-report-v2");
    fnv.u64(config.seed);
    fnv.u64(config.person_count as u64);
    fnv.u64(config.years);
    fnv.u64(config.sim_seconds_per_year as u64);
    fnv.u64(kernel.now().as_seconds() as u64);

    fnv.str("persons");
    fnv.u64(rows.len() as u64);
    if let Ok(views) = kernel.persons() {
        for view in &views {
            fnv.u64(view.id().get());
            fnv.u64(view.location().x() as u64);
            fnv.u64(view.location().y() as u64);
            fnv.str(person_view_action_kind(view));
            fnv.str(match view.action() {
                ActionKind::Move => "Move",
                ActionKind::Eat => "Eat",
                ActionKind::Sleep => "Sleep",
                ActionKind::Work => "Work",
                ActionKind::Idle => "Idle",
            });
            fnv.u64(view.needs().hunger().raw() as u64);
            fnv.u64(view.needs().fatigue().raw() as u64);
            if let Some(target) = view.action_target() {
                fnv.byte(1);
                fnv.u64(target.x() as u64);
                fnv.u64(target.y() as u64);
            } else {
                fnv.byte(0);
                fnv.u64(u64::MAX);
            }
        }
    }

    if let Ok(observations) = kernel.observations() {
        fnv.str("observations");
        fnv.u64(observations.boundary_count);
        for (id, row) in &observations.persons {
            fnv.u64(id.get());
            for value in [
                row.movement_steps,
                row.movement_phases,
                row.moves,
                row.eats,
                row.sleeps,
                row.works,
                row.idles,
            ] {
                fnv.u64(value);
            }
        }
        fnv.u64(observations.queue_depth_sum);
        fnv.u64(observations.queue_depth_max as u64);
        fnv.u64(observations.queue_nodes_sum);
        fnv.u64(observations.queue_nodes_max as u64);
    }
    fnv.str("checkpoints");
    if let Ok(observations) = kernel.observations() {
        fnv.u64(observations.boundary_count);
    }
    fnv.str("days");
    if let Ok(value) = serde_json::to_vec(&rows) {
        for byte in value {
            fnv.byte(byte);
        }
    }

    fnv.str("sites");
    if let Ok(sites) = kernel.sites() {
        for site in all_sites(sites) {
            fnv.u64(site.coord().x() as u64);
            fnv.u64(site.coord().y() as u64);
            fnv.str(match site.kind() {
                SiteKind::Meal => "Meal",
                SiteKind::Rest => "Rest",
                SiteKind::Work => "Work",
            });
            if let Some(work) = site.work() {
                fnv.u64(work.get());
            }
        }
    }

    let metrics = kernel.metrics();
    fnv.u64(metrics.rounds_total);
    fnv.u64(metrics.transitions_total);
    fnv.u64(metrics.decisions_total);
    fnv.u64(metrics.events_total);
    fnv.u64(metrics.events_digest);
    fnv.u64(metrics.scheduler_queue_depth as u64);
    fnv.finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    use palimpsest_sim_ai::{NeedValue, Needs};

    #[test]
    fn needs_in_bounds_rejects_out_of_range() {
        let ok = Needs::new(NeedValue::MIN, NeedValue::MAX);
        assert!(needs_in_bounds(EntityId::MIN, ok).is_none());
        // A NeedValue is always constructed in range, so exercise the predicate
        // directly on a raw integer that is structurally out of range.
        let error = value_in_bounds(
            EntityId::MIN,
            BoundedValue {
                label: "hunger",
                raw: NEED_MAX + 1,
                min: 0,
                max: NEED_MAX,
            },
        );
        assert!(matches!(error, Some(ChaosError::NonFinite { .. })));
        let negative = value_in_bounds(
            EntityId::MIN,
            BoundedValue {
                label: "fatigue",
                raw: -1,
                min: 0,
                max: NEED_MAX,
            },
        );
        assert!(matches!(negative, Some(ChaosError::NonFinite { .. })));
    }

    #[test]
    fn queue_detector_fires_on_growth() {
        let (live, nodes) = queue_limits(100);
        assert!(queue_bounded("scheduled_entries", live, live).is_none());
        assert!(queue_bounded("scheduled_entries", live + 1, live).is_some());
        assert!(queue_bounded("queue_nodes", nodes + 1, nodes).is_some());
    }

    #[test]
    fn dangling_reference_detector_fires() {
        let mut population = BTreeSet::new();
        population.insert(EntityId::MIN);
        assert!(actor_resolves(EntityId::MIN, &population).is_none());
        let other = EntityId::new(999).expect("non-exhausted id");
        assert!(actor_resolves(other, &population).is_some());
    }

    #[test]
    fn deterministic_two_day_run_is_stable_and_population_preserved() {
        let config = ChaosConfig {
            years: 1,
            sim_seconds_per_year: 86_400 * 2,
            ..ChaosConfig::default()
        };
        let first = run_chaos(&config, false).expect("short run succeeds");
        let second = run_chaos(&config, false).expect("short run succeeds");
        assert_eq!(first.truth_hash, second.truth_hash);
        assert!(first.deterministic_eq(&second));
        let mut corrupted = second.clone();
        corrupted.day_samples[0].queue_depth += 1;
        assert!(!first.deterministic_eq(&corrupted));
        assert_eq!(first.events_total, second.events_total);
        assert_eq!(first.events_digest, second.events_digest);
        assert_eq!(first.per_person_digest, second.per_person_digest);
        assert_eq!(first.day_samples.len(), 2);
        assert_eq!(first.person_count, 100);
        assert!(first.violated_invariants.is_empty());
        assert_eq!(first.death_stats, "NotApplicable");
        assert_eq!(first.database_consistency, "NotApplicable");
    }

    #[test]
    fn different_seed_produces_different_world() {
        let config = ChaosConfig {
            years: 1,
            sim_seconds_per_year: 86_400,
            ..ChaosConfig::default()
        };
        let base = run_chaos(&config, false).expect("seed 42 runs");
        let other = ChaosConfig { seed: 1, ..config };
        let changed = run_chaos(&other, false).expect("seed 1 runs");
        assert_ne!(base.truth_hash, changed.truth_hash);
    }

    #[test]
    fn idle_is_detected_when_no_site_is_reachable() {
        let config = ChaosConfig::default();
        let map = WorldMap::generate(WorldSeed::new(config.seed), WorldGenConfig::default());
        // An empty site set is valid (sim-world test) and leaves only Idle
        // viable, so the Idle observation instrument must report it.
        let sites = ActivitySites::new(Vec::new()).expect("empty site set is valid");
        let origin = map
            .local()
            .coords()
            .find(|coord| {
                map.local()
                    .get(coord.x(), coord.y())
                    .is_some_and(|kind| kind.is_walkable())
            })
            .expect("a walkable spawn cell");
        let mut kernel = WorldKernel::new(map, sites, KernelConfig::default());
        let id = kernel.spawn_person(origin).expect("spawn");
        kernel.start_world(crate::SimInstant::EPOCH).expect("start");
        kernel
            .advance(crate::SimInstant::from_seconds(60))
            .expect("advance");
        let mut ever_idle = BTreeSet::new();
        let mut observations = 0;
        observe_idle(&kernel, &mut ever_idle, &mut observations).expect("observe");
        assert!(ever_idle.contains(&id));
        assert!(observations > 0);
    }

    #[test]
    fn valid_actor_does_not_hide_a_dangling_event_target() {
        let mut record = crate::EventRecord::new(
            crate::EventId::new(1).unwrap(),
            crate::SimInstant::EPOCH,
            "action.completed",
        )
        .unwrap();
        record.add_actor(EntityId::MIN).unwrap();
        record.add_target(EntityId::new(999).unwrap()).unwrap();
        let population = BTreeSet::from([EntityId::MIN]);
        assert!(matches!(
            validate_event_references(&record, &population),
            Err(ChaosError::DanglingReference { .. })
        ));
    }

    #[test]
    fn zero_person_count_is_rejected() {
        let config = ChaosConfig {
            person_count: 0,
            ..ChaosConfig::default()
        };
        assert!(matches!(
            run_chaos(&config, false),
            Err(ChaosError::Config(_))
        ));
    }

    #[test]
    fn fixture_has_three_site_kinds_reachable() {
        let config = ChaosConfig::default();
        let map = WorldMap::generate(WorldSeed::new(config.seed), WorldGenConfig::default());
        let sites = ActivitySites::place_defaults(&map);
        let spawns = resolve_spawns(&map, &sites, config.person_count).expect("fixture reachable");
        assert_eq!(spawns.len(), config.person_count);
    }
}
