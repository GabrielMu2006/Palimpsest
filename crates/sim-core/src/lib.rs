//! Headless simulation-core root crate.
//!
//! Phase 0 domain APIs are introduced by their own scoped tasks. This crate is
//! intentionally independent of Godot and contains no game systems yet.

mod spike_workload;

pub use palimpsest_sim_entity::{EntityId, EntityIdAllocator};
pub use palimpsest_sim_events::{EventId, EventRecord, SignificanceScore, Visibility};
pub use palimpsest_sim_scheduler::{ScheduleToken, Scheduled, Scheduler, SchedulerMetrics};
pub use palimpsest_sim_time::{SimClock, SimDuration, SimInstant};
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
