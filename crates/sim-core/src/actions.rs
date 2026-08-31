// Authored by Kimi Code (AI coding agent) — task CHRON-027.
//! Phase 1 action execution state machine (CHRON-027, ADR-0021).
//!
//! [`ActionRuntime`] owns at most one execution record per person and drives
//! it over [`SimInstant`] time through a due-time/FIFO [`Scheduler`]
//! (ADR-0004). It executes exactly the five CHRON-025 action kinds — `Move`,
//! `Eat`, `Sleep`, `Work`, `Idle` — with atomic single-commit transitions,
//! deterministic blocked/failed recovery to `Idle`, and interruption only via
//! an explicit cancel that the Utility driver requests after a fresh
//! selection (ADR-0014/0018/0019; Master Spec §14). It never scores, selects,
//! or invents an action; decision requests are surfaced to the caller.
//!
//! Movement costs one simulated second per 4-directional cell; `find_path`
//! paths include the start cell, which is never re-walked. Every action
//! occupies at least one second (a zero-distance arrival still ticks once),
//! so same-instant completion loops are unrepresentable. Needs grow from real
//! elapsed seconds exactly once per person and are materialized at every
//! transition boundary; completion first accrues growth to that instant and
//! only then applies the Eat/Sleep relief. High-level outcomes are appended
//! as validated, bounded in-memory [`EventRecord`] values (ADR-0006); this is
//! a runtime diagnostic sink, not durable retention.

use std::collections::{HashMap, VecDeque};
use std::fmt::{Display, Formatter};

use palimpsest_sim_ai::{
    ActionCandidate, ActionKind, CandidateContext, DecisionError, Needs, PerturbationSpec,
    Selection, Weights, candidate_actions, select_action,
};
use palimpsest_sim_ai::{CRITICAL_PRESSURE, FATIGUE_RATE_PER_SECOND, HUNGER_RATE_PER_SECOND};
use palimpsest_sim_entity::EntityId;
use palimpsest_sim_events::{EventId, EventRecord};
use palimpsest_sim_scheduler::{ScheduleToken, Scheduler, SchedulerError, SchedulerMetrics};
use palimpsest_sim_time::{SimDuration, SimInstant};
use palimpsest_sim_world::{
    ActivitySites, LocalCoord, Path, PathConfig, SiteKind, TerrainKind, WorldMap, find_path,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::person::PersonRuntime;

/// Hunger relief applied by a completed Eat action (P1-REMAINING D1): one
/// full drive (`NEED_MAX` raw units), saturating at zero inside `Needs`.
pub const EAT_RELIEF: i64 = 100_000;
/// Fatigue relief applied by a completed Sleep action; same contract as
/// [`EAT_RELIEF`].
pub const REST_RELIEF: i64 = 100_000;
/// Capacity of the bounded in-memory outcome-event buffer. Overflow drops the
/// oldest record and increments the visible rotation counter; this is a
/// diagnostic sink, not durable retention (ADR-0021 §5).
pub const EVENT_BUFFER_CAPACITY: usize = 4_096;

/// FNV-1a-64 offset basis for the committed-event stream digest (ADR-0024 D5).
pub(crate) const EVENT_DIGEST_OFFSET: u64 = 14_695_981_039_346_656_037;
/// FNV-1a-64 prime for the committed-event stream digest.
const EVENT_DIGEST_PRIME: u64 = 1_099_511_628_211;

/// Folds `bytes` through the FNV-1a-64 hash starting from `hash` (explicit
/// wrapping multiply; deterministic diagnostic only, not anti-collision).
fn fold_digest(mut hash: u64, bytes: &[u8]) -> u64 {
    for byte in bytes {
        hash = (hash ^ u64::from(*byte)).wrapping_mul(EVENT_DIGEST_PRIME);
    }
    hash
}

/// Timing and pathfinding configuration for action execution.
///
/// The `Default` table is the P1-REMAINING D1 Phase 1 set: one second per
/// movement cell, Eat 600s, Sleep 28,800s, Work 1,800s, Idle wait 60s, retry
/// delay 1s, critical recheck delay 60s, and the default pathfinding budget.
/// These are validation tuning values, not final MVP balancing.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ActionConfig {
    move_step: SimDuration,
    eat: SimDuration,
    sleep: SimDuration,
    work: SimDuration,
    idle_wait: SimDuration,
    retry_delay: SimDuration,
    critical_recheck_delay: SimDuration,
    path: PathConfig,
}

impl ActionConfig {
    /// Creates a configuration; every duration must be at least one second so
    /// every action occupies positive time and no same-instant loop exists.
    // A flat record of the seven D1 timings plus the path budget: grouping
    // them into nested structs would not add meaning.
    #[allow(clippy::too_many_arguments)]
    #[must_use]
    pub fn new(
        move_step: SimDuration,
        eat: SimDuration,
        sleep: SimDuration,
        work: SimDuration,
        idle_wait: SimDuration,
        retry_delay: SimDuration,
        critical_recheck_delay: SimDuration,
        path: PathConfig,
    ) -> Option<Self> {
        let durations = [
            move_step,
            eat,
            sleep,
            work,
            idle_wait,
            retry_delay,
            critical_recheck_delay,
        ];
        if durations.iter().any(|duration| duration.as_seconds() < 1) {
            return None;
        }
        Some(Self {
            move_step,
            eat,
            sleep,
            work,
            idle_wait,
            retry_delay,
            critical_recheck_delay,
            path,
        })
    }

    /// Seconds per 4-directional movement cell.
    #[must_use]
    pub const fn move_step(&self) -> SimDuration {
        self.move_step
    }

    /// Duration of the activity phase of `kind`; `Move` has none.
    #[must_use]
    pub const fn activity_duration(&self, kind: ActionKind) -> Option<SimDuration> {
        match kind {
            ActionKind::Eat => Some(self.eat),
            ActionKind::Sleep => Some(self.sleep),
            ActionKind::Work => Some(self.work),
            ActionKind::Idle => Some(self.idle_wait),
            ActionKind::Move => None,
        }
    }

    /// Delay before a blocked/failed action may be re-decided.
    #[must_use]
    pub const fn retry_delay(&self) -> SimDuration {
        self.retry_delay
    }

    /// Positive delay between critical-need rechecks while a person stays
    /// critical.
    #[must_use]
    pub const fn critical_recheck_delay(&self) -> SimDuration {
        self.critical_recheck_delay
    }

    /// The pathfinding budget applied to every execution-time path query.
    #[must_use]
    pub const fn path(&self) -> PathConfig {
        self.path
    }
}

impl Default for ActionConfig {
    fn default() -> Self {
        fn seconds(value: i64) -> SimDuration {
            SimDuration::from_seconds(value).expect("positive constant durations are valid")
        }
        Self {
            move_step: seconds(1),
            eat: seconds(600),
            sleep: seconds(28_800),
            work: seconds(1_800),
            idle_wait: seconds(60),
            retry_delay: seconds(1),
            critical_recheck_delay: seconds(60),
            path: PathConfig::default(),
        }
    }
}

/// The observable state of a person's current action (CHRON-027 contract).
///
/// A person executing an Idle wait reports `Idle` while still counting as
/// busy; use [`ActionRuntime::current_action`] to distinguish "executing an
/// Idle wait" from "no active action".
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ActionState {
    /// No active action, or an Idle wait in progress.
    Idle,
    /// Moving toward the recorded action's target (also the sole phase of a
    /// standalone `Move`).
    Moving {
        /// The top-level action this movement phase belongs to.
        action: ActionKind,
    },
    /// Performing Eat at the target Meal site.
    Eating,
    /// Performing Sleep at the target Rest site.
    Sleeping,
    /// Performing Work at the target Work site.
    Working,
}

/// Why a [`Transition`] was committed.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TransitionReason {
    /// `start` accepted a new action.
    Started,
    /// One movement cell was traversed.
    Step,
    /// The movement phase reached the target.
    Arrived,
    /// The action completed successfully.
    Completed,
    /// The target was unavailable at the arrival recheck.
    Blocked,
    /// Execution failed (e.g. the work counter rejected the completion).
    Failed,
    /// A higher-priority decision superseded the action.
    Interrupted,
    /// An external cancel ended the action.
    Cancelled,
}

/// A single atomic, committed state change (CHRON-027 API contract).
///
/// No state change is observable before its transition is committed; the
/// vector returned by [`ActionRuntime::advance`] preserves scheduler
/// due-time/FIFO order.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Transition {
    person: EntityId,
    from: ActionState,
    to: ActionState,
    action: ActionKind,
    target: Option<LocalCoord>,
    at: SimInstant,
    reason: TransitionReason,
    location: LocalCoord,
}

impl Transition {
    /// The person whose action changed.
    #[must_use]
    pub const fn person(&self) -> EntityId {
        self.person
    }

    /// The state before the commit.
    #[must_use]
    pub const fn from(&self) -> ActionState {
        self.from
    }

    /// The state after the commit.
    #[must_use]
    pub const fn to(&self) -> ActionState {
        self.to
    }

    /// The top-level action kind of the record.
    #[must_use]
    pub const fn action(&self) -> ActionKind {
        self.action
    }

    /// The action target, if any.
    #[must_use]
    pub const fn target(&self) -> Option<LocalCoord> {
        self.target
    }

    /// The instant the transition committed.
    #[must_use]
    pub const fn at(&self) -> SimInstant {
        self.at
    }

    /// Why the transition committed.
    #[must_use]
    pub const fn reason(&self) -> TransitionReason {
        self.reason
    }

    /// The person's location immediately after the commit.
    #[must_use]
    pub const fn location(&self) -> LocalCoord {
        self.location
    }
}

/// Why an active action is being cancelled.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CancelReason {
    /// A fresh selection elected a different `(kind, target)`; the full
    /// `DecisionTrace` stays with the driver (ADR-0014).
    Interrupted,
    /// A caller-initiated stop outside the decision loop.
    External,
}

/// Why the executor asks the driver for a fresh decision.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DecisionReason {
    /// The previous action completed normally.
    Completed,
    /// A blocked/failed action's retry delay elapsed.
    Retry,
    /// A per-person critical-need boundary was reached.
    CriticalBoundary,
}

/// A surfaced re-decision trigger; the executor never selects itself.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DecisionRequest {
    person: EntityId,
    reason: DecisionReason,
    at: SimInstant,
}

impl DecisionRequest {
    /// The person needing a decision.
    #[must_use]
    pub const fn person(&self) -> EntityId {
        self.person
    }

    /// Why the decision is requested.
    #[must_use]
    pub const fn reason(&self) -> DecisionReason {
        self.reason
    }

    /// The instant the request became due.
    #[must_use]
    pub const fn at(&self) -> SimInstant {
        self.at
    }
}

/// The committed result of one [`ActionRuntime::advance`] call.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct AdvanceOutcome {
    transitions: Vec<Transition>,
    decision_requests: Vec<DecisionRequest>,
}

impl AdvanceOutcome {
    /// Committed transitions in scheduler order.
    #[must_use]
    pub fn transitions(&self) -> &[Transition] {
        &self.transitions
    }

    /// Decision requests in scheduler order.
    #[must_use]
    pub fn decision_requests(&self) -> &[DecisionRequest] {
        &self.decision_requests
    }
}

/// Monotonic execution counters. Movement-phase completions (arrivals) and
/// top-level action completions are recorded separately (P1-REMAINING D1).
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ActionStats {
    /// Actions accepted by `start`.
    pub started: u64,
    /// Movement cells traversed.
    pub steps: u64,
    /// Movement-phase completions (arrivals at the target).
    pub movement_completions: u64,
    /// Completed standalone Move actions.
    pub move_completions: u64,
    /// Completed Eat actions.
    pub eat_completions: u64,
    /// Completed Sleep actions.
    pub sleep_completions: u64,
    /// Completed Work actions.
    pub work_completions: u64,
    /// Completed Idle waits.
    pub idle_completions: u64,
    /// Blocked recoveries.
    pub blocked: u64,
    /// Failed recoveries.
    pub failed: u64,
    /// Interrupted actions.
    pub interrupted: u64,
    /// Externally cancelled actions.
    pub cancelled: u64,
}

