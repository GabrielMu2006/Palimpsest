//! Deterministic Phase 0 headless simulation harness.

pub use palimpsest_sim_core::{
    SpikeRunError as RunError, SpikeRunMetrics as RunMetrics, run_spike_workload as run,
};

/// Representative Phase 1 benchmark fixtures and observations.
pub mod micro_bench;
