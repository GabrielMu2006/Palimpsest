//! Independent V2 boundary regressions, not a replacement for V1 tests.
mod common;

use common::repair_fixture::{
    RepairFixture, action_config_with_work, at, default_weights, zero_perturbation,
};
use palimpsest_sim_core::{
    KernelConfig, KernelReadError, KernelState, RenderSnapshot, WorldKernel,
};

fn kernel(count: usize, capacity: usize) -> WorldKernel {
    let f = RepairFixture::new();
    let config = KernelConfig::new(
        palimpsest_sim_core::ActionConfig::default(),
        default_weights(),
        zero_perturbation(),
        1024,
        capacity,
    )
    .unwrap();
    let mut world = WorldKernel::new(f.map, f.sites, config);
    for _ in 0..count {
        world.spawn_person(f.origin).unwrap();
    }
    world.start_world(at(0)).unwrap();
    world
}

#[test]
fn real_fault_preserves_complete_metrics_and_blocks_live_reads() {
    let f = RepairFixture::new();
    let config = KernelConfig::new(
        action_config_with_work(i64::MAX),
        default_weights(),
        zero_perturbation(),
        1024,
        4096,
    )
    .unwrap();
    let mut world = WorldKernel::new(f.map, f.sites, config);
    let id = world.spawn_person(f.meal).unwrap();
    world.start_world(at(0)).unwrap();
    world.advance(at(1)).unwrap();
    assert_eq!(world.health().last_complete, world.now());
    let before = world.metrics();
    assert!(world.advance(at(2)).is_err());
    let after = world.metrics();
    assert_eq!(after.state, KernelState::Faulted);
    assert_eq!(after.failed_at, Some(at(2)));
    assert_eq!(after.now, at(1));
    assert_eq!(after.scheduler_queue_depth, before.scheduler_queue_depth);
    assert_eq!(after.live_actions, before.live_actions);
    assert_eq!(after.rounds_total, before.rounds_total);
    assert_eq!(after.transitions_total, before.transitions_total);
    assert_eq!(after.events_total, before.events_total);
    assert_eq!(after.events_digest, before.events_digest);
    assert_eq!(world.next_due(), Err(KernelReadError::KernelFaulted));
    assert_eq!(world.sites().err(), Some(KernelReadError::KernelFaulted));
    assert_eq!(world.person(id), Err(KernelReadError::KernelFaulted));
    assert!(world.persons().is_err());
    assert!(world.latest_trace(id).is_err());
    assert!(RenderSnapshot::from_kernel(&world).is_err());
    assert!(world.advance(at(3)).is_err());
    assert!(world.spawn_person(f.origin).is_err());
    assert!(world.start_world(at(0)).is_err());
    assert_eq!(world.health().last_complete, at(1));
}

#[test]
fn generation_counts_include_upstream_rotation_at_all_capacity_edges() {
    for count in [4095, 4096, 4097] {
        let mut world = kernel(count, 4096);
        let advance = world.advance(at(1801)).unwrap();
        assert!(advance.reached_target());
        assert_eq!(advance.events(), count);
        let metrics = world.metrics();
        assert_eq!(metrics.events_total, count as u64);
        assert_eq!(metrics.events_buffered, count.min(4096));
        assert_eq!(metrics.events_rotated, count.saturating_sub(4096) as u64);
        let delivered = world.drain_events().len() as u64;
        assert_eq!(
            metrics.events_total,
            delivered + world.metrics().events_rotated
        );
    }
}

fn independent_digest(events: &[palimpsest_sim_events::EventRecord]) -> u64 {
    // Independent oracle: canonical EventRecord serde field order; ordered metadata.
    let mut value = 14_695_981_039_346_656_037_u64;
    for event in events {
        let bytes = serde_json::to_vec(event).unwrap();
        for byte in (bytes.len() as u64).to_le_bytes().iter().chain(&bytes) {
            value ^= u64::from(*byte);
            value = value.wrapping_mul(1_099_511_628_211);
        }
    }
    value
}

#[test]
fn digest_matches_independent_oracle_and_is_not_a_retained_buffer_hash() {
    let mut baseline = kernel(3, 4096);
    baseline.advance(at(3602)).unwrap();
    let expected = baseline.metrics().events_digest;
    let events = baseline.drain_events();
    assert_eq!(expected, independent_digest(&events));
    let mut reversed = events.clone();
    reversed.reverse();
    assert_ne!(expected, independent_digest(&reversed));
    let mut altered = events.clone();
    altered[0].insert_metadata("probe", serde_json::json!(1));
    assert_ne!(expected, independent_digest(&altered));
    for capacity in [1, 4096] {
        for drain in [false, true] {
            let mut world = kernel(3, capacity);
            let mut delivered = 0_u64;
            let mut counted = 0_usize;
            loop {
                let advance = world.advance_to(at(3602), 1).unwrap();
                counted += advance.events();
                if drain {
                    delivered += world.drain_events().len() as u64;
                }
                if advance.reached_target() {
                    break;
                }
            }
            let m = world.metrics();
            assert_eq!(m.events_total, counted as u64);
            assert_eq!(
                m.events_total,
                delivered + m.events_buffered as u64 + m.events_rotated
            );
            assert_eq!(m.events_digest, expected);
        }
    }
}

#[test]
fn health_complete_marker_tracks_targets_even_without_due_work() {
    let mut empty = kernel(0, 4096);
    empty.advance(at(100)).unwrap();
    assert_eq!(empty.health().last_complete, at(100));
    let mut populated = kernel(1, 4096);
    populated.advance(at(100)).unwrap();
    assert_eq!(populated.health().last_complete, at(100));
}
