use std::collections::HashMap;
use std::hint::black_box;
use std::time::Instant;

use bevy_ecs::prelude::{Component, Entity, World};
use palimpsest_sim_core::{EntityId, EntityIdAllocator};
use serde::Serialize;

#[derive(Component)]
struct StableId(EntityId);

#[derive(Component)]
struct DummyState {
    energy: u32,
    updates: u64,
}

#[derive(Serialize)]
struct ResultMetrics {
    entities: usize,
    steps: u64,
    elapsed_ns: u128,
    steps_per_second: f64,
    entity_updates_per_second: f64,
    stable_mapping_entries: usize,
    checksum: u64,
}

fn main() {
    let (entity_count, steps) = parse_args();
    let mut world = World::new();
    let mut allocator = EntityIdAllocator::default();
    let mut stable_to_runtime = HashMap::<EntityId, Entity>::with_capacity(entity_count);

    for _ in 0..entity_count {
        let stable = allocator.allocate().expect("entity ID capacity");
        let runtime = world
            .spawn((
                StableId(stable),
                DummyState {
                    energy: 100,
                    updates: 0,
                },
            ))
            .id();
        assert!(stable_to_runtime.insert(stable, runtime).is_none());
    }

    let started = Instant::now();
    let mut query = world.query::<(&StableId, &mut DummyState)>();
    for step in 0..steps {
        for (stable, mut state) in query.iter_mut(&mut world) {
            let step_delta = u32::try_from(step % 97).expect("small modulo fits u32");
            state.energy = state.energy.wrapping_add(1).wrapping_add(step_delta);
            state.updates = state
                .updates
                .checked_add(1)
                .expect("update counter capacity");
            black_box(stable.0);
            black_box(state.energy);
        }
    }
    let elapsed = started.elapsed();

    let mut checksum = 0_u64;
    for (stable, state) in query.iter(&world) {
        let runtime = stable_to_runtime
            .get(&stable.0)
            .expect("stable mapping exists");
        assert!(world.get_entity(*runtime).is_ok());
        assert_eq!(state.updates, steps);
        checksum = checksum.wrapping_add(u64::from(state.energy));
    }
    let updates = u64::try_from(entity_count)
        .expect("entity count fits u64")
        .saturating_mul(steps);
    let seconds = elapsed.as_secs_f64();
    let steps_f64 = f64::from(u32::try_from(steps).expect("benchmark steps must fit u32"));
    let updates_f64 = f64::from(u32::try_from(updates).expect("benchmark updates must fit u32"));
    let metrics = ResultMetrics {
        entities: entity_count,
        steps,
        elapsed_ns: elapsed.as_nanos(),
        steps_per_second: if seconds == 0.0 {
            0.0
        } else {
            steps_f64 / seconds
        },
        entity_updates_per_second: if seconds == 0.0 {
            0.0
        } else {
            updates_f64 / seconds
        },
        stable_mapping_entries: stable_to_runtime.len(),
        checksum,
    };
    println!(
        "{}",
        serde_json::to_string(&metrics).expect("serialize metrics")
    );
}

fn parse_args() -> (usize, u64) {
    let mut args = std::env::args().skip(1);
    let entities = args.next().map_or(10_000, |value| {
        value.parse().expect("entities must be usize")
    });
    let steps = args
        .next()
        .map_or(1_000, |value| value.parse().expect("steps must be u64"));
    (entities, steps)
}