/// Read-only runtime health snapshot for Developer Metrics.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ActionRuntimeMetrics {
    /// Underlying scheduler health.
    pub scheduler: SchedulerMetrics,
    /// Persons with an active execution record.
    pub live_actions: usize,
    /// Persons with a pending retry token and no active record.
    pub pending_retries: usize,
    /// Live critical-need check tokens.
    pub live_checks: usize,
    /// Buffered outcome events.
    pub events_buffered: usize,
    /// Outcome events dropped by buffer rotation.
    pub events_rotated: u64,
    /// Total high-level outcome events ever committed (independent of buffer
    /// retention; ADR-0024 D5).
    pub events_total: u64,
    /// Cumulative FNV-1a-64 stream digest of every committed event.
    pub events_digest: u64,
}

/// Failures of action execution operations (CHRON-027 API contract).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ActionError {
    /// No person with this stable identity exists.
    UnknownPerson {
        /// The identity that was not found.
        id: EntityId,
    },
    /// The person already has an active action (including an Idle wait).
    AlreadyExecuting {
        /// The busy person.
        id: EntityId,
    },
    /// The candidate's kind/target combination is not executable as given.
    /// Structurally unreachable through the validated `ActionCandidate`
    /// constructor (ADR-0019); retained as a defensive boundary.
    InvalidTarget {
        /// The offending action kind.
        kind: ActionKind,
    },
    /// The target site is missing or of the wrong kind.
    Blocked {
        /// The blocked action kind.
        kind: ActionKind,
        /// The unavailable target.
        target: LocalCoord,
    },
    /// No path connects the person's location to the target under the
    /// configured budget.
    Unreachable {
        /// The action kind that cannot route.
        kind: ActionKind,
        /// The unreachable target.
        target: LocalCoord,
    },
    /// The referenced action was already interrupted.
    Interrupted {
        /// The affected person.
        id: EntityId,
    },
    /// The requested transition does not exist (e.g. cancelling a person with
    /// no active action).
    InvalidTransition {
        /// The affected person.
        id: EntityId,
    },
    /// A time computation overflowed or went backwards.
    TimeOverflow {
        /// The affected person.
        id: EntityId,
    },
    /// The bounded event log could not allocate a fresh event identity.
    EventLogExhausted,
    /// The underlying scheduler rejected work.
    Schedule {
        /// The scheduler failure.
        source: SchedulerError,
    },
}

impl Display for ActionError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownPerson { id } => write!(formatter, "unknown person {}", id.get()),
            Self::AlreadyExecuting { id } => {
                write!(
                    formatter,
                    "person {} already has an active action",
                    id.get()
                )
            }
            Self::InvalidTarget { kind } => {
                write!(formatter, "invalid target for {kind:?}")
            }
            Self::Blocked { kind, target } => write!(
                formatter,
                "{kind:?} target ({}, {}) is unavailable",
                target.x(),
                target.y()
            ),
            Self::Unreachable { kind, target } => write!(
                formatter,
                "{kind:?} target ({}, {}) is unreachable",
                target.x(),
                target.y()
            ),
            Self::Interrupted { id } => {
                write!(formatter, "action of person {} was interrupted", id.get())
            }
            Self::InvalidTransition { id } => {
                write!(formatter, "invalid transition for person {}", id.get())
            }
            Self::TimeOverflow { id } => {
                write!(formatter, "time overflow for person {}", id.get())
            }
            Self::EventLogExhausted => formatter.write_str("event identity space is exhausted"),
            Self::Schedule { source } => write!(formatter, "scheduler failure: {source}"),
        }
    }
}

impl std::error::Error for ActionError {}

impl From<SchedulerError> for ActionError {
    fn from(source: SchedulerError) -> Self {
        Self::Schedule { source }
    }
}

/// Borrowed world state for execution: the person runtime, the terrain map,
/// and the activity-site collection. The executor mutates person
/// location/needs and the bounded work counter only; terrain is static.
pub struct ActionEnvironment<'a> {
    /// The person runtime being executed against.
    pub persons: &'a mut PersonRuntime,
    /// The static local world.
    pub map: &'a WorldMap,
    /// The static activity sites (work counter is the only mutable part).
    pub sites: &'a mut ActivitySites,
}

/// Internal execution phase of a live record.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Phase {
    Moving,
    Active,
}

/// Scheduler payloads. Token identity is verified against the person's
/// current record on every pop, so stale or double delivery cannot execute.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum DueWork {
    /// Continue the active action (next movement step or completion).
    Continue { person: EntityId },
    /// Surface a retry decision after a blocked/failed recovery delay.
    Retry { person: EntityId },
    /// Surface a critical-need boundary check.
    CriticalCheck { person: EntityId },
}

struct ExecutionRecord {
    action: ActionKind,
    target: Option<LocalCoord>,
    phase: Phase,
    path: Option<Path>,
    next_index: usize,
    started_at: SimInstant,
}

/// Per-person execution bookkeeping: at most one active record and at most
/// two live scheduler tokens (continuation/retry + critical check).
struct PersonExecution {
    record: Option<ExecutionRecord>,
    continue_token: Option<ScheduleToken>,
    check_token: Option<ScheduleToken>,
    last_needs_at: SimInstant,
    /// Latest successful action-boundary commit. This is independent of the
    /// lazy Needs materialization baseline.
    last_commit_at: SimInstant,
}

impl PersonExecution {
    const fn new(last_needs_at: SimInstant) -> Self {
        Self {
            record: None,
            continue_token: None,
            check_token: None,
            last_needs_at,
            last_commit_at: last_needs_at,
        }
    }
}

const fn state_of(record: &ExecutionRecord) -> ActionState {
    match (record.action, record.phase) {
        // A standalone Move completes at arrival and never enters the active
        // phase, so the (Move, Active) arm is unreachable by construction.
        (ActionKind::Idle, _) | (ActionKind::Move, Phase::Active) => ActionState::Idle,
        (_, Phase::Moving) => ActionState::Moving {
            action: record.action,
        },
        (ActionKind::Eat, Phase::Active) => ActionState::Eating,
        (ActionKind::Sleep, Phase::Active) => ActionState::Sleeping,
        (ActionKind::Work, Phase::Active) => ActionState::Working,
    }
}

const fn required_site_kind(kind: ActionKind) -> Option<SiteKind> {
    match kind {
        ActionKind::Eat => Some(SiteKind::Meal),
        ActionKind::Sleep => Some(SiteKind::Rest),
        ActionKind::Work => Some(SiteKind::Work),
        ActionKind::Move | ActionKind::Idle => None,
    }
}

const fn kind_name(kind: ActionKind) -> &'static str {
    match kind {
        ActionKind::Move => "Move",
        ActionKind::Eat => "Eat",
        ActionKind::Sleep => "Sleep",
        ActionKind::Work => "Work",
        ActionKind::Idle => "Idle",
    }
}

/// Seconds until the first drive reaches `CRITICAL_PRESSURE`; the caller
/// guarantees the needs are not already critical. Exact integer ceiling
/// division on the committed raw values and rates.
fn seconds_until_critical(needs: Needs) -> i64 {
    let critical_raw =
        palimpsest_sim_ai::NEED_MAX * CRITICAL_PRESSURE / palimpsest_sim_ai::PRESSURE_MAX;
    let hunger = needs.hunger().raw();
    let fatigue = needs.fatigue().raw();
    debug_assert!(hunger < critical_raw && fatigue < critical_raw);
    let hunger_seconds =
        (critical_raw - hunger + HUNGER_RATE_PER_SECOND - 1) / HUNGER_RATE_PER_SECOND;
    let fatigue_seconds =
        (critical_raw - fatigue + FATIGUE_RATE_PER_SECOND - 1) / FATIGUE_RATE_PER_SECOND;
    hunger_seconds.min(fatigue_seconds)
}

fn critical_due(
    person: EntityId,
    now: SimInstant,
    needs: Needs,
    config: &ActionConfig,
) -> Result<SimInstant, ActionError> {
    let delay = if needs.is_critical() {
        config.critical_recheck_delay
    } else {
        SimDuration::from_seconds(seconds_until_critical(needs))
            .ok_or(ActionError::TimeOverflow { id: person })?
    };
    now.checked_add(delay)
        .ok_or(ActionError::TimeOverflow { id: person })
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize)]
/// Attempted path queries; observational counters, not committed truth.
pub struct PathQueryCounts {
    /// Queries used by candidate enumeration and validation/scoring.
    pub candidate_queries: u64,
    /// Queries used to construct an execution path, including failures.
    pub execution_queries: u64,
}

/// The Phase 1 action executor (CHRON-027, ADR-0021).
///
/// Deterministic and headless: no wall-clock, thread, float, or
/// unordered-iteration dependence enters truth. Runtime state is never
/// serialized; `EntityId` is the only identity (ADR-0002/0011).
pub struct ActionRuntime {
    config: ActionConfig,
    scheduler: Scheduler<DueWork>,
    executions: HashMap<EntityId, PersonExecution>,
    events: VecDeque<EventRecord>,
    events_rotated: u64,
    next_event_raw: u64,
    events_total: u64,
    events_digest: u64,
    stats: ActionStats,
    candidate_path_queries: std::cell::Cell<u64>,
    execution_path_queries: u64,
}

impl ActionRuntime {
    /// Creates an executor with the given configuration.
    #[must_use]
    pub fn new(config: ActionConfig) -> Self {
        Self {
            config,
            scheduler: Scheduler::new(),
            executions: HashMap::new(),
            events: VecDeque::new(),
            events_rotated: 0,
            next_event_raw: 1,
            events_total: 0,
            events_digest: EVENT_DIGEST_OFFSET,
            stats: ActionStats::default(),
            candidate_path_queries: std::cell::Cell::new(0),
            execution_path_queries: 0,
        }
    }

    /// Successful scheduler operations, separate from simulation truth.
    #[must_use]
    pub fn scheduler_counters(&self) -> palimpsest_sim_scheduler::SchedulerCounters {
        self.scheduler.counters()
    }

    /// Attempted path queries, independent of committed action statistics.
    #[must_use]
    pub fn path_query_counts(&self) -> PathQueryCounts {
        PathQueryCounts {
            candidate_queries: self.candidate_path_queries.get(),
            execution_queries: self.execution_path_queries,
        }
    }

    /// The configuration in use.
    #[must_use]
    pub const fn config(&self) -> &ActionConfig {
        &self.config
    }

