// Authored by opencode (AI coding agent) — task CHRON-028 (ADR-0022).
//! The Phase 1 headless world kernel (CHRON-028, ADR-0022).
//!
//! [`WorldKernel`] is the sole owner of time advancement, ordering, and the
//! person/terrain/action world: it owns the [`SimClock`], the static
//! [`WorldMap`], the [`ActivitySites`], the [`PersonRuntime`], the identity
//! allocator, the [`ActionRuntime`] (CHRON-027), the decision
//! weights/perturbation, the latest per-person decision trace, and a bounded
//! in-memory event buffer. It drives the world by jumping between due
//! instants — never scanning every person every simulated second (ADR-0004,
//! P1-REMAINING D2) — and it resolves only the [`crate::DecisionRequest`] values the
//! action runtime surfaces; it never invents a decision or an action.
//!
//! The kernel is deterministic and headless: no wall-clock, thread, float, or
//! unordered-iteration dependence enters truth, and `EntityId` is the only
//! identity crossing any boundary (ADR-0002/0011). Runtime execution state and
//! scheduler tokens are never serialized. Every [`advance_to`]
//! call is bounded by a work budget and reports the
//! last fully committed instant. The budget counts full due instants, not wall
//! time; a round's cost scales with population. Splitting a target is equivalent to
//! advancing to it in one call (segmentation equivalence).

use std::collections::{BTreeMap, VecDeque};
use std::error::Error;
use std::fmt::{Display, Formatter};

use palimpsest_sim_ai::{ActionKind, DecisionTrace, Needs, PerturbationSpec, Weights};
use palimpsest_sim_entity::{EntityId, EntityIdAllocationError, EntityIdAllocator};
use palimpsest_sim_events::{EventRecord, EventValidationError};
use palimpsest_sim_time::{SimClock, SimClockError, SimInstant};
use palimpsest_sim_world::{ActivitySites, LocalCoord, WorldGenConfig, WorldMap, WorldSeed};
use serde::{Deserialize, Serialize};

use crate::actions::{
    ActionConfig, ActionEnvironment, ActionError, ActionRuntime, ActionRuntimeMetrics, ActionState,
    DecisionDriveError, Transition, TransitionReason, decide_and_start, resolve_decisions,
};
use crate::person::{PersonError, PersonRuntime};

/// Default maximum number of due-instant advance rounds processed per
/// [`WorldKernel::advance_to`] call (P1-REMAINING D2: "at most 1,024");
/// each round processes one fully due instant and its decision requests.
pub const DEFAULT_WORK_BUDGET: usize = 1_024;

/// Default capacity of the kernel's bounded in-memory outcome-event buffer.
/// Overflow drops the oldest record and increments the visible rotation
/// counter; this is a runtime diagnostic sink, not durable retention.
pub const DEFAULT_EVENT_BUFFER_CAPACITY: usize = 4_096;

/// Kernel composition inputs (CHRON-028, ADR-0022 §1).
///
/// The `Default` table is the P1-REMAINING D2 Phase 1 set: the default action
/// configuration, the ADR-0018 default weights, zero perturbation, the
/// default work budget, and the default event-buffer capacity.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct KernelConfig {
    action: ActionConfig,
    weights: Weights,
    perturbation: PerturbationSpec,
    work_budget: usize,
    event_buffer_capacity: usize,
}

impl KernelConfig {
    /// Creates a kernel configuration.
    ///
    /// # Errors
    ///
    /// Returns [`KernelConfigError::InvalidWorkBudget`] when `work_budget` is
    /// zero, or [`KernelConfigError::InvalidEventCapacity`] when
    /// `event_buffer_capacity` is zero.
    pub fn new(
        action: ActionConfig,
        weights: Weights,
        perturbation: PerturbationSpec,
        work_budget: usize,
        event_buffer_capacity: usize,
    ) -> Result<Self, KernelConfigError> {
        if work_budget == 0 {
            return Err(KernelConfigError::InvalidWorkBudget);
        }
        if event_buffer_capacity == 0 {
            return Err(KernelConfigError::InvalidEventCapacity);
        }
        Ok(Self {
            action,
            weights,
            perturbation,
            work_budget,
            event_buffer_capacity,
        })
    }

    /// The action execution configuration.
    #[must_use]
    pub const fn action(&self) -> ActionConfig {
        self.action
    }

    /// The utility weights applied to every decision.
    #[must_use]
    pub const fn weights(&self) -> Weights {
        self.weights
    }

    /// The perturbation spec applied to every decision.
    #[must_use]
    pub const fn perturbation(&self) -> PerturbationSpec {
        self.perturbation
    }

    /// The default advance-round budget per call.
    #[must_use]
    pub const fn work_budget(&self) -> usize {
        self.work_budget
    }

    /// The bounded event-buffer capacity.
    #[must_use]
    pub const fn event_buffer_capacity(&self) -> usize {
        self.event_buffer_capacity
    }
}

impl Default for KernelConfig {
    fn default() -> Self {
        Self {
            action: ActionConfig::default(),
            weights: Weights::default(),
            perturbation: PerturbationSpec::ZERO,
            work_budget: DEFAULT_WORK_BUDGET,
            event_buffer_capacity: DEFAULT_EVENT_BUFFER_CAPACITY,
        }
    }
}

/// The committed result of one bounded [`WorldKernel::advance_to`] call.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct KernelAdvance {
    committed_to: SimInstant,
    rounds: usize,
    reached_target: bool,
    transitions: usize,
    decisions: usize,
    events: usize,
}

/// Committed action observations retained independently of event buffering.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize)]
pub struct PersonObservations {
    pub movement_steps: u64,
    pub movement_phases: u64,
    pub moves: u64,
    pub eats: u64,
    pub sleeps: u64,
    pub works: u64,
    pub idles: u64,
}

