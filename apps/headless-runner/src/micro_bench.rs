//! Representative fixture and timing observations (ADR-0029).
#![allow(clippy::cast_precision_loss)]
use palimpsest_sim_core::{ChaosConfig, RenderSnapshot, WorldKernel, build_chaos_kernel};
use std::time::Instant;

/// Build the identical seed42 fixture at every scale.
/// # Errors
/// Rejects invalid inputs or insufficient reachable cells.
pub fn build_fixture(persons: usize, seconds: i64) -> Result<WorldKernel, String> {
    build_chaos_kernel(&ChaosConfig {
        seed: 42,
        person_count: persons,
        years: 1,
        sim_seconds_per_year: seconds,
    })
    .map_err(|e| e.to_string())
}

/// Advance only complete bounded kernel rounds.
/// # Errors
/// Propagates kernel faults or bounded progress failures.
pub fn advance_to_target(kernel: &mut WorldKernel, seconds: i64) -> Result<(), String> {
    let target = palimpsest_sim_core::SimInstant::from_seconds(seconds);
    let mut calls = 0usize;
    loop {
        let before = kernel.now();
        let advance = kernel.advance(target).map_err(|e| e.to_string())?;
        if kernel.now() <= before && !advance.reached_target() {
            return Err("advance made no progress".into());
        }
        calls += 1;
        if calls > 2_000_000 {
            return Err("advance budget exceeded".into());
        }
        if advance.reached_target() {
            return Ok(());
        }
    }
}

/// Validate actual live final state and required work.
/// # Errors
/// Rejects missing work or violated state invariants.
pub fn validate_kernel(
    kernel: &mut WorldKernel,
    persons: usize,
    seconds: i64,
) -> Result<(), String> {
    if kernel.now().as_seconds() != seconds || kernel.person_count() != persons {
        return Err("final population or time mismatch".into());
    }
    let observations = kernel.observations().map_err(|e| e.to_string())?;
    if observations.persons.len() != persons {
        return Err("observation population mismatch".into());
    }
    if observations
        .persons
        .values()
        .any(|row| row.eats == 0 || row.sleeps == 0 || row.works == 0 || row.movement_phases == 0)
    {
        return Err("required per-person work incomplete".into());
    }
    let views = kernel.persons().map_err(|e| e.to_string())?;
    if views.len() != persons || views.windows(2).any(|w| w[0].id() >= w[1].id()) {
        return Err("person identities/count invalid".into());
    }
    for person in views {
        if let Some(error) = palimpsest_sim_core::needs_in_bounds(person.id(), person.needs()) {
            return Err(error.to_string());
        }
        let walkable = |c: palimpsest_sim_world::LocalCoord| {
            kernel
                .map()
                .local()
                .get(c.x(), c.y())
                .is_some_and(|t| t.is_walkable())
        };
        if !walkable(person.location()) || person.action_target().is_some_and(|c| !walkable(c)) {
            return Err("person/target terrain invalid".into());
        }
    }
    let metrics = kernel.metrics();
    let (depth_limit, chaos_nodes_limit) = palimpsest_sim_core::queue_limits(persons);
    // Small smoke populations still inherit the scheduler's 64-node compaction floor.
    // The formal >=100-person bound remains the existing 8N chaos bound.
    let nodes_limit = chaos_nodes_limit.max(4 * persons + 64);
    if metrics.scheduler_queue_depth + metrics.scheduler_stale_nodes > nodes_limit {
        return Err("queue nodes bound exceeded".into());
    }
    if metrics.scheduler_queue_depth > depth_limit {
        return Err("queue bound exceeded".into());
    }
    if kernel
        .next_due()
        .map_err(|e| e.to_string())?
        .is_none_or(|next| next <= kernel.now())
    {
        return Err("missing future scheduled work".into());
    }
    Ok(())
}