    /// Starts executing `action` for `person` at `now`.
    ///
    /// The candidate must come from a selection over the same live context;
    /// imported diagnostic values are never accepted as commands (ADR-0019).
    /// Start rechecks preconditions against simulation truth: the site kind
    /// for Eat/Sleep/Work and pathfinding reachability for every targeted
    /// action. Needs growth up to `now` is materialized so the critical-need
    /// schedule is computed from exact values.
    ///
    /// # Errors
    ///
    /// - [`ActionError::UnknownPerson`] — no live person with this identity.
    /// - [`ActionError::AlreadyExecuting`] — the person has an active record;
    ///   nothing changes.
    /// - [`ActionError::InvalidTarget`] — defensive kind/target mismatch.
    /// - [`ActionError::Blocked`] — the target site is missing or of the
    ///   wrong kind; nothing changes.
    /// - [`ActionError::Unreachable`] — no path to the target under the
    ///   configured budget; nothing changes.
    /// - [`ActionError::TimeOverflow`], [`ActionError::Schedule`] — the
    ///   follow-up instant or token could not be allocated.
    ///
    /// # Panics
    ///
    /// Never in practice: the per-person entry is inserted earlier in the
    /// same call, so the follow-up lookup always succeeds.
    #[allow(clippy::too_many_lines)] // Keep the atomic validation/commit boundary together.
    pub fn start(
        &mut self,
        person: EntityId,
        action: ActionCandidate,
        env: &mut ActionEnvironment<'_>,
        now: SimInstant,
    ) -> Result<Transition, ActionError> {
        let location = env
            .persons
            .location(person)
            .ok_or(ActionError::UnknownPerson { id: person })?;
        let kind = action.kind();
        let target = action.target();

        self.validate_start_time(person, now)?;
        let stored_needs = env
            .persons
            .needs(person)
            .ok_or(ActionError::UnknownPerson { id: person })?;
        let prepared_needs = self.projected_needs(person, stored_needs, now)?;
        let prepared_check_due = critical_due(person, now, prepared_needs, &self.config)?;

        // Preflight (no mutation): candidate shape, target/path, and the
        // follow-up due instants that the commit will schedule.
        let (to, record, continue_due) = if kind == ActionKind::Idle {
            let due = now
                .checked_add(self.config.idle_wait)
                .ok_or(ActionError::TimeOverflow { id: person })?;
            (
                ActionState::Idle,
                ExecutionRecord {
                    action: ActionKind::Idle,
                    target: None,
                    phase: Phase::Active,
                    path: None,
                    next_index: 0,
                    started_at: now,
                },
                due,
            )
        } else {
            let target = target.ok_or(ActionError::InvalidTarget { kind })?;
            if let Some(required) = required_site_kind(kind) {
                let available = env
                    .sites
                    .site_at(target)
                    .is_some_and(|site| site.kind() == required);
                if !available {
                    return Err(ActionError::Blocked { kind, target });
                }
            }
            self.execution_path_queries = self.execution_path_queries.saturating_add(1);
            let path = find_path(
                env.map.local(),
                (location.x(), location.y()),
                (target.x(), target.y()),
                TerrainKind::is_walkable,
                self.config.path,
            )
            .map_err(|_| ActionError::Unreachable { kind, target })?;
            let due = now
                .checked_add(self.config.move_step)
                .ok_or(ActionError::TimeOverflow { id: person })?;
            (
                ActionState::Moving { action: kind },
                ExecutionRecord {
                    action: kind,
                    target: Some(target),
                    phase: Phase::Moving,
                    path: Some(path),
                    next_index: 1,
                    started_at: now,
                },
                due,
            )
        };
        // One continuation token and one critical-check token are scheduled.
        self.scheduler.check_schedule_capacity(2)?;

        // Commit. Any pending retry superseded by this fresh decision is
        // cancelled only now, after every rejection preflight has passed.
        {
            let exec = self
                .executions
                .entry(person)
                .or_insert_with(|| PersonExecution::new(SimInstant::EPOCH));
            if let Some(token) = exec.continue_token.take() {
                self.scheduler.cancel(token);
            }
        }
        self.commit_needs(person, now, prepared_needs, env)?;
        let token = self
            .scheduler
            .schedule_at(continue_due, DueWork::Continue { person })?;
        let exec = self
            .executions
            .get_mut(&person)
            .expect("entry inserted above");
        exec.record = Some(record);
        exec.continue_token = Some(token);
        self.schedule_critical_check_at(person, prepared_check_due)?;
        self.executions
            .get_mut(&person)
            .expect("entry inserted above")
            .last_commit_at = now;
        self.stats.started = self.stats.started.saturating_add(1);
        Ok(Transition {
            person,
            from: ActionState::Idle,
            to,
            action: kind,
            target,
            at: now,
            reason: TransitionReason::Started,
            location,
        })
    }

    fn validate_start_time(&self, person: EntityId, now: SimInstant) -> Result<(), ActionError> {
        if let Some(exec) = self.executions.get(&person) {
            if exec.record.is_some() {
                return Err(ActionError::AlreadyExecuting { id: person });
            }
            if now < exec.last_needs_at || now < exec.last_commit_at {
                return Err(ActionError::TimeOverflow { id: person });
            }
        } else if now < SimInstant::EPOCH {
            return Err(ActionError::TimeOverflow { id: person });
        }
        Ok(())
    }

    /// Processes every due item at or before `now` in due-time/FIFO order and
    /// returns the committed transitions plus surfaced decision requests.
    ///
    /// Each item commits **at its own due instant**, not at `now`: a single
    /// long advance is exactly equivalent to stepping through each due
    /// instant separately (P1-REMAINING D2 segmentation equivalence). Each
    /// pop commits atomically; follow-up work is always scheduled strictly
    /// after its own due instant, so the drain terminates.
    ///
    /// # Errors
    ///
    /// Returns [`ActionError`] when a follow-up schedule or time computation
    /// fails; already-committed transitions are not rolled back.
    pub fn advance(
        &mut self,
        now: SimInstant,
        env: &mut ActionEnvironment<'_>,
    ) -> Result<AdvanceOutcome, ActionError> {
        let mut outcome = AdvanceOutcome::default();
        while let Some(item) = self.scheduler.pop_due(now) {
            let token = item.token();
            let due = item.due();
            match item.into_payload() {
                DueWork::Continue { person } => {
                    self.on_continue(person, token, due, env, &mut outcome)?;
                }
                DueWork::Retry { person } => self.on_retry(person, token, due, &mut outcome),
                DueWork::CriticalCheck { person } => {
                    self.on_critical_check(person, token, due, env, &mut outcome)?;
                }
            }
        }
        Ok(outcome)
    }

    /// Cancels the person's active action: needs are materialized without any
    /// completion reward, both live tokens are retired (the critical check is
    /// rescheduled from the committed values), and one atomic transition to
    /// `Idle` is committed with the corresponding outcome event.
    ///
    /// # Errors
    ///
    /// Returns [`ActionError::UnknownPerson`] for a non-live identity and
    /// [`ActionError::InvalidTransition`] when the person has no active
    /// action; nothing changes in either case.
    ///
    /// # Panics
    ///
    /// Never in practice: the preflight above verified the person is tracked,
    /// so the follow-up `executions` lookup in the commit block always
    /// succeeds.
    pub fn cancel(
        &mut self,
        person: EntityId,
        reason: CancelReason,
        now: SimInstant,
        env: &mut ActionEnvironment<'_>,
    ) -> Result<Transition, ActionError> {
        if env.persons.location(person).is_none() {
            return Err(ActionError::UnknownPerson { id: person });
        }
        // Preflight (no mutation): an active action exists, time is not
        // reversed, the outcome event id is available, and the follow-up
        // critical-check reschedule (one token) fits.
        let (from, kind, target) = {
            let exec = self
                .executions
                .get(&person)
                .ok_or(ActionError::InvalidTransition { id: person })?;
            let record = exec
                .record
                .as_ref()
                .ok_or(ActionError::InvalidTransition { id: person })?;
            if now < exec.last_needs_at {
                return Err(ActionError::TimeOverflow { id: person });
            }
            if now < exec.last_commit_at {
                return Err(ActionError::TimeOverflow { id: person });
            }
            (state_of(record), record.action, record.target)
        };
        let stored_needs = env
            .persons
            .needs(person)
            .ok_or(ActionError::UnknownPerson { id: person })?;
        let prepared_needs = self.projected_needs(person, stored_needs, now)?;
        let prepared_check_due = critical_due(person, now, prepared_needs, &self.config)?;
        if self.next_event_raw == 0 {
            return Err(ActionError::EventLogExhausted);
        }
        self.scheduler.check_schedule_capacity(1)?;

        // Commit from here on.
        {
            let exec = self
                .executions
                .get_mut(&person)
                .expect("preflight verified the person");
            if let Some(token) = exec.continue_token.take() {
                self.scheduler.cancel(token);
            }
            exec.record = None;
        }
        self.commit_needs(person, now, prepared_needs, env)?;
        let (transition_reason, event_type) = match reason {
            CancelReason::Interrupted => (TransitionReason::Interrupted, "action.interrupted"),
            CancelReason::External => (TransitionReason::Cancelled, "action.cancelled"),
        };
        match reason {
            CancelReason::Interrupted => {
                self.stats.interrupted = self.stats.interrupted.saturating_add(1);
            }
            CancelReason::External => {
                self.stats.cancelled = self.stats.cancelled.saturating_add(1);
            }
        }
        self.push_event(person, now, event_type, kind, target, None)?;
        self.schedule_critical_check_at(person, prepared_check_due)?;
        self.executions
            .get_mut(&person)
            .expect("preflight verified the person")
            .last_commit_at = now;
        let location = env
            .persons
            .location(person)
            .ok_or(ActionError::UnknownPerson { id: person })?;
        Ok(Transition {
            person,
            from,
            to: ActionState::Idle,
            action: kind,
            target,
            at: now,
            reason: transition_reason,
            location,
        })
    }

    /// The person's current observable state; `None` when the identity is not
    /// tracked by this executor. An Idle wait in progress reports
    /// [`ActionState::Idle`].
    #[must_use]
    pub fn current(&self, person: EntityId) -> Option<ActionState> {
        self.executions
            .get(&person)
            .and_then(|exec| exec.record.as_ref().map(state_of))
    }

    /// The active action kind and target; `None` when the person is free.
    /// `Some((ActionKind::Idle, None))` means an Idle wait is executing.
    #[must_use]
    pub fn current_action(&self, person: EntityId) -> Option<(ActionKind, Option<LocalCoord>)> {
        self.executions
            .get(&person)
            .and_then(|exec| exec.record.as_ref())
            .map(|record| (record.action, record.target))
    }

    /// The earliest due instant across all scheduled work.
    pub fn next_due(&mut self) -> Option<SimInstant> {
        self.scheduler.next_due()
    }

    /// Drains the bounded outcome-event buffer in insertion order.
    pub fn drain_events(&mut self) -> Vec<EventRecord> {
        self.events.drain(..).collect()
    }

    /// The monotonic execution counters.
    #[must_use]
    pub const fn stats(&self) -> ActionStats {
        self.stats
    }

    /// Read-only runtime health metrics (diagnostics only; iteration here
    /// never feeds simulation truth).
    #[must_use]
    pub fn metrics(&self) -> ActionRuntimeMetrics {
        let mut live_actions = 0_usize;
        let mut pending_retries = 0_usize;
        let mut live_checks = 0_usize;
        for exec in self.executions.values() {
            if exec.record.is_some() {
                live_actions += 1;
            } else if exec.continue_token.is_some() {
                pending_retries += 1;
            }
            if exec.check_token.is_some() {
                live_checks += 1;
            }
        }
        ActionRuntimeMetrics {
            scheduler: self.scheduler.metrics(),
            live_actions,
            pending_retries,
            live_checks,
            events_buffered: self.events.len(),
            events_rotated: self.events_rotated,
            events_total: self.events_total,
            events_digest: self.events_digest,
        }
    }

    /// Total high-level outcome events ever committed by this runtime
    /// (independent of buffer retention and drain frequency).
    #[must_use]
    pub const fn events_total(&self) -> u64 {
        self.events_total
    }

    /// The cumulative FNV-1a-64 stream digest of every committed event.
    #[must_use]
    pub const fn events_digest(&self) -> u64 {
        self.events_digest
    }

    /// Outcome events dropped by this runtime's retention buffer.
    #[must_use]
    pub const fn events_rotated(&self) -> u64 {
        self.events_rotated
    }

    /// Computes the person's Needs projected to `now` from the authoritative
    /// materialization baseline, without writing back or scheduling work.
    ///
    /// `stored` is the last materialized `Needs`; `now` must be at or after
    /// the person's base instant, otherwise a reversal/overflow is reported.
    /// This is the read-only view used by the kernel's `person`/`persons`
    /// accesses and by the selection context (ADR-0024 D4): it never advances
    /// a second time on the next `materialize`.
    ///
    /// # Errors
    ///
    /// Returns [`ActionError::TimeOverflow`] when `now` is earlier than the
    /// base instant.
    pub fn projected_needs(
        &self,
        person: EntityId,
        stored: Needs,
        now: SimInstant,
    ) -> Result<Needs, ActionError> {
        let last = self
            .executions
            .get(&person)
            .map_or(SimInstant::EPOCH, |exec| exec.last_needs_at);
        if now < last {
            return Err(ActionError::TimeOverflow { id: person });
        }
        let elapsed = now
            .duration_since(last)
            .ok_or(ActionError::TimeOverflow { id: person })?;
        Ok(stored.advance(elapsed))
    }

    /// Commits needs growth up to `now` exactly once per person. Every
    /// transition boundary materializes, so decision-time reads of
    /// `PersonRuntime::needs` are exact at decision instants.
    fn materialize(
        &mut self,
        person: EntityId,
        now: SimInstant,
        env: &mut ActionEnvironment<'_>,
    ) -> Result<(), ActionError> {
        let last = self
            .executions
            .entry(person)
            .or_insert_with(|| PersonExecution::new(SimInstant::EPOCH))
            .last_needs_at;
        if now == last {
            return Ok(());
        }
        let elapsed = now
            .duration_since(last)
            .ok_or(ActionError::TimeOverflow { id: person })?;
        let needs = env
            .persons
            .needs(person)
            .ok_or(ActionError::UnknownPerson { id: person })?;
        env.persons
            .set_needs(person, needs.advance(elapsed))
            .map_err(|_| ActionError::UnknownPerson { id: person })?;
        self.executions
            .get_mut(&person)
            .expect("entry inserted above")
            .last_needs_at = now;
        Ok(())
    }