/// Read-only, ordered diagnostics for committed kernel work.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize)]
pub struct KernelObservations {
    pub persons: BTreeMap<EntityId, PersonObservations>,
    pub boundary_count: u64,
    pub queue_depth_sum: u64,
    /// Minimum live entries across successful advance returns; None before any.
    pub queue_depth_min: Option<usize>,
    pub queue_depth_max: usize,
    pub queue_nodes_sum: u64,
    /// Minimum heap nodes across successful advance returns.
    pub queue_nodes_min: Option<usize>,
    pub queue_nodes_max: usize,
}

impl KernelAdvance {
    /// The last instant at which work was fully committed.
    #[must_use]
    pub const fn committed_to(&self) -> SimInstant {
        self.committed_to
    }

    /// The number of full due-instant advance rounds processed.
    #[must_use]
    pub const fn rounds(&self) -> usize {
        self.rounds
    }

    /// Whether the target was reached without exhausting the work budget.
    #[must_use]
    pub const fn reached_target(&self) -> bool {
        self.reached_target
    }

    /// The number of action transitions committed.
    #[must_use]
    pub const fn transitions(&self) -> usize {
        self.transitions
    }

    /// The number of decisions resolved.
    #[must_use]
    pub const fn decisions(&self) -> usize {
        self.decisions
    }

    /// The number of high-level outcome events accounted.
    #[must_use]
    pub const fn events(&self) -> usize {
        self.events
    }
}

/// A read-only view of one person crossing the kernel boundary (CHRON-028).
///
/// This is the only person shape the kernel exposes for rendering and
/// diagnostics: stable `EntityId`, tile, needs, current action kind/target, and
/// observable action state. Runtime ECS handles and scheduler tokens never
/// appear (ADR-0002/0011).
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct KernelPersonView {
    id: EntityId,
    location: LocalCoord,
    needs: Needs,
    action: ActionKind,
    action_target: Option<LocalCoord>,
    state: ActionState,
}

impl KernelPersonView {
    /// The person's stable persistent identity.
    #[must_use]
    pub const fn id(&self) -> EntityId {
        self.id
    }

    /// The tile the person occupies.
    #[must_use]
    pub const fn location(&self) -> LocalCoord {
        self.location
    }

    /// The person's current needs.
    #[must_use]
    pub const fn needs(&self) -> Needs {
        self.needs
    }

    /// The top-level action kind the person is executing.
    #[must_use]
    pub const fn action(&self) -> ActionKind {
        self.action
    }

    /// The action target, if any.
    #[must_use]
    pub const fn action_target(&self) -> Option<LocalCoord> {
        self.action_target
    }

    /// The person's observable action state.
    #[must_use]
    pub const fn state(&self) -> ActionState {
        self.state
    }
}

/// Read-only kernel health for Developer Metrics (CHRON-028, ADR-0022 §3).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct KernelMetrics {
    /// Lifecycle marker; counts and queue diagnostics are from the last complete boundary.
    pub state: KernelState,
    /// Failed instant, when the live runtime must no longer be inspected.
    pub failed_at: Option<SimInstant>,
    /// The committed simulation instant.
    pub now: SimInstant,
    /// Number of spawned persons.
    pub person_count: usize,
    /// Persons with an active action execution record.
    pub live_actions: usize,
    /// Persons waiting on a positive retry delay.
    pub pending_retries: usize,
    /// Live critical-need check tokens.
    pub live_checks: usize,
    /// Live scheduler payloads under the action runtime.
    pub scheduler_queue_depth: usize,
    /// Lazy invalidated scheduler heap nodes awaiting compaction.
    pub scheduler_stale_nodes: usize,
    /// Outcome events currently buffered in the kernel.
    pub events_buffered: usize,
    /// Outcome events dropped by either retention buffer (each event once).
    pub events_rotated: u64,
    /// The cumulative FNV-1a-64 stream digest of every committed event.
    pub events_digest: u64,
    /// Total advance rounds processed across all calls.
    pub rounds_total: u64,
    /// Total action transitions committed across all calls.
    pub transitions_total: u64,
    /// Total decisions resolved across all calls.
    pub decisions_total: u64,
    /// Total validated high-level outcome events accounted.
    pub events_total: u64,
}

/// Failures of kernel operations (CHRON-028 API contract).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum KernelError {
    /// A requested target earlier than the current clock; no mutation occurs.
    ClockRegression {
        /// The current committed instant.
        current: SimInstant,
        /// The rejected earlier target.
        requested: SimInstant,
    },
    /// The clock could not be advanced.
    Clock {
        /// The clock failure.
        source: SimClockError,
    },
    /// The identity allocator was exhausted while spawning.
    Identity {
        /// The allocator failure.
        source: EntityIdAllocationError,
    },
    /// The person runtime rejected an operation.
    Person {
        /// The person failure.
        source: PersonError,
    },
    /// The action runtime rejected an operation.
    Action {
        /// The action failure.
        source: ActionError,
    },
    /// A fresh decision (selection or execution start) failed.
    Decision {
        /// The decision-drive failure.
        source: DecisionDriveError,
    },
    /// A high-level outcome event failed validation.
    Event {
        /// The event-validation failure.
        source: EventValidationError,
    },
    /// A zero work budget was passed to [`WorldKernel::advance_to`]; the
    /// call is a recoverable input rejection and does not fault the kernel.
    InvalidBudget,
    /// A forward advance was requested on a non-empty `Setup` world that has
    /// not been started; the call is a recoverable rejection.
    NotStarted,
    /// [`WorldKernel::start_world`] was called when the kernel is already
    /// `Running`; a recoverable rejection.
    AlreadyStarted,
    /// A Setup-only operation was attempted outside `Setup` (e.g. a spawn or
    /// `start_world` on a `Running`/`Faulted` kernel); recoverable.
    NotSetup,
    /// `start_world` was called with an instant other than the epoch (or the
    /// clock is not at the epoch); recoverable.
    NotAtEpoch,
    /// An operation was attempted on a faulted kernel; recoverable.
    KernelFaulted,
}

