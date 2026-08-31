// Authored by opencode (AI coding agent) — task CHRON-029.
//! Render snapshot DTO tests (CHRON-029, ADR-0023).
//!
//! These exercise the immutable, versioned snapshot contract: schema version,
//! structural bounds, non-zero/unique/ascending stable identity, the absence
//! of any mutable accessors or ECS handles, the headless diagnostic serde
//! boundary, and fidelity to the kernel's committed tick.

use palimpsest_sim_ai::ActionKind;
use palimpsest_sim_core::{
    EntityId, KernelConfig, KernelPersonView, RENDER_SCHEMA_VERSION, RenderSnapshot, SimInstant,
    WorldKernel,
};
use palimpsest_sim_world::{
    LOCAL_GRID_CELL_COUNT, LOCAL_GRID_HEIGHT, LOCAL_GRID_WIDTH, WorldGenConfig, WorldMap, WorldSeed,
};

/// Locked fixture seed (ADR-0018 reference context).
const FIXTURE_SEED: u64 = 25_025;
/// One simulation day in seconds.
const DAY: i64 = 86_400;

fn at(value: i64) -> SimInstant {
    SimInstant::from_seconds(value)
}

fn walkable_block_origin(map: &WorldMap) -> palimpsest_sim_world::LocalCoord {
    map.local()
        .coords()
        .find(|origin| {
            (0..3).all(|dy| {
                (0..3).all(|dx| {
                    palimpsest_sim_world::LocalCoord::new(origin.x() + dx, origin.y() + dy)
                        .is_some_and(|coord| {
                            map.local()
                                .get(coord.x(), coord.y())
                                .is_some_and(|kind| kind.is_walkable())
                        })
                })
            })
        })
        .expect("spawn clearing contains a 3x3 walkable block")
}

fn seeded_kernel(persons: usize) -> WorldKernel {
    let mut kernel = WorldKernel::from_world(WorldSeed::new(FIXTURE_SEED), KernelConfig::default());
    let map = WorldMap::generate(WorldSeed::new(FIXTURE_SEED), WorldGenConfig::default());
    let origin = walkable_block_origin(&map);
    for _ in 0..persons {
        kernel
            .spawn_person(origin)
            .expect("identity capacity for the fixture population");
    }
    kernel
}

fn spin_to(kernel: &mut WorldKernel, target: SimInstant) {
    loop {
        let advance = kernel.advance(target).expect("bounded advance succeeds");
        if advance.reached_target() {
            return;
        }
    }
}

#[test]
fn snapshot_exposes_the_documented_schema_version() {
    let kernel = seeded_kernel(1);
    let snapshot = RenderSnapshot::from_kernel(&kernel).expect("complete-boundary snapshot");
    assert_eq!(snapshot.schema_version(), RENDER_SCHEMA_VERSION);
    assert_eq!(snapshot.sim_second(), at(0));
}

#[test]
fn snapshot_batches_the_full_local_tile_grid() {
    let kernel = seeded_kernel(1);
    let snapshot = RenderSnapshot::from_kernel(&kernel).expect("complete-boundary snapshot");
    assert_eq!(snapshot.terrain().width(), LOCAL_GRID_WIDTH);
    assert_eq!(snapshot.terrain().height(), LOCAL_GRID_HEIGHT);
    assert_eq!(snapshot.terrain().cells().len(), LOCAL_GRID_CELL_COUNT);
    assert!(snapshot.validate().is_ok());
}

#[test]
fn person_batch_carries_stable_identity_and_current_action() {
    let mut kernel = seeded_kernel(4);
    kernel.start_world(at(0)).expect("start");
    spin_to(&mut kernel, at(DAY));
    let snapshot = RenderSnapshot::from_kernel(&kernel).expect("complete-boundary snapshot");
    assert_eq!(snapshot.person_count(), 4);
    let mut previous: Option<EntityId> = None;
    for person in snapshot.persons() {
        assert_ne!(person.person_id().get(), 0, "entity id is never zero");
        if let Some(prev) = previous {
            assert!(person.person_id() > prev, "batch is ascending by EntityId");
        }
        previous = Some(person.person_id());
    }
    // Every presented person matches the kernel view (fidelity).
    let views: Vec<KernelPersonView> = kernel
        .persons()
        .expect("running read")
        .into_iter()
        .collect();
    assert_eq!(views.len(), snapshot.person_count());
    for pair in views.iter().zip(snapshot.persons()) {
        let id = pair.0.id();
        assert_eq!(pair.1.person_id(), id);
        assert_eq!(pair.1.tile(), pair.0.location());
        assert_eq!(pair.1.action(), pair.0.action());
        assert_eq!(pair.1.action_target(), pair.0.action_target());
        assert_eq!(pair.1.action_state(), pair.0.state());
    }
    // The snapshot reflects only the five Phase 1 action kinds.
    for person in snapshot.persons() {
        assert!(matches!(
            person.action(),
            ActionKind::Move
                | ActionKind::Eat
                | ActionKind::Sleep
                | ActionKind::Work
                | ActionKind::Idle
        ));
    }
}