    /// (Re)schedules the person's critical-need boundary check from the
    /// committed needs: the exact crossing instant, or the positive recheck
    /// delay while already critical (ADR-0021 §4).
    fn schedule_critical_check(
        &mut self,
        person: EntityId,
        now: SimInstant,
        env: &mut ActionEnvironment<'_>,
    ) -> Result<(), ActionError> {
        let needs = env
            .persons
            .needs(person)
            .ok_or(ActionError::UnknownPerson { id: person })?;
        let due = critical_due(person, now, needs, &self.config)?;
        self.schedule_critical_check_at(person, due)
    }

    fn schedule_critical_check_at(
        &mut self,
        person: EntityId,
        due: SimInstant,
    ) -> Result<(), ActionError> {
        let exec = self
            .executions
            .get_mut(&person)
            .ok_or(ActionError::UnknownPerson { id: person })?;
        if let Some(old) = exec.check_token.take() {
            self.scheduler.cancel(old);
        }
        let token = self
            .scheduler
            .schedule_at(due, DueWork::CriticalCheck { person })?;
        exec.check_token = Some(token);
        Ok(())
    }

    fn commit_needs(
        &mut self,
        person: EntityId,
        now: SimInstant,
        needs: Needs,
        env: &mut ActionEnvironment<'_>,
    ) -> Result<(), ActionError> {
        env.persons
            .set_needs(person, needs)
            .map_err(|_| ActionError::UnknownPerson { id: person })?;
        self.executions
            .get_mut(&person)
            .ok_or(ActionError::UnknownPerson { id: person })?
            .last_needs_at = now;
        Ok(())
    }

    fn schedule_continue(
        &mut self,
        person: EntityId,
        due: SimInstant,
    ) -> Result<ScheduleToken, ActionError> {
        let token = self
            .scheduler
            .schedule_at(due, DueWork::Continue { person })?;
        self.executions
            .get_mut(&person)
            .ok_or(ActionError::UnknownPerson { id: person })?
            .continue_token = Some(token);
        Ok(token)
    }

    fn on_continue(
        &mut self,
        person: EntityId,
        token: ScheduleToken,
        now: SimInstant,
        env: &mut ActionEnvironment<'_>,
        outcome: &mut AdvanceOutcome,
    ) -> Result<(), ActionError> {
        let Some(exec) = self.executions.get(&person) else {
            return Ok(());
        };
        // Token correspondence: stale or double delivery cannot execute.
        if exec.continue_token != Some(token) || exec.record.is_none() {
            return Ok(());
        }
        let phase = exec.record.as_ref().expect("checked above").phase;
        match phase {
            Phase::Moving => self.step_movement(person, now, env, outcome),
            Phase::Active => self.complete_action(person, now, env, outcome),
        }
    }

    fn on_retry(
        &mut self,
        person: EntityId,
        token: ScheduleToken,
        now: SimInstant,
        outcome: &mut AdvanceOutcome,
    ) {
        let Some(exec) = self.executions.get_mut(&person) else {
            return;
        };
        if exec.continue_token != Some(token) || exec.record.is_some() {
            return;
        }
        exec.continue_token = None;
        exec.last_commit_at = now;
        outcome.decision_requests.push(DecisionRequest {
            person,
            reason: DecisionReason::Retry,
            at: now,
        });
    }

    fn on_critical_check(
        &mut self,
        person: EntityId,
        token: ScheduleToken,
        now: SimInstant,
        env: &mut ActionEnvironment<'_>,
        outcome: &mut AdvanceOutcome,
    ) -> Result<(), ActionError> {
        let Some(exec) = self.executions.get_mut(&person) else {
            return Ok(());
        };
        if exec.check_token != Some(token) {
            return Ok(());
        }
        exec.check_token = None;
        self.materialize(person, now, env)?;
        self.executions
            .get_mut(&person)
            .expect("tracked check")
            .last_commit_at = now;
        outcome.decision_requests.push(DecisionRequest {
            person,
            reason: DecisionReason::CriticalBoundary,
            at: now,
        });
        self.schedule_critical_check(person, now, env)
    }

    /// Advances one movement cell per pop; the path's start cell is never
    /// re-walked (execution begins at index 1). Arrival commits the movement
    /// phase completion and dispatches per action kind.
    fn step_movement(
        &mut self,
        person: EntityId,
        now: SimInstant,
        env: &mut ActionEnvironment<'_>,
        outcome: &mut AdvanceOutcome,
    ) -> Result<(), ActionError> {
        let (action, target, next_index, path_len, next_coord) = {
            let exec = self
                .executions
                .get(&person)
                .ok_or(ActionError::UnknownPerson { id: person })?;
            let record = exec.record.as_ref().expect("live record checked");
            let path = record.path.as_ref().expect("moving record has a path");
            let next_coord = if record.next_index < path.len() {
                Some(path.coords()[record.next_index])
            } else {
                None
            };
            (
                record.action,
                record.target,
                record.next_index,
                path.len(),
                next_coord,
            )
        };
        let arrival_index = path_len - 1;
        if let Some(coord) = next_coord {
            // Compute the next due instant before changing location or stats.
            let next_due = if next_index == arrival_index {
                None
            } else {
                Some(
                    now.checked_add(self.config.move_step)
                        .ok_or(ActionError::TimeOverflow { id: person })?,
                )
            };
            env.persons
                .set_location(person, coord)
                .map_err(|_| ActionError::UnknownPerson { id: person })?;
            self.stats.steps = self.stats.steps.saturating_add(1);
            let arrived = next_index == arrival_index;
            let location = coord;
            let from = ActionState::Moving { action };
            if arrived {
                self.stats.movement_completions = self.stats.movement_completions.saturating_add(1);
                outcome.transitions.push(Transition {
                    person,
                    from,
                    to: from,
                    action,
                    target,
                    at: now,
                    reason: TransitionReason::Arrived,
                    location,
                });
                self.arrive(person, action, target, now, env, outcome)
            } else {
                self.executions
                    .get_mut(&person)
                    .expect("record checked")
                    .record
                    .as_mut()
                    .expect("record checked")
                    .next_index += 1;
                self.schedule_continue(person, next_due.expect("non-arrival has due"))?;
                self.executions
                    .get_mut(&person)
                    .expect("record checked")
                    .last_commit_at = now;
                outcome.transitions.push(Transition {
                    person,
                    from,
                    to: from,
                    action,
                    target,
                    at: now,
                    reason: TransitionReason::Step,
                    location,
                });
                Ok(())
            }
        } else {
            // Zero-distance path (start == target): the single arrival tick
            // keeps every action at least one second long (ADR-0021 §2).
            self.stats.movement_completions = self.stats.movement_completions.saturating_add(1);
            self.arrive(person, action, target, now, env, outcome)
        }
    }

    /// Handles arrival at the target: a standalone Move completes; Eat/Sleep/
    /// Work recheck the site against simulation truth and enter the activity
    /// phase, or recover as blocked (ADR-0019's executor boundary).
    fn arrive(
        &mut self,
        person: EntityId,
        action: ActionKind,
        target: Option<LocalCoord>,
        now: SimInstant,
        env: &mut ActionEnvironment<'_>,
        outcome: &mut AdvanceOutcome,
    ) -> Result<(), ActionError> {
        if action == ActionKind::Move {
            return self.complete_action(person, now, env, outcome);
        }
        let Some(target) = target else {
            return self.recover(person, RecoveryKind::Failed, now, env, outcome);
        };
        let required = required_site_kind(action);
        let available = required.is_some_and(|kind| {
            env.sites
                .site_at(target)
                .is_some_and(|site| site.kind() == kind)
        });
        if !available {
            return self.recover(person, RecoveryKind::Blocked, now, env, outcome);
        }
        let duration = self
            .config
            .activity_duration(action)
            .ok_or(ActionError::InvalidTransition { id: person })?;
        let due = now
            .checked_add(duration)
            .ok_or(ActionError::TimeOverflow { id: person })?;
        let from = {
            let exec = self
                .executions
                .get_mut(&person)
                .ok_or(ActionError::UnknownPerson { id: person })?;
            let record = exec.record.as_mut().expect("live record checked");
            let from = state_of(record);
            record.phase = Phase::Active;
            record.path = None;
            from
        };
        self.schedule_continue(person, due)?;
        self.executions
            .get_mut(&person)
            .expect("record present")
            .last_commit_at = now;
        let location = env
            .persons
            .location(person)
            .ok_or(ActionError::UnknownPerson { id: person })?;
        outcome.transitions.push(Transition {
            person,
            from,
            to: state_of(
                self.executions
                    .get(&person)
                    .and_then(|exec| exec.record.as_ref())
                    .expect("record present"),
            ),
            action,
            target: Some(target),
            at: now,
            reason: TransitionReason::Arrived,
            location,
        });
        Ok(())
    }

    /// Commits a successful completion: materialize needs growth to `now`,
    /// apply the Eat/Sleep relief or the bounded Work counter, emit the
    /// outcome event, and request the next decision. Interrupted/blocked/
    /// failed actions never reach this path (ADR-0021 §2).
    fn complete_action(
        &mut self,
        person: EntityId,
        now: SimInstant,
        env: &mut ActionEnvironment<'_>,
        outcome: &mut AdvanceOutcome,
    ) -> Result<(), ActionError> {
        let (from, action, target, started_at) = {
            let exec = self
                .executions
                .get(&person)
                .ok_or(ActionError::UnknownPerson { id: person })?;
            let record = exec.record.as_ref().expect("live record checked");
            (
                state_of(record),
                record.action,
                record.target,
                record.started_at,
            )
        };
        self.materialize(person, now, env)?;
        if action == ActionKind::Work {
            let target = target.ok_or(ActionError::InvalidTarget { kind: action })?;
            if env.sites.record_work(target).is_err() {
                return self.recover(person, RecoveryKind::Failed, now, env, outcome);
            }
        }
        if matches!(action, ActionKind::Eat | ActionKind::Sleep) {
            let needs = env
                .persons
                .needs(person)
                .ok_or(ActionError::UnknownPerson { id: person })?;
            let relieved = match action {
                ActionKind::Eat => needs.eat(EAT_RELIEF).0,
                ActionKind::Sleep => needs.rest(REST_RELIEF).0,
                _ => needs,
            };
            env.persons
                .set_needs(person, relieved)
                .map_err(|_| ActionError::UnknownPerson { id: person })?;
        }
        {
            let exec = self
                .executions
                .get_mut(&person)
                .expect("record checked above");
            exec.record = None;
            exec.continue_token = None;
            exec.last_commit_at = now;
        }
        match action {
            ActionKind::Move => {
                self.stats.move_completions = self.stats.move_completions.saturating_add(1);
            }
            ActionKind::Eat => {
                self.stats.eat_completions = self.stats.eat_completions.saturating_add(1);
            }
            ActionKind::Sleep => {
                self.stats.sleep_completions = self.stats.sleep_completions.saturating_add(1);
            }
            ActionKind::Work => {
                self.stats.work_completions = self.stats.work_completions.saturating_add(1);
            }
            ActionKind::Idle => {
                self.stats.idle_completions = self.stats.idle_completions.saturating_add(1);
            }
        }
        // Idle completions are pacing artifacts: counted, not emitted.
        if action != ActionKind::Idle {
            let duration = now.duration_since(started_at);
            self.push_event(person, now, "action.completed", action, target, duration)?;
        }
        self.schedule_critical_check(person, now, env)?;
        let location = env
            .persons
            .location(person)
            .ok_or(ActionError::UnknownPerson { id: person })?;
        outcome.transitions.push(Transition {
            person,
            from,
            to: ActionState::Idle,
            action,
            target,
            at: now,
            reason: TransitionReason::Completed,
            location,
        });
        outcome.decision_requests.push(DecisionRequest {
            person,
            reason: DecisionReason::Completed,
            at: now,
        });
        Ok(())
    }