/// The kernel lifecycle (ADR-0024 D3).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum KernelState {
    /// A fresh, un-started kernel at the epoch; the only state that accepts
    /// [`WorldKernel::spawn_person`] and [`WorldKernel::start_world`].
    Setup,
    /// The kernel has been started and is advancing; the normal state.
    Running,
    /// A real execution/decision/identity-honest error stopped the kernel.
    /// No further mutation or dynamic read is allowed; the world must be
    /// re-created to run again.
    Faulted,
}

/// Read-only kernel health (ADR-0024 D3): state plus full-boundary markers.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct KernelHealth {
    /// The lifecycle state.
    pub state: KernelState,
    /// The last fully committed boundary instant.
    pub last_complete: SimInstant,
    /// The instant at which a fatal fault occurred, if any.
    pub failed_at: Option<SimInstant>,
    /// The typed cause of a fatal fault, if any.
    pub cause: Option<KernelError>,
}

/// Errors from reading dynamic kernel state (ADR-0024 D3).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum KernelReadError {
    /// The kernel is faulted; no live dynamic view is available.
    KernelFaulted,
    /// Needs cannot be projected to the committed instant; never return stale values.
    InvalidNeedsTime {
        /// Stable identity of the affected person.
        id: EntityId,
        /// The requested complete-boundary instant.
        at: SimInstant,
    },
}

impl Display for KernelReadError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::KernelFaulted => formatter.write_str("kernel is faulted"),
            Self::InvalidNeedsTime { id, at } => {
                write!(formatter, "cannot project needs for {id} to {at}")
            }
        }
    }
}

impl Error for KernelReadError {}

/// Invalid [`KernelConfig`] inputs (ADR-0024 D3).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum KernelConfigError {
    /// The work budget must be non-zero.
    InvalidWorkBudget,
    /// The event-buffer capacity must be non-zero.
    InvalidEventCapacity,
}

impl Display for KernelConfigError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidWorkBudget => formatter.write_str("kernel work budget must be non-zero"),
            Self::InvalidEventCapacity => {
                formatter.write_str("kernel event buffer capacity must be non-zero")
            }
        }
    }
}

impl Error for KernelConfigError {}

impl Display for KernelError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ClockRegression { current, requested } => write!(
                formatter,
                "simulation time cannot move backward from {current} to {requested}"
            ),
            Self::Clock { source } => write!(formatter, "clock failure: {source}"),
            Self::Identity { source } => write!(formatter, "identity failure: {source}"),
            Self::Person { source } => write!(formatter, "person runtime failure: {source}"),
            Self::Action { source } => write!(formatter, "action failure: {source}"),
            Self::Decision { source } => write!(formatter, "decision failure: {source}"),
            Self::Event { source } => write!(formatter, "event failure: {source}"),
            Self::InvalidBudget => formatter.write_str("kernel work budget must be non-zero"),
            Self::NotStarted => formatter.write_str("kernel world has not been started"),
            Self::AlreadyStarted => formatter.write_str("kernel world is already running"),
            Self::NotSetup => formatter.write_str("kernel operation requires the Setup state"),
            Self::NotAtEpoch => formatter.write_str("kernel start_world requires the epoch"),
            Self::KernelFaulted => formatter.write_str("kernel is faulted"),
        }
    }
}

impl Error for KernelError {}

impl From<SimClockError> for KernelError {
    fn from(source: SimClockError) -> Self {
        Self::Clock { source }
    }
}

impl From<EntityIdAllocationError> for KernelError {
    fn from(source: EntityIdAllocationError) -> Self {
        Self::Identity { source }
    }
}

impl From<PersonError> for KernelError {
    fn from(source: PersonError) -> Self {
        Self::Person { source }
    }
}

impl From<ActionError> for KernelError {
    fn from(source: ActionError) -> Self {
        Self::Action { source }
    }
}

impl From<DecisionDriveError> for KernelError {
    fn from(source: DecisionDriveError) -> Self {
        Self::Decision { source }
    }
}

impl From<EventValidationError> for KernelError {
    fn from(source: EventValidationError) -> Self {
        Self::Event { source }
    }
}

/// The authoritative Phase 1 headless world kernel (CHRON-028, ADR-0022 §1).
pub struct WorldKernel {
    clock: SimClock,
    state: KernelState,
    failed_at: Option<SimInstant>,
    fault_cause: Option<KernelError>,
    map: WorldMap,
    sites: ActivitySites,
    persons: PersonRuntime,
    allocator: EntityIdAllocator,
    actions: ActionRuntime,
    committed_action_metrics: ActionRuntimeMetrics,
    weights: Weights,
    perturbation: PerturbationSpec,
    work_budget: usize,
    event_buffer_capacity: usize,
    population: Vec<EntityId>,
    decisions: BTreeMap<EntityId, DecisionTrace>,
    events: VecDeque<EventRecord>,
    events_rotated: u64,
    events_digest: u64,
    rounds_total: u64,
    transitions_total: u64,
    decisions_total: u64,
    events_total: u64,
    observations: KernelObservations,
}

