//! Regression for the distinction between completing an activity and moving.
mod common;
use common::repair_fixture::RepairFixture;
use palimpsest_sim_core::{KernelConfig, SimInstant, WorldKernel};

#[test]
fn zero_distance_work_is_not_movement_and_real_arrivals_are_counted_once() {
    let fixture = RepairFixture::new();
    let mut kernel = WorldKernel::new(fixture.map, fixture.sites, KernelConfig::default());
    let id = kernel.spawn_person(fixture.origin).unwrap();
    kernel.start_world(SimInstant::EPOCH).unwrap();
    while !kernel
        .advance(SimInstant::from_seconds(3600))
        .unwrap()
        .reached_target()
    {}
    let row = kernel.observations().unwrap().persons[&id];
    assert!(row.works > 0);
    assert_eq!(row.movement_steps, 0);
    assert_eq!(row.movement_phases, 0);
    while !kernel
        .advance(SimInstant::from_seconds(172_800))
        .unwrap()
        .reached_target()
    {}
    let row = kernel.observations().unwrap().persons[&id];
    assert!(row.eats > 0 && row.sleeps > 0 && row.works > 0);
    assert!(row.movement_phases > 0);
    // All three sites differ by two or four grid steps in this fixed fixture.
    assert!(row.movement_steps >= 2 * row.movement_phases);
    assert!(row.movement_phases < row.eats + row.sleeps + row.works);
    let before = kernel.observations().unwrap().persons.clone();
    kernel.drain_events();
    assert_eq!(kernel.observations().unwrap().persons, before);
}
