// Authored by Kimi Code (AI coding agent) — task CHRON-021 (module wiring only).
// Extended by Kimi Code (AI coding agent) — task CHRON-027.
// Extended by Kimi Code (AI coding agent) — task CHRON-030.
//! Headless simulation-core root crate.
//!
//! Phase 0 domain APIs are introduced by their own scoped tasks. This crate is
//! intentionally independent of Godot. Phase 1 adds the person runtime shell
//! (CHRON-021), the action execution state machine (CHRON-027, ADR-0021), and
//! the simulation worker command bridge (CHRON-030, ADR-0015 supplement);
//! broader game systems arrive with their own scoped tasks.

mod actions;
mod chaos;
mod kernel;
mod person;
mod render;
mod spike_workload;
mod worker;

pub use actions::{
    ActionConfig, ActionEnvironment, ActionError, ActionRuntime, ActionRuntimeMetrics, ActionState,
    ActionStats, AdvanceOutcome, CancelReason, DecisionDriveError, DecisionReason, DecisionRequest,
    DecisionResolution, EAT_RELIEF, EVENT_BUFFER_CAPACITY, PathQueryCounts, PersonResolution,
    REST_RELIEF, Transition, TransitionReason, decide_and_start, resolve_decision,
    resolve_decisions, run_until,
};
pub use chaos::{
    ActionCounts, ChaosCheckpoint, ChaosConfig, ChaosError, ChaosMeasurement, ChaosReport,
    DaySample, MAX_ADVANCE_CALLS_PER_DAY, MAX_STALLED_ADVANCE_CALLS, PerPersonCompletionRow,
    PerPersonCompletions, SECONDS_PER_DAY, SECONDS_PER_YEAR, TEN_YEARS_SECONDS, actor_resolves,
    build_chaos_kernel, needs_in_bounds, queue_bounded, queue_limits, resolve_spawns, run_chaos,
    run_chaos_observed,
};
pub use kernel::{
    DEFAULT_EVENT_BUFFER_CAPACITY, DEFAULT_WORK_BUDGET, KernelAdvance, KernelConfig,
    KernelConfigError, KernelError, KernelHealth, KernelMetrics, KernelObservations,
    KernelPersonView, KernelReadError, KernelState, PersonObservations, WorldKernel,
};
pub use palimpsest_sim_entity::{EntityId, EntityIdAllocator};
pub use palimpsest_sim_events::{EventId, EventRecord, SignificanceScore, Visibility};
pub use palimpsest_sim_scheduler::{ScheduleToken, Scheduled, Scheduler, SchedulerMetrics};
pub use palimpsest_sim_time::{SimClock, SimDuration, SimInstant};
pub use person::{
    Location, Person, PersonError, PersonNeeds, PersonRuntime, PersonView, StableEntityId,
};
pub use render::{
    ActivitySiteRender, PersonRender, RENDER_SCHEMA_VERSION, RenderError, RenderMetrics,
    RenderSnapshot, TerrainBatch,
};
pub use spike_workload::{SpikeRunError, SpikeRunMetrics, run_spike_workload};
pub use worker::{
    ACK_LOG_CAPACITY, COMMAND_QUEUE_CAPACITY, CommandAck, CommandOutcome, CommandSequence,
    CommandStatus, MAX_STEP_STEPS, SimulationWorker, SpeedMultiplier, WorkerCommand, WorkerError,
    WorkerObservation, WorkerPhase, WorkerPublication, WorkerStatus,
};

#[cfg(test)]
mod tests {
    use super::{EntityId, SimClock};

    #[test]
    fn workspace_test_harness_runs() {
        assert_eq!(EntityId::MIN.get(), 1);
        assert_eq!(SimClock::default().now().as_seconds(), 0);
    }
}