impl WorldKernel {
    /// Creates a kernel over an already-generated world and site set.
    #[must_use]
    pub fn new(map: WorldMap, sites: ActivitySites, config: KernelConfig) -> Self {
        let actions = ActionRuntime::new(config.action);
        let committed_action_metrics = actions.metrics();
        Self {
            clock: SimClock::default(),
            state: KernelState::Setup,
            failed_at: None,
            fault_cause: None,
            map,
            sites,
            persons: PersonRuntime::new(),
            allocator: EntityIdAllocator::default(),
            actions,
            committed_action_metrics,
            weights: config.weights,
            perturbation: config.perturbation,
            work_budget: config.work_budget,
            event_buffer_capacity: config.event_buffer_capacity,
            population: Vec::new(),
            decisions: BTreeMap::new(),
            events: VecDeque::new(),
            events_rotated: 0,
            events_digest: crate::actions::EVENT_DIGEST_OFFSET,
            rounds_total: 0,
            transitions_total: 0,
            decisions_total: 0,
            events_total: 0,
            observations: KernelObservations::default(),
        }
    }

    /// Generates a deterministic world from `seed` and places the default
    /// activity site set, then builds a kernel over it.
    #[must_use]
    pub fn from_world(seed: WorldSeed, config: KernelConfig) -> Self {
        let map = WorldMap::generate(seed, WorldGenConfig::default());
        let sites = ActivitySites::place_defaults(&map);
        Self::new(map, sites, config)
    }

    /// The committed simulation instant (always the last full boundary).
    #[must_use]
    pub const fn now(&self) -> SimInstant {
        self.clock.now()
    }

    /// The current lifecycle state.
    #[must_use]
    pub const fn state(&self) -> KernelState {
        self.state
    }

    /// Read-only kernel health (state plus full-boundary fault markers).
    #[must_use]
    pub fn health(&self) -> KernelHealth {
        KernelHealth {
            state: self.state,
            last_complete: self.clock.now(),
            failed_at: self.failed_at,
            cause: self.fault_cause.clone(),
        }
    }

    /// The static local world map.
    #[must_use]
    pub const fn map(&self) -> &WorldMap {
        &self.map
    }

    /// The activity-site collection, including committed Work counters.
    ///
    /// # Errors
    /// Returns [`KernelReadError::KernelFaulted`] rather than exposing partial work.
    pub fn sites(&self) -> Result<&ActivitySites, KernelReadError> {
        if self.state == KernelState::Faulted {
            return Err(KernelReadError::KernelFaulted);
        }
        Ok(&self.sites)
    }

    /// Spawns a person at `location`, allocating a fresh stable identity.
    ///
    /// The person is placed but enters no action until
    /// [`start_world`](WorldKernel::start_world).
    ///
    /// # Errors
    ///
    /// Returns [`KernelError::NotSetup`], [`KernelError::KernelFaulted`],
    /// [`KernelError::NotAtEpoch`], or a wrapped identity error; no partial
    /// spawn state is left behind.
    pub fn spawn_person(&mut self, location: LocalCoord) -> Result<EntityId, KernelError> {
        match self.state {
            KernelState::Faulted => return Err(KernelError::KernelFaulted),
            KernelState::Setup if self.clock.now() == SimInstant::EPOCH => {}
            KernelState::Setup => return Err(KernelError::NotAtEpoch),
            KernelState::Running => return Err(KernelError::NotSetup),
        }
        let id = self.persons.spawn(&mut self.allocator, location)?;
        self.population.push(id);
        self.observations
            .persons
            .insert(id, PersonObservations::default());
        Ok(id)
    }

    /// Kicks off the decision loop for every spawned person at `at`.
    ///
    /// This is the only seed step: it runs [`decide_and_start`] per person so
    /// every person enters advance already holding an active action (at least
    /// an `Idle` wait). The kernel never does this implicitly during advance.
    /// It is legal only in `Setup` at the epoch; success transitions the
    /// kernel to `Running`.
    ///
    /// # Errors
    ///
    /// Returns [`KernelError::NotSetup`]/[`KernelError::KernelFaulted`]/
    /// [`KernelError::AlreadyStarted`]/[`KernelError::NotAtEpoch`], or a
    /// decision error which faults the kernel.
    pub fn start_world(&mut self, at: SimInstant) -> Result<usize, KernelError> {
        match self.state {
            KernelState::Faulted => return Err(KernelError::KernelFaulted),
            KernelState::Running => return Err(KernelError::AlreadyStarted),
            KernelState::Setup
                if at == SimInstant::EPOCH && self.clock.now() == SimInstant::EPOCH => {}
            KernelState::Setup => return Err(KernelError::NotAtEpoch),
        }
        let ids = self.population.clone();
        let mut started = 0_usize;
        for id in ids {
            let result = {
                let mut env = ActionEnvironment {
                    persons: &mut self.persons,
                    map: &self.map,
                    sites: &mut self.sites,
                };
                decide_and_start(
                    &mut self.actions,
                    id,
                    &mut env,
                    &self.weights,
                    &self.perturbation,
                    at,
                )
            };
            match result {
                Ok(resolution) => {
                    self.decisions
                        .insert(id, resolution.selection().trace().clone());
                    started += 1;
                }
                Err(error) => {
                    let cause = KernelError::Decision { source: error };
                    self.enter_fault(cause.clone(), at);
                    return Err(cause);
                }
            }
        }
        self.state = KernelState::Running;
        self.committed_action_metrics = self.actions.metrics();
        Ok(started)
    }

    fn enter_fault(&mut self, cause: KernelError, failed_at: SimInstant) {
        self.state = KernelState::Faulted;
        self.failed_at = Some(failed_at);
        self.fault_cause = Some(cause);
    }

    /// Advances to `target` with the kernel's configured default work budget.
    ///
    /// See [`advance_to`](WorldKernel::advance_to) for semantics.
    ///
    /// # Errors
    ///
    /// Returns a recoverable [`KernelError`] for a clock regression, zero
    /// budget, un-started non-empty `Setup` world, or a faulted kernel, and the
    /// wrapped action/decision/event/clock error (which faults the kernel) on
    /// the first real execution failure.
    pub fn advance(&mut self, target: SimInstant) -> Result<KernelAdvance, KernelError> {
        let budget = self.work_budget;
        self.advance_to(target, budget)
    }

