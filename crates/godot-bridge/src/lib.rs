//! Narrow Godot presentation adapter for Rust-owned render snapshots.

mod frames;

use std::cell::Cell;
use std::time::Instant;

use godot::classes::{IRefCounted, RefCounted};
use godot::prelude::*;
use palimpsest_sim_core::{
    CommandSequence, EntityId, KernelConfig, SimClock, SimInstant, SimulationWorker, WorldKernel,
    run_spike_workload,
};
use palimpsest_sim_world::WorldSeed;

use crate::frames::{
    command_status_wire, encode_entity_ids_le, frame_from_snapshot, parse_command, sequence_to_wire,
};

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

/// The Phase 1 micro-world presentation object (CHRON-031, ADR-0026).
///
/// Owns the CHRON-030 [`SimulationWorker`]; every per-frame read is one
/// batched `snapshot_frame()` dictionary, and every client intent goes through
/// the bounded worker command path. The bridge exposes no kernel mutation
/// outside that path, and the Scene Tree never stores simulation truth.
#[derive(GodotClass)]
#[class(base=RefCounted)]
struct PalimpsestMicroWorld {
    base: Base<RefCounted>,
    worker: Option<SimulationWorker>,
}

#[godot_api]
impl IRefCounted for PalimpsestMicroWorld {
    fn init(base: Base<RefCounted>) -> Self {
        Self { base, worker: None }
    }
}

#[godot_api]
impl PalimpsestMicroWorld {
    /// Maximum population this presentation task creates (CHRON-031 scope).
    const MAX_PERSONS: i64 = 100;

    /// Creates the deterministic micro world (decimal-u64 seed, lossless) and
    /// starts its worker paused with the initial snapshot published.
    // godot-rust #[func] does not support reference parameters.
    #[allow(clippy::needless_pass_by_value)]
    #[func]
    fn create_world(&mut self, seed_text: GString, persons: i64) -> VarDictionary {
        let mut result = VarDictionary::new();
        if self.worker.is_some() {
            result.set("ok", false);
            result.set("error", "already_created");
            return result;
        }
        let Some(seed) = seed_text.to_string().parse::<u64>().ok() else {
            result.set("ok", false);
            result.set("error", "invalid_seed");
            return result;
        };
        if !(1..=Self::MAX_PERSONS).contains(&persons) {
            result.set("ok", false);
            result.set("error", "invalid_persons");
            return result;
        }
        let mut kernel = WorldKernel::from_world(WorldSeed::new(seed), KernelConfig::default());
        let Ok(spawns) = palimpsest_sim_core::resolve_spawns(
            kernel.map(),
            kernel.sites().expect("setup sites are readable"),
            usize::try_from(persons).expect("validated positive population"),
        ) else {
            result.set("ok", false);
            result.set("error", "no_reachable_spawn");
            return result;
        };
        for origin in spawns {
            if kernel.spawn_person(origin).is_err() {
                result.set("ok", false);
                result.set("error", "spawn_failed");
                return result;
            }
        }
        if kernel.start_world(SimInstant::EPOCH).is_err() {
            result.set("ok", false);
            result.set("error", "start_failed");
            return result;
        }
        if let Ok(worker) = SimulationWorker::new(kernel) {
            self.worker = Some(worker);
            result.set("ok", true);
        } else {
            result.set("ok", false);
            result.set("error", "worker_start_failed");
        }
        result
    }

    /// Benchmark-only read; consume after a successful `AdvanceTo` acknowledgement.
    #[func]
    fn snapshot_diagnostic_hash(&self) -> GString {
        self.worker.as_ref().map_or_else(GString::new, |worker| {
            GString::from(
                worker
                    .latest_snapshot()
                    .diagnostic_hash()
                    .to_string()
                    .as_str(),
            )
        })
    }

