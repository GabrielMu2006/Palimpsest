//! Narrow Godot presentation adapter for Rust-owned render snapshots.

use std::cell::Cell;
use std::time::Instant;

use godot::classes::{IRefCounted, RefCounted};
use godot::prelude::*;
use palimpsest_sim_core::{EntityId, SimClock, run_spike_workload};

/// Presentation-only bridge object exposed to Godot.
#[derive(GodotClass)]
#[class(base=RefCounted)]
struct PalimpsestBridge {
    base: Base<RefCounted>,
    snapshot_calls: Cell<u64>,
}

#[godot_api]
impl IRefCounted for PalimpsestBridge {
    fn init(base: Base<RefCounted>) -> Self {
        Self {
            base,
            snapshot_calls: Cell::new(0),
        }
    }
}

#[godot_api]
impl PalimpsestBridge {
    /// Minimal round trip used to measure the GDScript-to-Rust boundary.
    #[func]
    fn ping(&self, value: i64) -> i64 {
        let _snapshot_calls = self.snapshot_calls.get();
        value
    }

    /// Runs the shared Phase 0 workload inside the rendered Godot process.
    #[func]
    fn benchmark_spike_workload(
        &self,
        entity_count: i64,
        final_sim_second: i64,
        sample_count: i64,
    ) -> VarDictionary {
        let _snapshot_calls = self.snapshot_calls.get();
        let mut result = VarDictionary::new();
        let Ok(entities) = u64::try_from(entity_count) else {
            result.set("ok", false);
            result.set("error", "entity_count must be positive");
            return result;
        };
        let Ok(samples) = usize::try_from(sample_count) else {
            result.set("ok", false);
            result.set("error", "sample_count must be positive");
            return result;
        };
        if entities == 0
            || entities > 1_000_000
            || samples == 0
            || samples > 100
            || final_sim_second < 0
        {
            result.set("ok", false);
            result.set("error", "invalid workload limits");
            return result;
        }

        let mut elapsed = Vec::with_capacity(samples);
        for _ in 0..samples {
            let started = Instant::now();
            let Ok(metrics) = run_spike_workload(entities, final_sim_second) else {
                result.set("ok", false);
                result.set("error", "shared workload failed");
                return result;
            };
            elapsed.push(started.elapsed());
            if metrics.processed_work != entities || metrics.remaining_scheduled != 0 {
                result.set("ok", false);
                result.set("error", "shared workload invariant failed");
                return result;
            }
        }
        elapsed.sort_unstable();
        let median = elapsed[samples / 2];
        let median_ns = i64::try_from(median.as_nanos()).unwrap_or(i64::MAX);
        result.set("ok", true);
        result.set("mode", "rendered");
        result.set("entities", entity_count);
        result.set("final_sim_second", final_sim_second);
        result.set("samples", sample_count);
        result.set("median_ns", median_ns);
        result.set(
            "entity_work_per_second",
            f64::from(u32::try_from(entities).expect("validated entity limit"))
                / median.as_secs_f64(),
        );
        result
    }

    /// Returns a small immutable view model proving the bridge direction.
    #[func]
    fn render_snapshot(&self) -> VarDictionary {
        let call_count = self.snapshot_calls.get().saturating_add(1);
        self.snapshot_calls.set(call_count);
        let mut snapshot = VarDictionary::new();
        snapshot.set("schema_version", 1_i64);
        snapshot.set("source", "rust");
        snapshot.set("sim_second", SimClock::default().now().as_seconds());
        snapshot.set(
            "bridge_call_count",
            i64::try_from(call_count).unwrap_or(i64::MAX),
        );
        snapshot.set(
            "example_entity_id",
            i64::try_from(EntityId::MIN.get()).unwrap_or(1),
        );
        snapshot
    }
}

struct PalimpsestExtension;

// SAFETY: godot-rust requires this marker implementation at the single
// GDExtension registration boundary. No Simulation Core unsafe code is allowed.
#[gdextension]
unsafe impl ExtensionLibrary for PalimpsestExtension {}