    /// Advances to `target`, processing at most `work_budget` full
    /// due-instant advance rounds, and reports the last committed instant.
    ///
    /// The kernel jumps between due instants (ADR-0004) rather than scanning
    /// every person every second. Each round runs `actions.advance(d)`
    /// (all work due at exactly `d`, due-time/FIFO), resolves every surfaced
    /// decision via the merged [`resolve_decisions`], and advances the clock to
    /// `d`; **each completed round is committed to the running totals before
    /// the next round begins**. When the budget is exhausted before `target`
    /// the call yields `reached_target == false` and reports the actual
    /// `committed_to` so the caller may continue; when all work at or before
    /// `target` is done the clock advances to `target` and
    /// `reached_target == true`.
    ///
    /// `work_budget` counts due-instant advance rounds, not individual items;
    /// a single round's cost scales with the population (ADR-0024 D3).
    ///
    /// # Errors
    ///
    /// - [`KernelError::KernelFaulted`] — the kernel is faulted; no mutation.
    /// - [`KernelError::ClockRegression`] — `target` is earlier than the clock.
    /// - [`KernelError::InvalidBudget`] — `work_budget` is zero.
    /// - [`KernelError::NotStarted`] — a forward advance of a non-empty
    ///   `Setup` world that has not been started.
    /// - A wrapped action/decision/event/clock error — the kernel records the
    ///   fault and returns the error; the last fully committed boundary and
    ///   its committed counts are retained.
    pub fn advance_to(
        &mut self,
        target: SimInstant,
        work_budget: usize,
    ) -> Result<KernelAdvance, KernelError> {
        if target < self.clock.now() {
            return Err(KernelError::ClockRegression {
                current: self.clock.now(),
                requested: target,
            });
        }
        if work_budget == 0 {
            return Err(KernelError::InvalidBudget);
        }
        match self.state {
            KernelState::Faulted => return Err(KernelError::KernelFaulted),
            KernelState::Setup if !self.population.is_empty() && target > self.clock.now() => {
                return Err(KernelError::NotStarted);
            }
            KernelState::Setup | KernelState::Running => {}
        }
        let start_now = self.clock.now();
        let mut rounds = 0_usize;
        let mut transitions = 0_usize;
        let mut decisions = 0_usize;
        let mut events = 0_usize;
        let mut reached = true;
        loop {
            let due = self.actions.next_due();
            let Some(due) = due else {
                break;
            };
            if due > target {
                break;
            }
            if rounds >= work_budget {
                reached = false;
                break;
            }
            let (round_transitions, round_decisions, round_events) = match self.process_round(due) {
                Ok(round) => round,
                Err(cause) => {
                    self.enter_fault(cause.clone(), due);
                    return Err(cause);
                }
            };
            self.transitions_total = self
                .transitions_total
                .saturating_add(round_transitions as u64);
            self.decisions_total = self.decisions_total.saturating_add(round_decisions as u64);
            self.rounds_total = self.rounds_total.saturating_add(1);
            rounds += 1;
            transitions += round_transitions;
            decisions += round_decisions;
            events += round_events;
        }
        // An empty `Setup` world may advance directly to the target and become
        // `Running`; an equal-target no-op leaves the kernel in `Setup`.
        if self.state == KernelState::Setup && target > start_now {
            self.state = KernelState::Running;
        }
        if reached {
            self.clock.advance_to(target)?;
        }
        let metrics = self.metrics();
        let nodes = metrics
            .scheduler_queue_depth
            .saturating_add(metrics.scheduler_stale_nodes);
        self.observations.queue_depth_min = Some(
            self.observations
                .queue_depth_min
                .map_or(metrics.scheduler_queue_depth, |old| {
                    old.min(metrics.scheduler_queue_depth)
                }),
        );
        self.observations.queue_nodes_min = Some(
            self.observations
                .queue_nodes_min
                .map_or(nodes, |old| old.min(nodes)),
        );
        self.observations.boundary_count = self.observations.boundary_count.saturating_add(1);
        self.observations.queue_depth_sum = self
            .observations
            .queue_depth_sum
            .saturating_add(metrics.scheduler_queue_depth as u64);
        self.observations.queue_depth_max = self
            .observations
            .queue_depth_max
            .max(metrics.scheduler_queue_depth);
        self.observations.queue_nodes_sum = self
            .observations
            .queue_nodes_sum
            .saturating_add(nodes as u64);
        self.observations.queue_nodes_max = self.observations.queue_nodes_max.max(nodes);
        Ok(KernelAdvance {
            committed_to: self.clock.now(),
            rounds,
            reached_target: reached,
            transitions,
            decisions,
            events,
        })
    }