    /// Blocked/failed recovery: cancel the record's live continuation, commit
    /// one atomic transition to Idle, emit the outcome event, and schedule the
    /// retry decision at `now + retry_delay` (never the same instant).
    fn recover(
        &mut self,
        person: EntityId,
        kind: RecoveryKind,
        now: SimInstant,
        env: &mut ActionEnvironment<'_>,
        outcome: &mut AdvanceOutcome,
    ) -> Result<(), ActionError> {
        // Compute every fallible follow-up instant before retiring the live
        // record. Recovery is an internal boundary, but must not partially
        // mutate on a checked-time failure.
        let retry_due = now
            .checked_add(self.config.retry_delay)
            .ok_or(ActionError::TimeOverflow { id: person })?;
        let (from, action, target) = {
            let exec = self
                .executions
                .get_mut(&person)
                .ok_or(ActionError::UnknownPerson { id: person })?;
            let Some(record) = &exec.record else {
                return Err(ActionError::InvalidTransition { id: person });
            };
            let snapshot = (state_of(record), record.action, record.target);
            exec.record = None;
            exec.continue_token = None;
            snapshot
        };
        self.materialize(person, now, env)?;
        self.executions
            .get_mut(&person)
            .expect("record checked")
            .last_commit_at = now;
        let (reason, event_type) = match kind {
            RecoveryKind::Blocked => (TransitionReason::Blocked, "action.blocked"),
            RecoveryKind::Failed => (TransitionReason::Failed, "action.failed"),
        };
        match kind {
            RecoveryKind::Blocked => {
                self.stats.blocked = self.stats.blocked.saturating_add(1);
            }
            RecoveryKind::Failed => {
                self.stats.failed = self.stats.failed.saturating_add(1);
            }
        }
        self.push_event(person, now, event_type, action, target, None)?;
        let token = self
            .scheduler
            .schedule_at(retry_due, DueWork::Retry { person })?;
        self.executions
            .get_mut(&person)
            .expect("record checked above")
            .continue_token = Some(token);
        self.schedule_critical_check(person, now, env)?;
        let location = env
            .persons
            .location(person)
            .ok_or(ActionError::UnknownPerson { id: person })?;
        outcome.transitions.push(Transition {
            person,
            from,
            to: ActionState::Idle,
            action,
            target,
            at: now,
            reason,
            location,
        });
        Ok(())
    }

    /// Appends a validated schema-1 outcome event to the bounded buffer.
    fn push_event(
        &mut self,
        person: EntityId,
        at: SimInstant,
        event_type: &'static str,
        kind: ActionKind,
        target: Option<LocalCoord>,
        duration: Option<SimDuration>,
    ) -> Result<(), ActionError> {
        // A zero raw value is the exhaustion sentinel: EventId is non-zero.
        let Some(event_id) = EventId::new(self.next_event_raw) else {
            return Err(ActionError::EventLogExhausted);
        };
        self.next_event_raw = self.next_event_raw.checked_add(1).unwrap_or(0);
        let mut record =
            EventRecord::new(event_id, at, event_type).expect("event type constants are non-empty");
        record
            .add_actor(person)
            .expect("a fresh record has no actors to duplicate");
        record.insert_metadata("action_kind", Value::from(kind_name(kind)));
        if let Some(target) = target {
            record.insert_metadata("target_x", Value::from(target.x()));
            record.insert_metadata("target_y", Value::from(target.y()));
        }
        if let Some(duration) = duration {
            record.insert_metadata("duration_seconds", Value::from(duration.as_seconds()));
        }
        record
            .validate()
            .expect("constructed outcome records satisfy the schema invariants");
        // Count and fold into the stream digest BEFORE buffer retention
        // (ADR-0024 D5): total/digest are independent of drain frequency and
        // the bounded retention buffer.
        self.events_total = self.events_total.saturating_add(1);
        let body = serde_json::to_vec(&record).expect("a validated event serializes");
        let length = u64::try_from(body.len()).expect("event body length fits u64");
        let mut digest = self.events_digest;
        digest = fold_digest(digest, &length.to_le_bytes());
        digest = fold_digest(digest, &body);
        self.events_digest = digest;
        if self.events.len() == EVENT_BUFFER_CAPACITY {
            self.events.pop_front();
            self.events_rotated = self.events_rotated.saturating_add(1);
        }
        self.events.push_back(record);
        Ok(())
    }
}

impl Default for ActionRuntime {
    fn default() -> Self {
        Self::new(ActionConfig::default())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum RecoveryKind {
    Blocked,
    Failed,
}

/// Failures of the decision-driver composition helpers.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DecisionDriveError {
    /// The executor rejected an operation.
    Action(ActionError),
    /// Selection failed on the live candidate set.
    Selection(DecisionError),
}

impl Display for DecisionDriveError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Action(source) => write!(formatter, "action execution failure: {source}"),
            Self::Selection(source) => write!(formatter, "selection failure: {source}"),
        }
    }
}

impl std::error::Error for DecisionDriveError {}

/// The outcome of resolving one decision request: the fresh selection (its
/// full `DecisionTrace` stays with the caller) and the committed transitions.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DecisionResolution {
    selection: Selection,
    transitions: Vec<Transition>,
}

impl DecisionResolution {
    /// The selection produced from the live context.
    #[must_use]
    pub const fn selection(&self) -> &Selection {
        &self.selection
    }

    /// Transitions committed while applying the selection (empty when a
    /// critical check re-elected the current action).
    #[must_use]
    pub fn transitions(&self) -> &[Transition] {
        &self.transitions
    }
}

/// One resolved decision paired with the person it applies to.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PersonResolution {
    person: EntityId,
    resolution: DecisionResolution,
}

impl PersonResolution {
    /// The person whose decision was resolved.
    #[must_use]
    pub const fn person(&self) -> EntityId {
        self.person
    }

    /// The resolved selection and its committed transitions.
    #[must_use]
    pub const fn resolution(&self) -> &DecisionResolution {
        &self.resolution
    }
}

/// Resolves a batch of surfaced [`DecisionRequest`] values, merging any that
/// belong to the same person at the same instant (ADR-0024 D2).
///
/// Within one due instant a person may surface both a `Completed`/`Retry`
/// request and a `CriticalBoundary` request. Merging them ensures that person
/// is selected exactly once: a `Completed`/`Retry` request triggers one fresh
/// selection and start, while a lone `CriticalBoundary` compares against the
/// current action and interrupts only when a different `(kind, target)` wins.
/// The selection reads the final Needs of that instant (including any relief
/// applied by a just-completed Eat/Sleep). Across persons the first-occurrence
/// order of the input requests is preserved (no `HashMap`/identity reordering);
/// requests for different persons or instants are never merged.
///
/// # Errors
///
/// Returns [`DecisionDriveError`] on the first selection/executor failure;
/// work committed before the failure is not rolled back.
pub fn resolve_decisions(
    runtime: &mut ActionRuntime,
    requests: &[DecisionRequest],
    env: &mut ActionEnvironment<'_>,
    weights: &Weights,
    spec: &PerturbationSpec,
) -> Result<Vec<PersonResolution>, DecisionDriveError> {
    let mut order: Vec<(EntityId, SimInstant)> = Vec::new();
    let mut seen = std::collections::HashSet::new();
    let mut requires_start: std::collections::HashMap<(EntityId, SimInstant), bool> =
        std::collections::HashMap::new();
    for request in requests {
        let key = (request.person(), request.at());
        if seen.insert(key) {
            order.push(key);
        }
        let needs_start = matches!(
            request.reason(),
            DecisionReason::Completed | DecisionReason::Retry
        );
        let entry = requires_start.entry(key).or_insert(false);
        *entry |= needs_start;
    }
    let mut resolutions = Vec::with_capacity(order.len());
    for (person, instant) in order {
        let selection = select_live(runtime, person, env, weights, spec, instant)?;
        let winner = selection.candidate();
        let mut transitions = Vec::new();
        if requires_start
            .get(&(person, instant))
            .copied()
            .unwrap_or(false)
        {
            transitions.push(
                runtime
                    .start(person, winner, env, instant)
                    .map_err(DecisionDriveError::Action)?,
            );
        } else {
            let current = runtime.current_action(person);
            if current != Some((winner.kind(), winner.target())) {
                if current.is_some() {
                    transitions.push(
                        runtime
                            .cancel(person, CancelReason::Interrupted, instant, env)
                            .map_err(DecisionDriveError::Action)?,
                    );
                }
                transitions.push(
                    runtime
                        .start(person, winner, env, instant)
                        .map_err(DecisionDriveError::Action)?,
                );
            }
        }
        resolutions.push(PersonResolution {
            person,
            resolution: DecisionResolution {
                selection,
                transitions,
            },
        });
    }
    Ok(resolutions)
}

/// Drives the executor to `target`: processes due work in due-time/FIFO order
/// and resolves every surfaced decision request with the given weights and
/// perturbation. The caller starts the roster's first actions beforehand
/// (e.g. with [`decide_and_start`] at the world epoch).
///
/// This is the CHRON-027 reference driver used by the closed-loop tests and
/// benchmarks; CHRON-028's kernel supersedes it. It uses the same merged
/// [`resolve_decisions`] batch helper as the kernel (ADR-0024 D2).
///
/// # Errors
///
/// Returns [`DecisionDriveError`] on the first executor/selection failure;
/// work committed before the failure is not rolled back.
pub fn run_until(
    runtime: &mut ActionRuntime,
    env: &mut ActionEnvironment<'_>,
    target: SimInstant,
    weights: &Weights,
    spec: &PerturbationSpec,
) -> Result<(), DecisionDriveError> {
    while let Some(next) = runtime.next_due() {
        if next > target {
            break;
        }
        let outcome = runtime
            .advance(next, env)
            .map_err(DecisionDriveError::Action)?;
        resolve_decisions(runtime, outcome.decision_requests(), env, weights, spec)?;
    }
    Ok(())
}

/// Runs one decision cycle for `person`: enumerate candidates from the live
/// context, select with the given weights/perturbation, and start the winner.
///
/// The executor consumes the selection produced from this same live context;
/// no imported diagnostic value is ever executed (ADR-0019).
///
/// # Errors
///
/// Returns [`DecisionDriveError::Selection`] when selection fails and
/// [`DecisionDriveError::Action`] when the executor rejects the start.
pub fn decide_and_start(
    runtime: &mut ActionRuntime,
    person: EntityId,
    env: &mut ActionEnvironment<'_>,
    weights: &Weights,
    spec: &PerturbationSpec,
    now: SimInstant,
) -> Result<DecisionResolution, DecisionDriveError> {
    let selection = select_live(runtime, person, env, weights, spec, now)?;
    let transition = runtime
        .start(person, selection.candidate(), env, now)
        .map_err(DecisionDriveError::Action)?;
    Ok(DecisionResolution {
        selection,
        transitions: vec![transition],
    })
}

/// Resolves one surfaced [`DecisionRequest`].
///
/// `Completed`/`Retry` start the freshly selected winner. `CriticalBoundary`
/// re-selects on the live context and interrupts only when a different
/// `(kind, target)` wins: the interrupt is a normal explainable selection,
/// never an emergency bypass (ADR-0014/0018, P1-REMAINING D1).
///
/// # Errors
///
/// Returns [`DecisionDriveError`] when selection or the executor fails.
pub fn resolve_decision(
    runtime: &mut ActionRuntime,
    request: &DecisionRequest,
    env: &mut ActionEnvironment<'_>,
    weights: &Weights,
    spec: &PerturbationSpec,
) -> Result<DecisionResolution, DecisionDriveError> {
    let person = request.person();
    let now = request.at();
    let selection = select_live(runtime, person, env, weights, spec, now)?;
    let winner = selection.candidate();
    let mut transitions = Vec::new();
    match request.reason() {
        DecisionReason::Completed | DecisionReason::Retry => {
            transitions.push(
                runtime
                    .start(person, winner, env, now)
                    .map_err(DecisionDriveError::Action)?,
            );
        }
        DecisionReason::CriticalBoundary => {
            let current = runtime.current_action(person);
            if current != Some((winner.kind(), winner.target())) {
                if current.is_some() {
                    transitions.push(
                        runtime
                            .cancel(person, CancelReason::Interrupted, now, env)
                            .map_err(DecisionDriveError::Action)?,
                    );
                }
                transitions.push(
                    runtime
                        .start(person, winner, env, now)
                        .map_err(DecisionDriveError::Action)?,
                );
            }
        }
    }
    Ok(DecisionResolution {
        selection,
        transitions,
    })
}