/// Measure one prepared-to-final interval; post-validation and probes are separate.
/// # Errors
/// Rejects a fixture, advance or correctness failure rather than filtering it.
pub fn measure_scale(persons: usize, seconds: i64) -> Result<serde_json::Value, String> {
    use serde_json::json;
    let mut kernel = build_fixture(persons, seconds)?;
    let prepared_people = kernel.persons().map_err(|e| e.to_string())?;
    let initial = kernel.metrics();
    let initial_q = kernel.path_query_counts().map_err(|e| e.to_string())?;
    let initial_c = kernel.scheduler_counters().map_err(|e| e.to_string())?;
    let started = Instant::now();
    advance_to_target(&mut kernel, seconds)?;
    let elapsed_ns = started.elapsed().as_nanos();
    validate_kernel(&mut kernel, persons, seconds)?;
    let metrics = kernel.metrics();
    let started = Instant::now();
    let snapshot = RenderSnapshot::from_kernel(&kernel).map_err(|e| e.to_string())?;
    let snapshot_build_ns = started.elapsed().as_nanos();
    let started = Instant::now();
    let bytes = serde_json::to_vec(&snapshot).map_err(|e| e.to_string())?;
    let snapshot_serialize_ns = started.elapsed().as_nanos();
    let q = kernel.path_query_counts().map_err(|e| e.to_string())?;
    let c = kernel.scheduler_counters().map_err(|e| e.to_string())?;
    let work = json!({"rounds":metrics.rounds_total-initial.rounds_total,"decisions":metrics.decisions_total-initial.decisions_total,
        "transitions":metrics.transitions_total-initial.transitions_total,"events":metrics.events_total-initial.events_total,
        "candidate_queries":q.candidate_queries-initial_q.candidate_queries,"execution_queries":q.execution_queries-initial_q.execution_queries,
        "scheduler_enqueued":c.enqueued-initial_c.enqueued,"scheduler_dequeued":c.dequeued-initial_c.dequeued,
        "scheduler_cancelled":c.cancelled-initial_c.cancelled,"scheduler_rescheduled":c.rescheduled-initial_c.rescheduled});
    let observations = kernel.observations().map_err(|e| e.to_string())?;
    let count = persons.min(64);
    let mut calls = 0;
    let mut successes = 0;
    let mut lengths = Vec::new();
    let started = Instant::now();
    for i in 0..count {
        let origin = prepared_people[i * persons / count].location();
        for site in kernel
            .sites()
            .map_err(|e| e.to_string())?
            .sites_of(palimpsest_sim_world::SiteKind::Work)
        {
            if site.kind() != palimpsest_sim_world::SiteKind::Work {
                continue;
            }
            calls += 1;
            let target = site.coord();
            if let Ok(path) = palimpsest_sim_world::find_path(
                kernel.map().local(),
                (origin.x(), origin.y()),
                (target.x(), target.y()),
                palimpsest_sim_world::TerrainKind::is_walkable,
                palimpsest_sim_world::PathConfig::default(),
            ) {
                successes += 1;
                lengths.push(path.len());
                break;
            }
        }
    }
    let probe_ns = started.elapsed().as_nanos();
    if successes != count {
        return Err("path probe did not reach work sites".into());
    }
    Ok(
        json!({"scale":persons,"seconds":seconds,"elapsed_ns":elapsed_ns,"work":work,
        "snapshot_build_ns":snapshot_build_ns,"snapshot_serialize_ns":snapshot_serialize_ns,
        "snapshot_bytes":bytes.len(),"snapshot_hash":snapshot.diagnostic_hash().to_string(),
        "terrain_json_bytes":serde_json::to_vec(snapshot.terrain()).map_err(|e|e.to_string())?.len(),
        "persons_json_bytes":serde_json::to_vec(snapshot.persons()).map_err(|e|e.to_string())?.len(),
        "snapshot_metrics":snapshot.metrics(),"events_digest":metrics.events_digest,
        "queue":{"boundary_count":observations.boundary_count,"depth_min":observations.queue_depth_min,"depth_max":observations.queue_depth_max,
            "depth_sum":observations.queue_depth_sum,"nodes_min":observations.queue_nodes_min,"nodes_max":observations.queue_nodes_max,"nodes_sum":observations.queue_nodes_sum},
        "path_probe":{"calls":calls,"successes":successes,"lengths":lengths,"elapsed_ns":probe_ns,"method":"isolated prepared positions to Work sites; not integrated pathfinding wall share"}}),
    )
}
