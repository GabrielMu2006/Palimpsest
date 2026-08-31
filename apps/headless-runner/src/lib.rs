//! Deterministic Phase1 headless runner and representative benchmark adapters.
use palimpsest_sim_core::RenderSnapshot;
use serde::Serialize;

/// Representative Phase1 benchmark fixtures and observations.
pub mod micro_bench;

/// Actual committed kernel work; no finite dummy workload is exposed.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct RunMetrics {
    /// Real persons in the final world.
    pub entities: u64,
    /// Reached simulation boundary.
    pub final_sim_second: i64,
    /// Committed action transitions (not old dummy updates).
    pub processed_work: u64,
    /// Validated high-level outcome events.
    pub generated_events: u64,
    /// Live future work; a running world normally has a nonempty queue.
    pub remaining_scheduled: usize,
    /// Render-state/work comparison only, not a persisted world hash.
    pub snapshot_hash: String,
}

/// Rejected configuration or real kernel failure.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RunError {
    /// Negative simulation horizon.
    NegativeFinalTime(i64),
    /// Zero or unrepresentable population.
    InvalidPopulation(u64),
    /// Fixture, advancement, or snapshot error from the real kernel.
    Kernel(String),
}
impl std::fmt::Display for RunError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Phase1 headless run failed: {self:?}")
    }
}
impl std::error::Error for RunError {}

/// Run the seed42 reachable fixture to an explicit nonnegative boundary.
/// # Errors
/// Rejects invalid population/time, unreachable fixtures, kernel or snapshot errors.
pub fn run(persons: u64, seconds: i64) -> Result<RunMetrics, RunError> {
    if seconds < 0 {
        return Err(RunError::NegativeFinalTime(seconds));
    }
    let count = usize::try_from(persons).map_err(|_| RunError::InvalidPopulation(persons))?;
    if count == 0 {
        return Err(RunError::InvalidPopulation(persons));
    }
    // A zero-duration observation is legal; fixture preparation itself needs a positive config horizon.
    let mut kernel = micro_bench::build_fixture(count, seconds.max(1)).map_err(RunError::Kernel)?;
    micro_bench::advance_to_target(&mut kernel, seconds).map_err(RunError::Kernel)?;
    let snapshot =
        RenderSnapshot::from_kernel(&kernel).map_err(|e| RunError::Kernel(e.to_string()))?;
    let metrics = kernel.metrics();
    Ok(RunMetrics {
        entities: persons,
        final_sim_second: kernel.now().as_seconds(),
        processed_work: metrics.transitions_total,
        generated_events: metrics.events_total,
        remaining_scheduled: metrics.scheduler_queue_depth,
        snapshot_hash: snapshot.diagnostic_hash().to_string(),
    })
}