    /// Processes one full due-instant round: folds the action runtime into the
    /// kernel's decisions/events and advances the clock to `due`.
    ///
    /// Returns the number of committed transitions, resolved decisions, and
    /// accounted high-level events, or the first real failure.
    fn process_round(&mut self, due: SimInstant) -> Result<(usize, usize, usize), KernelError> {
        let action_total_before = self.actions.events_total();
        let action_rotated_before = self.actions.events_rotated();
        let mut transitions = 0_usize;
        let mut committed_transitions: Vec<Transition> = Vec::new();
        let mut decisions = 0_usize;
        {
            let mut env = ActionEnvironment {
                persons: &mut self.persons,
                map: &self.map,
                sites: &mut self.sites,
            };
            let outcome = self.actions.advance(due, &mut env)?;
            transitions += outcome.transitions().len();
            committed_transitions.extend_from_slice(outcome.transitions());
            let resolutions = resolve_decisions(
                &mut self.actions,
                outcome.decision_requests(),
                &mut env,
                &self.weights,
                &self.perturbation,
            )?;
            for person_resolution in &resolutions {
                let person = person_resolution.person();
                let resolution = person_resolution.resolution();
                self.decisions
                    .insert(person, resolution.selection().trace().clone());
                transitions += resolution.transitions().len();
                committed_transitions.extend_from_slice(resolution.transitions());
                decisions += 1;
            }
        }
        // Upstream accounting: every event produced by the action runtime this
        // round, including any its retention buffer dropped, is counted once.
        let upstream_delta = self
            .actions
            .events_total()
            .saturating_sub(action_total_before);
        let upstream_rotated = self
            .actions
            .events_rotated()
            .saturating_sub(action_rotated_before);
        let committed_events = self.actions.drain_events();
        for event in &committed_events {
            event.validate()?;
        }
        // Validate the clock before publishing any complete-boundary diagnostics.
        self.clock.advance_to(due)?;
        self.events_total = self.events_total.saturating_add(upstream_delta);
        self.events_digest = self.actions.events_digest();
        let mut kernel_rotated = 0_u64;
        for event in committed_events {
            if self.events.len() >= self.event_buffer_capacity {
                self.events.pop_front();
                kernel_rotated += 1;
            }
            self.events.push_back(event);
        }
        self.events_rotated = self
            .events_rotated
            .saturating_add(upstream_rotated)
            .saturating_add(kernel_rotated);
        self.committed_action_metrics = self.actions.metrics();
        for transition in committed_transitions {
            self.record_observation(transition);
        }
        let events =
            usize::try_from(upstream_delta).expect("one round's allocated outcomes fit usize");
        Ok((transitions, decisions, events))
    }

    fn record_observation(&mut self, transition: Transition) {
        let Some(row) = self.observations.persons.get_mut(&transition.person()) else {
            return;
        };
        if transition.reason() == TransitionReason::Step
            || (transition.reason() == TransitionReason::Arrived
                && matches!(transition.from(), ActionState::Moving { .. })
                && matches!(transition.to(), ActionState::Moving { .. }))
        {
            row.movement_steps = row.movement_steps.saturating_add(1);
        }
        if transition.reason() == TransitionReason::Arrived
            && matches!(transition.from(), ActionState::Moving { .. })
            && matches!(transition.to(), ActionState::Moving { .. })
        {
            row.movement_phases = row.movement_phases.saturating_add(1);
        }
        if transition.reason() == TransitionReason::Completed {
            match transition.action() {
                ActionKind::Move => row.moves = row.moves.saturating_add(1),
                ActionKind::Eat => row.eats = row.eats.saturating_add(1),
                ActionKind::Sleep => row.sleeps = row.sleeps.saturating_add(1),
                ActionKind::Work => row.works = row.works.saturating_add(1),
                ActionKind::Idle => row.idles = row.idles.saturating_add(1),
            }
        }
    }

    /// Fallible read of committed observations; faulted kernels expose no partial data.
    ///
    /// # Errors
    /// Returns `KernelFaulted` when no live state may be exposed.
    pub fn observations(&self) -> Result<&KernelObservations, KernelReadError> {
        if self.state == KernelState::Faulted {
            return Err(KernelReadError::KernelFaulted);
        }
        Ok(&self.observations)
    }

    /// Returns the number of spawned persons.
    #[must_use]
    pub fn person_count(&self) -> usize {
        self.persons.person_count()
    }

    /// Returns a read-only view of the person, keyed by stable `EntityId`.
    ///
    /// # Errors
    ///
    /// Returns [`KernelReadError::KernelFaulted`] when the kernel is faulted;
    /// a genuinely unknown identity yields `Ok(None)`.
    pub fn person(&self, id: EntityId) -> Result<Option<KernelPersonView>, KernelReadError> {
        if self.state == KernelState::Faulted {
            return Err(KernelReadError::KernelFaulted);
        }
        let Some(view) = self.persons.get(id) else {
            return Ok(None);
        };
        let Some(stored_needs) = self.persons.needs(id) else {
            return Ok(None);
        };
        // Project Needs to the kernel's committed instant (ADR-0024 D4); a
        // projection failure should be unreachable on a complete boundary.
        let needs = self
            .actions
            .projected_needs(id, stored_needs, self.clock.now())
            .map_err(|_| KernelReadError::InvalidNeedsTime {
                id,
                at: self.clock.now(),
            })?;
        let (action, action_target) = self
            .actions
            .current_action(id)
            .unwrap_or((ActionKind::Idle, None));
        let state = self.actions.current(id).unwrap_or(ActionState::Idle);
        Ok(Some(KernelPersonView {
            id,
            location: view.location(),
            needs,
            action,
            action_target,
            state,
        }))
    }

    /// Returns every person's read-only view in stable `EntityId` order.
    ///
    /// # Errors
    ///
    /// Returns [`KernelReadError::KernelFaulted`] when the kernel is faulted.
    pub fn persons(&self) -> Result<Vec<KernelPersonView>, KernelReadError> {
        if self.state == KernelState::Faulted {
            return Err(KernelReadError::KernelFaulted);
        }
        let mut views = Vec::with_capacity(self.population.len());
        for id in &self.population {
            if let Some(view) = self.person(*id)? {
                views.push(view);
            }
        }
        Ok(views)
    }

    /// The earliest instant of pending scheduled work, if any.
    ///
    /// Diagnostics only: reading this never changes simulation truth.
    ///
    /// # Errors
    /// Returns [`KernelReadError::KernelFaulted`] for an incomplete live runtime.
    pub fn next_due(&mut self) -> Result<Option<SimInstant>, KernelReadError> {
        if self.state == KernelState::Faulted {
            return Err(KernelReadError::KernelFaulted);
        }
        Ok(self.actions.next_due())
    }