#[test]
fn snapshot_is_immutable_and_headless_boundary_safe() {
    let kernel = seeded_kernel(5);
    let snapshot = RenderSnapshot::from_kernel(&kernel).expect("complete-boundary snapshot");
    // Diagnostic serde round-trips without any mutator (no &mut accessor).
    let encoded = serde_json::to_string(&snapshot).expect("serialize snapshot");
    let restored: RenderSnapshot = serde_json::from_str(&encoded).expect("deserialize snapshot");
    assert_eq!(restored, snapshot);
    assert!(restored.validate().is_ok());
    // The DTO depends on no Godot/Ecs type and carries no runtime handle: it
    // serializes purely as stable ids and terrain cells.
    assert!(!encoded.contains("bevy_ecs"));
    assert!(!encoded.contains("ScheduleToken"));
}

#[test]
fn snapshot_validates_schema_and_wire_invariants() {
    let kernel = seeded_kernel(2);
    let snapshot = RenderSnapshot::from_kernel(&kernel).expect("complete-boundary snapshot");
    let valid = serde_json::to_value(&snapshot).expect("encode");

    // Wrong schema version is rejected.
    let mut bad = valid.clone();
    bad["schema_version"] = serde_json::json!(99);
    assert!(serde_json::from_value::<RenderSnapshot>(bad).is_err());

    // Wrong terrain cell count is rejected.
    let mut bad = valid.clone();
    bad["terrain"]["cells"] = serde_json::json!([0]);
    assert!(serde_json::from_value::<RenderSnapshot>(bad).is_err());

    // Duplicate person ids are rejected.
    let mut bad = valid.clone();
    let ids = bad["persons"]
        .as_array()
        .expect("persons is an array")
        .iter()
        .map(|person| person["person_id"].clone())
        .collect::<Vec<_>>();
    if ids.len() >= 2 {
        bad["persons"].as_array_mut().expect("persons array")[1]["person_id"] = ids[0].clone();
        assert!(serde_json::from_value::<RenderSnapshot>(bad).is_err());
    }

    // Metric count mismatch is rejected.
    let mut bad = valid.clone();
    bad["metrics"]["person_count"] = serde_json::json!(999);
    assert!(serde_json::from_value::<RenderSnapshot>(bad).is_err());
}

#[test]
fn snapshot_never_invents_unmeasured_metrics() {
    // The DTO carries observable kernel fields only; a zero-measurement world
    // still yields an empty person batch and matching metric count (no fake).
    let kernel = WorldKernel::from_world(WorldSeed::new(1), KernelConfig::default());
    let snapshot = RenderSnapshot::from_kernel(&kernel).expect("complete-boundary snapshot");
    assert_eq!(snapshot.person_count(), 0);
    assert_eq!(snapshot.metrics().person_count, 0);

    // A snapshot build never mutates the kernel.
    let kernel = seeded_kernel(3);
    let before = kernel.now();
    let _ = RenderSnapshot::from_kernel(&kernel).expect("complete-boundary snapshot");
    assert_eq!(kernel.now(), before, "building a snapshot mutates nothing");
}

#[test]
fn malformed_wire_duplicate_or_zero_id_is_rejected() {
    // Start from a real, valid snapshot (two persons, idle world) and corrupt
    // the person-identity invariants on the wire.
    let kernel = seeded_kernel(2);
    let base = serde_json::to_value(
        RenderSnapshot::from_kernel(&kernel).expect("complete-boundary snapshot"),
    )
    .expect("encode");
    assert!(serde_json::from_value::<RenderSnapshot>(base.clone()).is_ok());

    let mut zero = base.clone();
    zero["persons"][0]["person_id"] = serde_json::json!(0);
    assert!(serde_json::from_value::<RenderSnapshot>(zero).is_err());

    let mut dup = base.clone();
    dup["persons"][1]["person_id"] = base["persons"][0]["person_id"].clone();
    assert!(serde_json::from_value::<RenderSnapshot>(dup).is_err());

    let mut unsorted = base.clone();
    unsorted["persons"][0]["person_id"] = serde_json::json!(10);
    let first = base["persons"][0]["person_id"].clone();
    unsorted["persons"][1]["person_id"] = first;
    assert!(serde_json::from_value::<RenderSnapshot>(unsorted).is_err());

    let mut mismatch = base.clone();
    mismatch["metrics"]["person_count"] = serde_json::json!(1);
    assert!(serde_json::from_value::<RenderSnapshot>(mismatch).is_err());
}