/// Enumerates and selects from the person's live context.
fn select_live(
    runtime: &ActionRuntime,
    person: EntityId,
    env: &ActionEnvironment<'_>,
    weights: &Weights,
    spec: &PerturbationSpec,
    now: SimInstant,
) -> Result<Selection, DecisionDriveError> {
    let location = env
        .persons
        .location(person)
        .ok_or(DecisionDriveError::Action(ActionError::UnknownPerson {
            id: person,
        }))?;
    let stored = env.persons.needs(person).ok_or(DecisionDriveError::Action(
        ActionError::UnknownPerson { id: person },
    ))?;
    // Select with Needs projected to the request instant, not the last
    // materialized value (ADR-0024 D4).
    let needs = runtime
        .projected_needs(person, stored, now)
        .map_err(DecisionDriveError::Action)?;
    let context = CandidateContext::new(
        location,
        needs,
        &*env.sites,
        env.map,
        runtime.config().path(),
    )
    .with_path_query_counter(&runtime.candidate_path_queries);
    let candidates = candidate_actions(&context);
    select_action(&candidates, &context, weights, spec).map_err(DecisionDriveError::Selection)
}

#[cfg(test)]
mod tests {
    use super::{
        ActionConfig, ActionEnvironment, ActionError, ActionRuntime, ActionState, ActionStats,
        CancelReason, DecisionReason, DecisionRequest, EVENT_BUFFER_CAPACITY, Transition,
        TransitionReason, decide_and_start, resolve_decision, run_until,
    };
    use palimpsest_sim_ai::{
        ActionCandidate, ActionKind, NeedValue, Needs, PerturbationSpec, Weights,
    };
    use palimpsest_sim_entity::{EntityId, EntityIdAllocator};
    use palimpsest_sim_time::{SimDuration, SimInstant};
    use palimpsest_sim_world::{
        ActivitySite, ActivitySites, LocalCoord, PathConfig, SiteKind, WorldGenConfig, WorldMap,
        WorldSeed,
    };

    use crate::person::PersonRuntime;

    /// Locked fixture seed shared with the ADR-0018 reference context.
    const FIXTURE_SEED: u64 = 25_025;

    fn seconds(value: i64) -> SimDuration {
        SimDuration::from_seconds(value).expect("non-negative duration")
    }

    fn at(value: i64) -> SimInstant {
        SimInstant::from_seconds(value)
    }

    fn coord(x: i32, y: i32) -> LocalCoord {
        LocalCoord::new(x, y).expect("test coordinate in bounds")
    }

    fn needs_with(hunger: i64, fatigue: i64) -> Needs {
        Needs::new(
            NeedValue::from_raw(hunger).expect("in range"),
            NeedValue::from_raw(fatigue).expect("in range"),
        )
    }

    fn default_map() -> WorldMap {
        WorldMap::generate(WorldSeed::new(FIXTURE_SEED), WorldGenConfig::default())
    }

    /// Origin of a fully walkable 3×3 block, guaranteed by the generator's
    /// spawn clearing (same fixture pattern as the sim-ai test suites).
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

    /// The ADR-0018 reference fixture: Meal +(2,0), Rest +(0,2), Work +(2,2).
    struct Fixture {
        map: WorldMap,
        sites: ActivitySites,
        origin: LocalCoord,
        persons: PersonRuntime,
        allocator: EntityIdAllocator,
    }

    impl Fixture {
        fn new() -> Self {
            let map = default_map();
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

        fn meal(&self) -> LocalCoord {
            coord(self.origin.x() + 2, self.origin.y())
        }

        fn rest(&self) -> LocalCoord {
            coord(self.origin.x(), self.origin.y() + 2)
        }

        fn work(&self) -> LocalCoord {
            coord(self.origin.x() + 2, self.origin.y() + 2)
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

    fn candidate(kind: ActionKind, target: Option<LocalCoord>) -> ActionCandidate {
        ActionCandidate::new(kind, target, 0).expect("valid test candidate")
    }

    #[test]
    fn move_action_steps_one_cell_per_second_and_completes() {
        let mut fixture = Fixture::new();
        let person = fixture.spawn();
        let meal = fixture.meal();
        let (origin_x, origin_y) = (fixture.origin.x(), fixture.origin.y());
        let mut runtime = ActionRuntime::default();
        let started = {
            let mut env = fixture.env();
            runtime
                .start(
                    person,
                    candidate(ActionKind::Move, Some(meal)),
                    &mut env,
                    at(0),
                )
                .expect("start move")
        };
        assert_eq!(started.from(), ActionState::Idle);
        assert_eq!(
            started.to(),
            ActionState::Moving {
                action: ActionKind::Move
            }
        );
        assert_eq!(started.reason(), TransitionReason::Started);
        assert_eq!(
            runtime.current(person),
            Some(ActionState::Moving {
                action: ActionKind::Move
            })
        );
        // Nothing is due at the start instant.
        let mut env = fixture.env();
        assert!(
            runtime
                .advance(at(0), &mut env)
                .expect("advance")
                .transitions()
                .is_empty()
        );
        // One cell per second: path length 3 means steps at t=1 and t=2.
        let first = runtime.advance(at(1), &mut env).expect("advance");
        assert_eq!(first.transitions().len(), 1);
        assert_eq!(first.transitions()[0].reason(), TransitionReason::Step);
        assert_eq!(
            first.transitions()[0].location(),
            coord(origin_x + 1, origin_y)
        );
        let second = runtime.advance(at(2), &mut env).expect("advance");
        let reasons: Vec<TransitionReason> = second
            .transitions()
            .iter()
            .map(Transition::reason)
            .collect();
        assert_eq!(
            reasons,
            vec![TransitionReason::Arrived, TransitionReason::Completed]
        );
        assert_eq!(second.decision_requests().len(), 1);
        assert_eq!(
            second.decision_requests()[0].reason(),
            DecisionReason::Completed
        );
        assert_eq!(second.decision_requests()[0].at(), at(2));
        let stats = runtime.stats();
        assert_eq!(stats.steps, 2);
        assert_eq!(stats.movement_completions, 1);
        assert_eq!(stats.move_completions, 1);
        assert_eq!(runtime.current(person), None);
        assert_eq!(env.persons.location(person), Some(meal));
        let events = runtime.drain_events();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type(), "action.completed");
        assert_eq!(events[0].timestamp(), at(2));
        assert_eq!(events[0].actors(), &[person]);
        assert!(events[0].validate().is_ok());
        assert_eq!(
            events[0].metadata().get("action_kind"),
            Some(&serde_json::Value::from("Move"))
        );
    }

    #[test]
    fn zero_distance_action_still_occupies_one_second() {
        let mut fixture = Fixture::new();
        let meal = fixture.meal();
        // Spawn directly on the Meal site.
        let person = fixture
            .persons
            .spawn(&mut fixture.allocator, meal)
            .expect("spawn");
        let mut runtime = ActionRuntime::default();
        let mut env = fixture.env();
        runtime
            .start(
                person,
                candidate(ActionKind::Move, Some(meal)),
                &mut env,
                at(0),
            )
            .expect("start");
        assert!(
            runtime
                .advance(at(0), &mut env)
                .expect("advance")
                .transitions()
                .is_empty(),
            "no same-instant completion"
        );
        let outcome = runtime.advance(at(1), &mut env).expect("advance");
        assert_eq!(outcome.transitions().len(), 1);
        assert_eq!(
            outcome.transitions()[0].reason(),
            TransitionReason::Completed
        );
        assert_eq!(runtime.stats().move_completions, 1);
        assert_eq!(env.persons.location(person), Some(meal));
    }

    #[test]
    fn work_lifecycle_counts_completion_and_materializes_needs() {
        let mut fixture = Fixture::new();
        let person = fixture.spawn();
        let work = fixture.work();
        let mut runtime = ActionRuntime::default();
        let mut env = fixture.env();
        runtime
            .start(
                person,
                candidate(ActionKind::Work, Some(work)),
                &mut env,
                at(0),
            )
            .expect("start work");
        // Path length 5 (Manhattan 4): steps at t=1..=3, arrival at t=4.
        for t in 1..=3 {
            let outcome = runtime.advance(at(t), &mut env).expect("advance");
            assert_eq!(outcome.transitions()[0].reason(), TransitionReason::Step);
        }
        let arrival = runtime.advance(at(4), &mut env).expect("arrival");
        let reasons: Vec<TransitionReason> = arrival
            .transitions()
            .iter()
            .map(Transition::reason)
            .collect();
        assert_eq!(
            reasons,
            vec![TransitionReason::Arrived, TransitionReason::Arrived]
        );
        assert_eq!(runtime.current(person), Some(ActionState::Working));
        // Work runs 1,800 seconds: completion at t = 4 + 1800.
        assert!(
            runtime
                .advance(at(1_803), &mut env)
                .expect("advance")
                .transitions()
                .is_empty()
        );
        let done = runtime.advance(at(1_804), &mut env).expect("completion");
        assert_eq!(done.transitions().len(), 1);
        let completion = done.transitions()[0];
        assert_eq!(completion.from(), ActionState::Working);
        assert_eq!(completion.to(), ActionState::Idle);
        assert_eq!(completion.reason(), TransitionReason::Completed);
        let stats = runtime.stats();
        assert_eq!(stats.work_completions, 1);
        assert_eq!(stats.movement_completions, 1);
        // The bounded site work counter observed exactly one completion.
        let counter = env
            .sites
            .site_at(work)
            .and_then(palimpsest_sim_world::ActivitySite::work)
            .expect("work site has a counter");
        assert_eq!(counter.get(), 1);
        // Needs accrued from the epoch to the completion instant exactly once.
        let needs = env.persons.needs(person).expect("person exists");
        assert_eq!(needs.hunger().raw(), 1_804);
        assert_eq!(needs.fatigue().raw(), 3_608);
        let events = runtime.drain_events();
        assert_eq!(events.len(), 1);
        assert_eq!(
            events[0].metadata().get("duration_seconds"),
            Some(&serde_json::Value::from(1_804))
        );
    }

    #[test]
    fn eat_completion_relieves_hunger_after_growth() {
        let mut fixture = Fixture::new();
        let person = fixture.spawn();
        let meal = fixture.meal();
        fixture
            .persons
            .set_needs(person, needs_with(50_000, 10_000))
            .expect("set needs");
        let mut runtime = ActionRuntime::default();
        let mut env = fixture.env();
        runtime
            .start(
                person,
                candidate(ActionKind::Eat, Some(meal)),
                &mut env,
                at(0),
            )
            .expect("start eat");
        runtime.advance(at(1), &mut env).expect("step");
        let arrival = runtime.advance(at(2), &mut env).expect("arrival");
        assert_eq!(runtime.current(person), Some(ActionState::Eating));
        drop(arrival);
        let done = runtime.advance(at(602), &mut env).expect("completion");
        assert_eq!(done.transitions()[0].reason(), TransitionReason::Completed);
        // Materialize first (hunger 50_000 + 602), then relieve 100_000 raw.
        let needs = env.persons.needs(person).expect("person exists");
        assert_eq!(needs.hunger().raw(), 0);
        assert_eq!(needs.fatigue().raw(), 10_000 + 2 * 602);
        assert_eq!(runtime.stats().eat_completions, 1);
    }

    #[test]
    fn sleep_completion_relieves_fatigue_after_growth() {
        let mut fixture = Fixture::new();
        let person = fixture.spawn();
        let rest = fixture.rest();
        fixture
            .persons
            .set_needs(person, needs_with(1_000, 60_000))
            .expect("set needs");
        let mut runtime = ActionRuntime::default();
        let mut env = fixture.env();
        runtime
            .start(
                person,
                candidate(ActionKind::Sleep, Some(rest)),
                &mut env,
                at(0),
            )
            .expect("start sleep");
        runtime.advance(at(1), &mut env).expect("step");
        runtime.advance(at(2), &mut env).expect("arrival");
        assert_eq!(runtime.current(person), Some(ActionState::Sleeping));
        runtime.advance(at(28_802), &mut env).expect("completion");
        let needs = env.persons.needs(person).expect("person exists");
        // Fatigue saturates at NEED_MAX during the long sleep, then rests to 0.
        assert_eq!(needs.fatigue().raw(), 0);
        assert_eq!(needs.hunger().raw(), 1_000 + 28_802);
        assert_eq!(runtime.stats().sleep_completions, 1);
    }

    #[test]
    fn overlap_is_rejected_and_leaves_state_unchanged() {
        let mut fixture = Fixture::new();
        let person = fixture.spawn();
        let work = fixture.work();
        let mut runtime = ActionRuntime::default();
        let mut env = fixture.env();
        runtime
            .start(
                person,
                candidate(ActionKind::Work, Some(work)),
                &mut env,
                at(0),
            )
            .expect("start");
        let second = runtime.start(person, candidate(ActionKind::Idle, None), &mut env, at(0));
        assert_eq!(second, Err(ActionError::AlreadyExecuting { id: person }));
        assert_eq!(runtime.stats().started, 1);
        assert_eq!(
            runtime.current_action(person),
            Some((ActionKind::Work, Some(work)))
        );
    }

    #[test]
    fn unknown_person_and_repeated_cancel_are_typed_errors() {
        let mut fixture = Fixture::new();
        let person = fixture.spawn();
        let missing = EntityId::new(999).expect("non-zero");
        let work = fixture.work();
        let mut runtime = ActionRuntime::default();
        let mut env = fixture.env();
        assert_eq!(
            runtime.start(
                missing,
                candidate(ActionKind::Work, Some(work)),
                &mut env,
                at(0)
            ),
            Err(ActionError::UnknownPerson { id: missing })
        );
        assert_eq!(
            runtime.cancel(missing, CancelReason::External, at(0), &mut env),
            Err(ActionError::UnknownPerson { id: missing })
        );
        assert_eq!(
            runtime.cancel(person, CancelReason::External, at(0), &mut env),
            Err(ActionError::InvalidTransition { id: person }),
            "cancelling a free person is an invalid transition"
        );
        runtime
            .start(
                person,
                candidate(ActionKind::Work, Some(work)),
                &mut env,
                at(0),
            )
            .expect("start");
        runtime
            .cancel(person, CancelReason::External, at(0), &mut env)
            .expect("cancel active action");
        assert_eq!(
            runtime.cancel(person, CancelReason::External, at(0), &mut env),
            Err(ActionError::InvalidTransition { id: person }),
            "repeated cancel cannot execute twice"
        );
        assert_eq!(runtime.stats().cancelled, 1);
    }

    #[test]
    fn blocked_and_unreachable_starts_change_nothing() {
        let mut fixture = Fixture::new();
        let person = fixture.spawn();
        let work = fixture.work();
        let mut runtime = ActionRuntime::default();
        // Eat targeting a Work site is blocked at the start recheck.
        let mut env = fixture.env();
        assert_eq!(
            runtime.start(
                person,
                candidate(ActionKind::Eat, Some(work)),
                &mut env,
                at(0)
            ),
            Err(ActionError::Blocked {
                kind: ActionKind::Eat,
                target: work
            })
        );
        assert_eq!(runtime.current(person), None);
        assert_eq!(runtime.stats().started, 0);
        // A one-cell path cap makes any off-cell target unreachable.
        let mut tight = ActionRuntime::new(
            ActionConfig::new(
                seconds(1),
                seconds(600),
                seconds(28_800),
                seconds(1_800),
                seconds(60),
                seconds(1),
                seconds(60),
                PathConfig::new(usize::MAX, 1),
            )
            .expect("positive durations"),
        );
        assert_eq!(
            tight.start(
                person,
                candidate(ActionKind::Move, Some(work)),
                &mut env,
                at(0)
            ),
            Err(ActionError::Unreachable {
                kind: ActionKind::Move,
                target: work
            })
        );
        assert_eq!(tight.current(person), None);
        assert_eq!(tight.next_due(), None);
    }

    #[test]
    fn blocked_arrival_recovers_to_idle_and_retries_after_delay() {
        let mut fixture = Fixture::new();
        let person = fixture.spawn();
        let meal = fixture.meal();
        let mut runtime = ActionRuntime::default();
        {
            let mut env = fixture.env();
            runtime
                .start(
                    person,
                    candidate(ActionKind::Eat, Some(meal)),
                    &mut env,
                    at(0),
                )
                .expect("start eat");
            runtime.advance(at(1), &mut env).expect("step");
        }
        // The site disappears before arrival (contractual recheck path).
        let mut empty = ActivitySites::new(Vec::new()).expect("empty sites");
        let outcome = {
            let mut env = ActionEnvironment {
                persons: &mut fixture.persons,
                map: &fixture.map,
                sites: &mut empty,
            };
            runtime.advance(at(2), &mut env).expect("arrival recheck")
        };
        let reasons: Vec<TransitionReason> = outcome
            .transitions()
            .iter()
            .map(Transition::reason)
            .collect();
        assert_eq!(
            reasons,
            vec![TransitionReason::Arrived, TransitionReason::Blocked]
        );
        let transition = outcome.transitions()[1];
        assert_eq!(transition.to(), ActionState::Idle);
        assert!(
            outcome.decision_requests().is_empty(),
            "retry waits one second"
        );
        let stats = runtime.stats();
        assert_eq!(stats.blocked, 1);
        assert_eq!(runtime.current(person), None);
        let metrics = runtime.metrics();
        assert_eq!(metrics.live_actions, 0);
        assert_eq!(metrics.pending_retries, 1);
        let events = runtime.drain_events();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type(), "action.blocked");
        assert!(events[0].validate().is_ok());
        // The retry decision surfaces at now + retry_delay, never same-instant.
        let mut env = ActionEnvironment {
            persons: &mut fixture.persons,
            map: &fixture.map,
            sites: &mut empty,
        };
        let retry = runtime.advance(at(3), &mut env).expect("retry pop");
        assert_eq!(retry.decision_requests().len(), 1);
        assert_eq!(retry.decision_requests()[0].reason(), DecisionReason::Retry);
        assert_eq!(runtime.metrics().pending_retries, 0);
    }