    /// One batched read of the latest complete published snapshot plus the
    /// worker status (ADR-0026 §1). Read-only; never blocks the kernel
    /// mid-tick.
    #[func]
    fn snapshot_frame(&self) -> VarDictionary {
        let mut frame = VarDictionary::new();
        let Some(worker) = &self.worker else {
            frame.set("ok", false);
            frame.set("error", "world_not_created");
            return frame;
        };
        let conversion_started = std::time::Instant::now();
        let observed = worker.observe();
        let data = frame_from_snapshot(&observed.publication.snapshot, &observed.status);
        let micros =
            |duration: std::time::Duration| i64::try_from(duration.as_micros()).unwrap_or(i64::MAX);
        frame.set(
            "snapshot_age_us",
            micros(observed.publication.built_from.elapsed()),
        );
        frame.set(
            "snapshot_build_us",
            micros(
                observed
                    .publication
                    .published_at
                    .duration_since(observed.publication.built_from),
            ),
        );

        frame.set("ok", true);
        frame.set("schema_version", data.schema_version);
        frame.set("sim_second", data.sim_second);
        debug_assert_eq!(observed.publication.sequence, observed.status.publications);
        frame.set("publications", data.publications);
        frame.set("terrain", &PackedByteArray::from(data.terrain.as_slice()));
        let mut site_x = PackedInt32Array::new();
        let mut site_y = PackedInt32Array::new();
        let mut site_kind = PackedInt32Array::new();
        for (x, y, kind) in &data.sites {
            site_x.push(*x);
            site_y.push(*y);
            site_kind.push(*kind);
        }
        frame.set("site_x", &site_x);
        frame.set("site_y", &site_y);
        frame.set("site_kind", &site_kind);
        frame.set(
            "person_id",
            &PackedByteArray::from(
                encode_entity_ids_le(data.persons.iter().map(|person| person.id)).as_slice(),
            ),
        );
        let mut person_x = PackedInt32Array::new();
        let mut person_y = PackedInt32Array::new();
        let mut person_action = PackedInt32Array::new();
        let mut person_state = PackedInt32Array::new();
        let mut person_target_x = PackedInt32Array::new();
        let mut person_target_y = PackedInt32Array::new();
        for person in &data.persons {
            person_x.push(person.x);
            person_y.push(person.y);
            person_action.push(person.action);
            person_state.push(person.state);
            person_target_x.push(person.target_x);
            person_target_y.push(person.target_y);
        }
        frame.set("person_x", &person_x);
        frame.set("person_y", &person_y);
        frame.set("person_action", &person_action);
        frame.set("person_state", &person_state);
        frame.set("person_target_x", &person_target_x);
        frame.set("person_target_y", &person_target_y);

        let mut metrics = VarDictionary::new();
        metrics.set("person_count", data.metrics.person_count);
        metrics.set("scheduler_queue_depth", data.metrics.scheduler_queue_depth);
        metrics.set("events_committed", data.metrics.events_committed);
        metrics.set("events_buffered", data.metrics.events_buffered);
        metrics.set("buffer_rotations", data.metrics.buffer_rotations);
        metrics.set("live_actions", data.metrics.live_actions);
        metrics.set("rounds_total", data.metrics.rounds_total);
        metrics.set("transitions_total", data.metrics.transitions_total);
        metrics.set("decisions_total", data.metrics.decisions_total);
        frame.set("metrics", &metrics);

        let mut worker_dict = VarDictionary::new();
        worker_dict.set("phase", data.worker.phase);
        worker_dict.set("speed", data.worker.speed);
        worker_dict.set("committed", data.worker.committed);
        worker_dict.set("publications", data.worker.publications);
        worker_dict.set("commands_applied", data.worker.commands_applied);
        worker_dict.set("commands_rejected", data.worker.commands_rejected);
        worker_dict.set("queue_depth", data.worker.queue_depth);
        worker_dict.set("max_queue_depth", data.worker.max_queue_depth);
        worker_dict.set("faulted", data.worker.faulted);
        frame.set("worker", &worker_dict);
        frame.set("bridge_conversion_us", micros(conversion_started.elapsed()));
        frame
    }

    /// Submits one command to the bounded worker queue (ADR-0026 §2).
    /// `ok=false` means never enqueued; `ok=true` returns the sequence whose
    /// final outcome is observable through `command_status`.
    // godot-rust #[func] does not support reference parameters.
    #[allow(clippy::needless_pass_by_value)]
    #[func]
    fn command(&self, command_type: GString, value: i64) -> VarDictionary {
        let mut result = VarDictionary::new();
        let Some(worker) = &self.worker else {
            result.set("ok", false);
            result.set("error", "world_not_created");
            return result;
        };
        let parsed = parse_command(&command_type.to_string(), value);
        let Ok(command) = parsed else {
            result.set("ok", false);
            result.set("error", parsed.err().unwrap_or("unknown_command"));
            return result;
        };
        match worker.submit(command) {
            Ok(sequence) => {
                result.set("ok", true);
                result.set("sequence", sequence_to_wire(sequence));
            }
            Err(error) => {
                result.set("ok", false);
                result.set("error", error.to_string());
            }
        }
        result
    }

    /// The observable status of one submitted command sequence.
    #[func]
    fn command_status(&self, sequence: i64) -> VarDictionary {
        let mut result = VarDictionary::new();
        let Some(worker) = &self.worker else {
            result.set("status", "unknown");
            return result;
        };
        let Ok(raw) = u64::try_from(sequence) else {
            result.set("status", "unknown");
            return result;
        };
        let (status, outcome, committed_to) =
            command_status_wire(&worker.command_status(CommandSequence::new(raw)));
        result.set("status", status);
        if let Some(outcome) = outcome {
            result.set("outcome", outcome);
        }
        if let Some(committed_to) = committed_to {
            result.set("committed_to", committed_to);
        }
        result
    }
}

struct PalimpsestExtension;

// SAFETY: godot-rust requires this marker implementation at the single
// GDExtension registration boundary. No Simulation Core unsafe code is allowed.
#[gdextension]
unsafe impl ExtensionLibrary for PalimpsestExtension {}