    /// The latest complete decision trace for a person, if one was resolved.
    ///
    /// # Errors
    ///
    /// Returns [`KernelReadError::KernelFaulted`] when the kernel is faulted;
    /// a person with no resolved trace yields `Ok(None)`.
    pub fn latest_trace(&self, id: EntityId) -> Result<Option<&DecisionTrace>, KernelReadError> {
        if self.state == KernelState::Faulted {
            return Err(KernelReadError::KernelFaulted);
        }
        Ok(self.decisions.get(&id))
    }

    /// Drains the bounded outcome-event buffer in commit order.
    pub fn drain_events(&mut self) -> Vec<EventRecord> {
        self.events.drain(..).collect()
    }

    /// Read-only kernel health metrics.
    #[must_use]
    pub fn metrics(&self) -> KernelMetrics {
        let action = self.committed_action_metrics;
        KernelMetrics {
            state: self.state,
            failed_at: self.failed_at,
            now: self.clock.now(),
            person_count: self.person_count(),
            live_actions: action.live_actions,
            pending_retries: action.pending_retries,
            live_checks: action.live_checks,
            scheduler_queue_depth: action.scheduler.scheduled_entries,
            scheduler_stale_nodes: action.scheduler.stale_nodes,
            events_buffered: self.events.len(),
            events_rotated: self.events_rotated,
            events_digest: self.events_digest,
            rounds_total: self.rounds_total,
            transitions_total: self.transitions_total,
            decisions_total: self.decisions_total,
            events_total: self.events_total,
        }
    }

    /// Read-only scheduler operation counters from the last complete runtime state.
    ///
    /// # Errors
    /// Returns `KernelFaulted` when no live state may be exposed.
    pub fn scheduler_counters(
        &self,
    ) -> Result<palimpsest_sim_scheduler::SchedulerCounters, KernelReadError> {
        if self.state == KernelState::Faulted {
            return Err(KernelReadError::KernelFaulted);
        }
        Ok(self.actions.scheduler_counters())
    }

    /// Attempted path queries, excluding any external benchmark probes.
    ///
    /// # Errors
    /// Returns `KernelFaulted` when live state cannot be exposed.
    pub fn path_query_counts(&self) -> Result<crate::PathQueryCounts, KernelReadError> {
        if self.state == KernelState::Faulted {
            return Err(KernelReadError::KernelFaulted);
        }
        Ok(self.actions.path_query_counts())
    }

    /// Read-only action statistics from the last complete runtime state.
    ///
    /// # Errors
    /// Returns `KernelFaulted` when no live state may be exposed.
    pub fn action_stats(&self) -> Result<crate::ActionStats, KernelReadError> {
        if self.state == KernelState::Faulted {
            return Err(KernelReadError::KernelFaulted);
        }
        Ok(self.actions.stats())
    }
}

#[cfg(test)]
impl WorldKernel {
    /// A test-only hook to force a fatal fault directly (ADR-0024 D3):
    /// assertion tests must not expose a production-callable injection path.
    /// `pub(crate)` so worker unit tests (CHRON-030) can fault a kernel too.
    pub(crate) fn force_fault_for_test(&mut self, cause: KernelError, failed_at: SimInstant) {
        self.enter_fault(cause, failed_at);
    }
}

#[cfg(test)]
mod tests {
    use palimpsest_sim_ai::{PerturbationSpec, Weights};

    use super::{KernelConfig, KernelError, KernelReadError, KernelState, WorldKernel};
    use palimpsest_sim_world::{LocalCoord, WorldGenConfig, WorldMap, WorldSeed};

    const SEED: u64 = 25_025;

    #[test]
    fn partial_person_completion_before_a_real_error_never_publishes_history() {
        use crate::{ActionConfig, SimDuration, SimInstant};
        use palimpsest_sim_ai::{NeedValue, Needs};
        use palimpsest_sim_world::{ActivitySite, ActivitySites, PathConfig, SiteKind};
        let duration = |value| SimDuration::from_seconds(value).unwrap();
        let map = WorldMap::generate(WorldSeed::new(SEED), WorldGenConfig::default());
        let work = origin(&map);
        let rest = LocalCoord::new(work.x(), work.y() + 2).unwrap();
        let meal = LocalCoord::new(work.x() + 2, work.y()).unwrap();
        let sites = ActivitySites::new(vec![
            ActivitySite::new(&map, work, SiteKind::Work).unwrap(),
            ActivitySite::new(&map, rest, SiteKind::Rest).unwrap(),
            ActivitySite::new(&map, meal, SiteKind::Meal).unwrap(),
        ])
        .unwrap();
        let action = ActionConfig::new(
            duration(1),
            duration(600),
            duration(i64::MAX),
            duration(1),
            duration(60),
            duration(1),
            duration(60),
            PathConfig::default(),
        )
        .unwrap();
        let config = KernelConfig::new(
            action,
            Weights::default(),
            PerturbationSpec::ZERO,
            1024,
            4096,
        )
        .unwrap();
        let mut kernel = WorldKernel::new(map, sites, config);
        let worker = kernel.spawn_person(work).unwrap();
        let sleeper = kernel.spawn_person(work).unwrap();
        kernel
            .persons
            .set_needs(
                sleeper,
                Needs::new(NeedValue::MIN, NeedValue::from_raw(95_000).unwrap()),
            )
            .unwrap();
        kernel.start_world(SimInstant::EPOCH).unwrap();
        kernel.advance(SimInstant::from_seconds(1)).unwrap();
        let before = kernel.metrics();
        assert!(kernel.advance(SimInstant::from_seconds(2)).is_err());
        // The first person's real Work completion happened in the failed round,
        // before the second person's Sleep due-time addition overflowed.
        assert_eq!(kernel.sites.site_at(work).unwrap().work().unwrap().get(), 1);
        assert_eq!(kernel.actions.events_total(), 1);
        assert_eq!(kernel.metrics().events_total, before.events_total);
        assert_eq!(kernel.metrics().events_digest, before.events_digest);
        assert_eq!(kernel.metrics().rounds_total, 1);
        assert!(kernel.drain_events().is_empty());
        assert!(kernel.person(worker).is_err());
        assert!(kernel.sites().is_err());
        assert!(kernel.next_due().is_err());
        assert_eq!(kernel.health().last_complete, SimInstant::from_seconds(1));
        assert_eq!(kernel.health().failed_at, Some(SimInstant::from_seconds(2)));
    }