    #[test]
    fn failed_work_completion_recovers_to_idle() {
        let mut fixture = Fixture::new();
        let person = fixture.spawn();
        let work = fixture.work();
        let mut runtime = ActionRuntime::default();
        {
            let mut env = fixture.env();
            runtime
                .start(
                    person,
                    candidate(ActionKind::Work, Some(work)),
                    &mut env,
                    at(0),
                )
                .expect("start work");
            for t in 1..=4 {
                runtime.advance(at(t), &mut env).expect("movement");
            }
            assert_eq!(runtime.current(person), Some(ActionState::Working));
        }
        // The site vanishes before the completion instant.
        let mut empty = ActivitySites::new(Vec::new()).expect("empty sites");
        let mut env = ActionEnvironment {
            persons: &mut fixture.persons,
            map: &fixture.map,
            sites: &mut empty,
        };
        let outcome = runtime
            .advance(at(1_804), &mut env)
            .expect("completion pop");
        assert_eq!(outcome.transitions()[0].reason(), TransitionReason::Failed);
        assert_eq!(runtime.stats().failed, 1);
        assert_eq!(runtime.stats().work_completions, 0);
        let events = runtime.drain_events();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type(), "action.failed");
    }

    #[test]
    fn interrupt_commits_one_transition_and_cancels_live_tokens() {
        let mut fixture = Fixture::new();
        let person = fixture.spawn();
        let work = fixture.work();
        fixture
            .persons
            .set_needs(person, needs_with(30_000, 40_000))
            .expect("set needs");
        let mut runtime = ActionRuntime::default();
        let mut env = fixture.env();
        runtime
            .start(
                person,
                candidate(ActionKind::Work, Some(work)),
                &mut env,
                at(0),
            )
            .expect("start");
        runtime.advance(at(1), &mut env).expect("step");
        runtime.advance(at(2), &mut env).expect("step");
        let transition = runtime
            .cancel(person, CancelReason::Interrupted, at(2), &mut env)
            .expect("interrupt");
        assert_eq!(
            transition.from(),
            ActionState::Moving {
                action: ActionKind::Work
            }
        );
        assert_eq!(transition.to(), ActionState::Idle);
        assert_eq!(transition.reason(), TransitionReason::Interrupted);
        // Needs materialized without any completion reward.
        let needs = env.persons.needs(person).expect("person exists");
        assert_eq!(needs.hunger().raw(), 30_002);
        assert_eq!(needs.fatigue().raw(), 40_004);
        assert_eq!(runtime.stats().interrupted, 1);
        // Only the rescheduled critical check remains live.
        let metrics = runtime.metrics();
        assert_eq!(metrics.live_actions, 0);
        assert_eq!(metrics.live_checks, 1);
        assert_eq!(metrics.scheduler.scheduled_entries, 1);
        // No continuation can execute after the interrupt.
        runtime.advance(at(10_000), &mut env).expect("advance");
        assert_eq!(runtime.stats().steps, 2);
        let events = runtime.drain_events();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type(), "action.interrupted");
        assert!(events[0].validate().is_ok());
    }

    #[test]
    fn old_tokens_cannot_execute_twice_after_restart() {
        let mut fixture = Fixture::new();
        let person = fixture.spawn();
        let meal = fixture.meal();
        let rest = fixture.rest();
        let mut runtime = ActionRuntime::default();
        let mut env = fixture.env();
        runtime
            .start(
                person,
                candidate(ActionKind::Move, Some(meal)),
                &mut env,
                at(0),
            )
            .expect("start first move");
        runtime.advance(at(1), &mut env).expect("one step");
        runtime
            .cancel(person, CancelReason::External, at(1), &mut env)
            .expect("cancel first move");
        runtime
            .start(
                person,
                candidate(ActionKind::Move, Some(rest)),
                &mut env,
                at(1),
            )
            .expect("start second move");
        // A single long advance must equal per-instant stepping (ADR-0021).
        runtime
            .advance(at(4), &mut env)
            .expect("finish second move");
        let stats = runtime.stats();
        // First action: one step, cancelled. Second: three steps from the
        // stepped location, one arrival, one completion. Nothing from the
        // cancelled token executed.
        assert_eq!(stats.steps, 4);
        assert_eq!(stats.movement_completions, 1);
        assert_eq!(stats.move_completions, 1);
        assert_eq!(stats.cancelled, 1);
        let events = runtime.drain_events();
        let event_types: Vec<&str> = events
            .iter()
            .map(palimpsest_sim_events::EventRecord::event_type)
            .collect();
        assert_eq!(event_types, vec!["action.cancelled", "action.completed"]);
    }

    #[test]
    fn critical_boundary_fires_and_rechecks_with_positive_delay() {
        let mut fixture = Fixture::new();
        let person = fixture.spawn();
        let work = fixture.work();
        fixture
            .persons
            .set_needs(person, needs_with(0, 89_998))
            .expect("set needs");
        let mut runtime = ActionRuntime::default();
        let mut env = fixture.env();
        runtime
            .start(
                person,
                candidate(ActionKind::Work, Some(work)),
                &mut env,
                at(0),
            )
            .expect("start");
        // Fatigue crosses CRITICAL_PRESSURE after (90_000 - 89_998) / 2 = 1s.
        let first = runtime.advance(at(1), &mut env).expect("check pop");
        let requests: Vec<DecisionReason> = first
            .decision_requests()
            .iter()
            .map(DecisionRequest::reason)
            .collect();
        assert_eq!(requests, vec![DecisionReason::CriticalBoundary]);
        // Materialized at the boundary: fatigue is exactly critical now.
        let needs = env.persons.needs(person).expect("person exists");
        assert_eq!(needs.fatigue().raw(), 90_000);
        assert!(needs.is_critical());
        // Still critical: the next recheck waits the positive 60s delay.
        assert!(
            runtime
                .advance(at(60), &mut env)
                .expect("before recheck")
                .decision_requests()
                .is_empty()
        );
        let second = runtime.advance(at(61), &mut env).expect("recheck pop");
        assert!(
            second
                .decision_requests()
                .iter()
                .any(|request| request.reason() == DecisionReason::CriticalBoundary)
        );
    }

    #[test]
    fn driver_interrupts_only_when_another_action_wins() {
        let mut fixture = Fixture::new();
        let person = fixture.spawn();
        let work = fixture.work();
        let meal = fixture.meal();
        fixture
            .persons
            .set_needs(person, needs_with(0, 89_998))
            .expect("set needs");
        let mut runtime = ActionRuntime::default();
        let weights = Weights::default();
        let spec = PerturbationSpec::ZERO;
        // Start Work directly (bypassing the driver) so a critical fatigue
        // boundary can supersede it.
        {
            let mut env = fixture.env();
            runtime
                .start(
                    person,
                    candidate(ActionKind::Work, Some(work)),
                    &mut env,
                    at(0),
                )
                .expect("start work");
        }
        let mut env = fixture.env();
        let check = runtime.advance(at(1), &mut env).expect("critical check");
        let request = check.decision_requests()[0];
        let resolution =
            resolve_decision(&mut runtime, &request, &mut env, &weights, &spec).expect("resolve");
        // Fatigue is critical: Sleep must win over the executing Work.
        assert_eq!(resolution.selection().candidate().kind(), ActionKind::Sleep);
        assert_eq!(resolution.transitions().len(), 2);
        assert_eq!(
            resolution.transitions()[0].reason(),
            TransitionReason::Interrupted
        );
        assert_eq!(
            resolution.transitions()[1].reason(),
            TransitionReason::Started
        );
        assert_eq!(
            runtime.current_action(person).map(|(kind, _)| kind),
            Some(ActionKind::Sleep)
        );
        assert_eq!(runtime.stats().interrupted, 1);

        // Conversely: when the current action already is the winner, no
        // interrupt happens. Person eating with critical hunger.
        let mut fixture = Fixture::new();
        let person = fixture.spawn();
        fixture
            .persons
            .set_needs(person, needs_with(89_998, 0))
            .expect("set needs");
        let mut runtime = ActionRuntime::default();
        {
            let mut env = fixture.env();
            runtime
                .start(
                    person,
                    candidate(ActionKind::Eat, Some(meal)),
                    &mut env,
                    at(0),
                )
                .expect("start eat");
        }
        let mut env = fixture.env();
        // Movement to the Meal site takes 2s; the hunger boundary
        // ((90_000 - 89_998) / 1) fires at t=2 alongside the arrival.
        let check = runtime.advance(at(2), &mut env).expect("critical check");
        let request = check
            .decision_requests()
            .iter()
            .copied()
            .find(|request| request.reason() == DecisionReason::CriticalBoundary)
            .expect("critical check emitted");
        let resolution =
            resolve_decision(&mut runtime, &request, &mut env, &weights, &spec).expect("resolve");
        assert_eq!(resolution.selection().candidate().kind(), ActionKind::Eat);
        assert!(
            resolution.transitions().is_empty(),
            "same winner never interrupts"
        );
        assert_eq!(
            runtime.current_action(person).map(|(kind, _)| kind),
            Some(ActionKind::Eat)
        );
        assert_eq!(runtime.stats().interrupted, 0);
    }

    #[test]
    fn identical_runs_produce_byte_identical_transition_logs() {
        fn run() -> (Vec<super::Transition>, Vec<String>, super::ActionStats) {
            let mut fixture = Fixture::new();
            let person = fixture.spawn();
            let mut runtime = ActionRuntime::default();
            let weights = Weights::default();
            let spec = PerturbationSpec::ZERO;
            let mut transitions = Vec::new();
            {
                let mut env = fixture.env();
                let resolution =
                    decide_and_start(&mut runtime, person, &mut env, &weights, &spec, at(0))
                        .expect("initial decision");
                transitions.extend_from_slice(resolution.transitions());
            }
            let mut env = fixture.env();
            while let Some(next) = runtime.next_due() {
                if next > at(5_000) {
                    break;
                }
                let outcome = runtime.advance(next, &mut env).expect("advance");
                transitions.extend_from_slice(outcome.transitions());
                for request in outcome.decision_requests() {
                    let resolution =
                        resolve_decision(&mut runtime, request, &mut env, &weights, &spec)
                            .expect("resolve");
                    transitions.extend_from_slice(resolution.transitions());
                }
            }
            let events = runtime
                .drain_events()
                .iter()
                .map(|event| format!("{event:?}"))
                .collect();
            (transitions, events, runtime.stats())
        }
        let (first_t, first_e, first_s) = run();
        let (second_t, second_e, second_s) = run();
        assert_eq!(first_t, second_t, "transition logs diverge");
        assert_eq!(first_e, second_e, "event streams diverge");
        assert_eq!(first_s, second_s, "stats diverge");
        assert!(first_s.work_completions > 0, "the reference context works");
    }

    #[test]
    fn event_buffer_is_bounded_with_visible_rotation() {
        let mut fixture = Fixture::new();
        let person = fixture.spawn();
        let origin = fixture.origin;
        let east = coord(origin.x() + 1, origin.y());
        let mut runtime = ActionRuntime::default();
        let mut env = fixture.env();
        let total = EVENT_BUFFER_CAPACITY + 104;
        let mut now = 0_i64;
        let mut target = east;
        for _ in 0..total {
            runtime
                .start(
                    person,
                    candidate(ActionKind::Move, Some(target)),
                    &mut env,
                    at(now),
                )
                .expect("start move");
            now += 1;
            let outcome = runtime.advance(at(now), &mut env).expect("completion");
            assert_eq!(
                outcome.transitions().last().map(Transition::reason),
                Some(TransitionReason::Completed)
            );
            target = if target == east { origin } else { east };
        }
        let events = runtime.drain_events();
        assert_eq!(events.len(), EVENT_BUFFER_CAPACITY);
        assert_eq!(runtime.metrics().events_rotated, 104);
        for event in &events {
            assert!(event.validate().is_ok());
            assert_eq!(event.actors(), &[person]);
        }
        // Timestamps are monotonically non-decreasing in drain order.
        let mut previous = 0_i64;
        for event in &events {
            assert!(event.timestamp().as_seconds() >= previous);
            previous = event.timestamp().as_seconds();
        }
    }

    #[test]
    fn metrics_bound_two_live_tokens_per_person() {
        let mut fixture = Fixture::new();
        let person = fixture.spawn();
        let work = fixture.work();
        let mut runtime = ActionRuntime::default();
        let mut env = fixture.env();
        runtime
            .start(
                person,
                candidate(ActionKind::Work, Some(work)),
                &mut env,
                at(0),
            )
            .expect("start");
        let metrics = runtime.metrics();
        assert_eq!(metrics.live_actions, 1);
        assert_eq!(metrics.live_checks, 1);
        assert_eq!(metrics.scheduler.scheduled_entries, 2);
    }

    #[test]
    fn config_rejects_nonpositive_durations() {
        let zero = SimDuration::ZERO;
        assert!(
            ActionConfig::new(
                zero,
                seconds(600),
                seconds(28_800),
                seconds(1_800),
                seconds(60),
                seconds(1),
                seconds(60),
                PathConfig::default(),
            )
            .is_none()
        );
    }

    #[test]
    fn idle_wait_blocks_overlap_and_completes_without_event() {
        let mut fixture = Fixture::new();
        let person = fixture.spawn();
        let mut runtime = ActionRuntime::default();
        let mut env = fixture.env();
        runtime
            .start(person, candidate(ActionKind::Idle, None), &mut env, at(0))
            .expect("start idle");
        assert_eq!(runtime.current(person), Some(ActionState::Idle));
        assert_eq!(
            runtime.current_action(person),
            Some((ActionKind::Idle, None))
        );
        assert_eq!(
            runtime.start(person, candidate(ActionKind::Idle, None), &mut env, at(0)),
            Err(ActionError::AlreadyExecuting { id: person })
        );
        let outcome = runtime.advance(at(60), &mut env).expect("idle completion");
        assert_eq!(
            outcome.transitions()[0].reason(),
            TransitionReason::Completed
        );
        assert_eq!(runtime.stats().idle_completions, 1);
        assert!(
            runtime.drain_events().is_empty(),
            "Idle waits emit no event"
        );
        assert_eq!(
            outcome.decision_requests()[0].reason(),
            DecisionReason::Completed
        );
    }

    #[test]
    fn decide_and_start_uses_the_live_reference_context() {
        let mut fixture = Fixture::new();
        let person = fixture.spawn();
        let mut runtime = ActionRuntime::default();
        let mut env = fixture.env();
        let resolution = decide_and_start(
            &mut runtime,
            person,
            &mut env,
            &Weights::default(),
            &PerturbationSpec::ZERO,
            at(0),
        )
        .expect("initial decision");
        // ADR-0018 reference context: fresh needs select Work.
        assert_eq!(resolution.selection().candidate().kind(), ActionKind::Work);
        assert!(resolution.selection().trace().selected().is_some());
        assert_eq!(
            runtime.current_action(person),
            Some((ActionKind::Work, Some(fixture.work())))
        );
        // A person with no reachable sites falls back to the Idle baseline.
        let mut fixture = Fixture::new();
        let person = fixture.spawn();
        let mut empty = ActivitySites::new(Vec::new()).expect("empty sites");
        let mut runtime = ActionRuntime::default();
        let mut env = ActionEnvironment {
            persons: &mut fixture.persons,
            map: &fixture.map,
            sites: &mut empty,
        };
        let resolution = decide_and_start(
            &mut runtime,
            person,
            &mut env,
            &Weights::default(),
            &PerturbationSpec::ZERO,
            at(0),
        )
        .expect("idle decision");
        assert_eq!(resolution.selection().candidate().kind(), ActionKind::Idle);
        assert_eq!(runtime.current(person), Some(ActionState::Idle));
    }

    #[test]
    fn run_until_drives_the_closed_loop() {
        let mut fixture = Fixture::new();
        let person = fixture.spawn();
        let mut runtime = ActionRuntime::default();
        let weights = Weights::default();
        let spec = PerturbationSpec::ZERO;
        {
            let mut env = fixture.env();
            decide_and_start(&mut runtime, person, &mut env, &weights, &spec, at(0))
                .expect("initial decision");
        }
        let mut env = fixture.env();
        run_until(&mut runtime, &mut env, at(86_400), &weights, &spec).expect("one day");
        let stats = runtime.stats();
        assert!(stats.work_completions > 0);
        assert!(
            stats.sleep_completions > 0,
            "fatigue crosses the Sleep threshold within a day"
        );
        let needs = env.persons.needs(person).expect("person exists");
        assert!(needs.hunger().raw() <= 100_000 && needs.fatigue().raw() <= 100_000);
    }

    /// Runs the reference driver and collects every committed transition.
    /// `segmented` steps one due instant at a time; `!segmented` jumps to the
    /// target in one long advance per decision round.
    fn run_segmented(segmented: bool) -> (Vec<Transition>, Vec<String>, ActionStats) {
        let mut fixture = Fixture::new();
        let person = fixture.spawn();
        let mut runtime = ActionRuntime::default();
        let weights = Weights::default();
        let spec = PerturbationSpec::ZERO;
        let mut transitions = Vec::new();
        {
            let mut env = fixture.env();
            let resolution =
                decide_and_start(&mut runtime, person, &mut env, &weights, &spec, at(0))
                    .expect("initial decision");
            transitions.extend_from_slice(resolution.transitions());
        }
        let mut env = fixture.env();
        let target = at(5_000);
        loop {
            let horizon = if segmented {
                match runtime.next_due() {
                    Some(next) if next <= target => next,
                    _ => break,
                }
            } else {
                if runtime.next_due().is_none_or(|next| next > target) {
                    break;
                }
                target
            };
            let outcome = runtime.advance(horizon, &mut env).expect("advance");
            transitions.extend_from_slice(outcome.transitions());
            for request in outcome.decision_requests() {
                let resolution = resolve_decision(&mut runtime, request, &mut env, &weights, &spec)
                    .expect("resolve");
                transitions.extend_from_slice(resolution.transitions());
            }
        }
        let events = runtime
            .drain_events()
            .iter()
            .map(|event| format!("{event:?}"))
            .collect();
        (transitions, events, runtime.stats())
    }

    #[test]
    fn one_long_advance_equals_per_instant_stepping() {
        let (seg_t, seg_e, seg_s) = run_segmented(true);
        let (long_t, long_e, long_s) = run_segmented(false);
        assert_eq!(seg_t, long_t, "long advance diverges from stepping");
        assert_eq!(seg_e, long_e);
        assert_eq!(seg_s, long_s);
        assert!(seg_s.work_completions > 0);
    }

    #[test]
    fn exhausted_event_id_rejects_cancel_without_mutation() {
        let mut fixture = Fixture::new();
        let person = fixture.spawn();
        let mut runtime = ActionRuntime::default();
        let mut env = fixture.env();
        runtime
            .start(person, candidate(ActionKind::Idle, None), &mut env, at(0))
            .expect("start");
        let before = runtime.metrics();
        runtime.next_event_raw = 0;
        let result = runtime.cancel(person, CancelReason::External, at(0), &mut env);
        assert_eq!(result, Err(ActionError::EventLogExhausted));
        assert_eq!(
            runtime.current_action(person),
            Some((ActionKind::Idle, None))
        );
        assert_eq!(runtime.metrics().events_total, before.events_total);
        assert_eq!(runtime.metrics().events_digest, before.events_digest);
        assert_eq!(runtime.metrics().scheduler, before.scheduler);
    }
}
