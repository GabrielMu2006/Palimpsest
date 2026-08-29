// Authored by Kimi Code (AI coding agent) — task CHRON-021 (module wiring only).
//! Headless simulation-core root crate.
//!
//! Phase 0 domain APIs are introduced by their own scoped tasks. This crate is
//! intentionally independent of Godot. Phase 1 adds the person runtime shell
//! (CHRON-021); broader game systems arrive with their own scoped tasks.

mod person;
mod spike_workload;

pub use palimpsest_sim_entity::{EntityId, EntityIdAllocator};
pub use palimpsest_sim_events::{EventId, EventRecord, SignificanceScore, Visibility};
pub use palimpsest_sim_scheduler::{ScheduleToken, Scheduled, Scheduler, SchedulerMetrics};
pub use palimpsest_sim_time::{SimClock, SimDuration, SimInstant};
pub use person::{
    Location, Person, PersonError, PersonNeeds, PersonRuntime, PersonView, StableEntityId,
};
pub use spike_workload::{SpikeRunError, SpikeRunMetrics, run_spike_workload};

#[cfg(test)]
mod tests {
    use super::{EntityId, SimClock};

    #[test]
    fn workspace_test_harness_runs() {
        assert_eq!(EntityId::MIN.get(), 1);
        assert_eq!(SimClock::default().now().as_seconds(), 0);
    }
}