    fn origin(map: &WorldMap) -> LocalCoord {
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
            .expect("walkable 3x3 block")
    }

    #[test]
    fn config_rejects_zero_budget_and_zero_capacity() {
        assert_eq!(
            KernelConfig::new(
                crate::ActionConfig::default(),
                Weights::default(),
                PerturbationSpec::ZERO,
                0,
                4096,
            ),
            Err(super::KernelConfigError::InvalidWorkBudget),
        );
        assert_eq!(
            KernelConfig::new(
                crate::ActionConfig::default(),
                Weights::default(),
                PerturbationSpec::ZERO,
                1024,
                0,
            ),
            Err(super::KernelConfigError::InvalidEventCapacity),
        );
        assert!(KernelConfig::default().work_budget() > 0);
        assert!(KernelConfig::default().event_buffer_capacity() > 0);
    }

    #[test]
    fn zero_budget_advance_is_rejected() {
        let map = WorldMap::generate(WorldSeed::new(SEED), WorldGenConfig::default());
        let origin = origin(&map);
        let mut kernel = WorldKernel::new(
            map.clone(),
            palimpsest_sim_world::ActivitySites::place_defaults(&map),
            KernelConfig::default(),
        );
        kernel.spawn_person(origin).expect("spawn");
        assert_eq!(
            kernel.advance_to(crate::SimInstant::from_seconds(1), 0),
            Err(KernelError::InvalidBudget),
        );
    }

    #[test]
    fn forward_advance_before_start_is_not_started() {
        let map = WorldMap::generate(WorldSeed::new(SEED), WorldGenConfig::default());
        let origin = origin(&map);
        let mut kernel = WorldKernel::new(
            map.clone(),
            palimpsest_sim_world::ActivitySites::place_defaults(&map),
            KernelConfig::default(),
        );
        kernel.spawn_person(origin).expect("spawn");
        assert_eq!(
            kernel.advance_to(crate::SimInstant::from_seconds(100), 1024),
            Err(KernelError::NotStarted),
        );
        // An equal-target advance is a side-effect-free no-op.
        kernel
            .advance_to(crate::SimInstant::EPOCH, 1024)
            .expect("equal target no-op");
        // start_world at the epoch transitions to Running.
        kernel.start_world(crate::SimInstant::EPOCH).expect("start");
        assert_eq!(kernel.state(), KernelState::Running);
    }

    #[test]
    fn start_world_twice_and_post_start_spawn_are_rejected() {
        let map = WorldMap::generate(WorldSeed::new(SEED), WorldGenConfig::default());
        let origin = origin(&map);
        let mut kernel = WorldKernel::new(
            map.clone(),
            palimpsest_sim_world::ActivitySites::place_defaults(&map),
            KernelConfig::default(),
        );
        kernel.spawn_person(origin).expect("spawn");
        kernel.start_world(crate::SimInstant::EPOCH).expect("start");
        assert_eq!(
            kernel.start_world(crate::SimInstant::EPOCH),
            Err(KernelError::AlreadyStarted),
        );
        assert_eq!(kernel.spawn_person(origin), Err(KernelError::NotSetup),);
    }

    #[test]
    fn a_fault_blocks_dynamic_reads_advance_and_preserves_complete_totals() {
        let map = WorldMap::generate(WorldSeed::new(SEED), WorldGenConfig::default());
        let origin = origin(&map);
        let mut kernel = WorldKernel::new(
            map.clone(),
            palimpsest_sim_world::ActivitySites::place_defaults(&map),
            KernelConfig::default(),
        );
        let person = kernel.spawn_person(origin).expect("spawn");
        kernel.start_world(crate::SimInstant::EPOCH).expect("start");
        kernel
            .advance_to(crate::SimInstant::from_seconds(500), 1024)
            .expect("advance");
        let complete_totals = (
            kernel.metrics().transitions_total,
            kernel.metrics().decisions_total,
            kernel.metrics().events_total,
        );
        let last_complete = kernel.now();

        let cause = KernelError::Action {
            source: crate::ActionError::TimeOverflow { id: person },
        };
        kernel.force_fault_for_test(cause.clone(), last_complete);

        assert_eq!(kernel.state(), KernelState::Faulted);
        let health = kernel.health();
        assert_eq!(health.last_complete, last_complete);
        assert!(health.failed_at.is_some());
        assert!(health.cause.is_some());

        assert_eq!(kernel.person(person), Err(KernelReadError::KernelFaulted),);
        assert_eq!(kernel.persons(), Err(KernelReadError::KernelFaulted));
        assert_eq!(
            kernel.latest_trace(person),
            Err(KernelReadError::KernelFaulted),
        );
        assert!(matches!(
            kernel.advance_to(crate::SimInstant::from_seconds(600), 1024),
            Err(KernelError::KernelFaulted),
        ));

        // The pre-fault complete-boundary totals survive; the failed instant is
        // not counted.
        assert_eq!(kernel.metrics().transitions_total, complete_totals.0);
        assert_eq!(kernel.metrics().decisions_total, complete_totals.1);
        assert_eq!(kernel.metrics().events_total, complete_totals.2);
    }
}
