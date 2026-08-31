//! Independent serde-boundary probes for the render DTO repair (ADR-0025).

use palimpsest_sim_core::{KernelConfig, PersonRender, RenderSnapshot, TerrainBatch, WorldKernel};
use palimpsest_sim_world::{WorldGenConfig, WorldMap, WorldSeed};

fn base_snapshot() -> serde_json::Value {
    let seed = WorldSeed::new(25_025);
    let map = WorldMap::generate(seed, WorldGenConfig::default());
    let tile = map
        .local()
        .coords()
        .find(|coord| {
            map.local()
                .get(coord.x(), coord.y())
                .is_some_and(|terrain| terrain.is_walkable())
        })
        .expect("generated map has a walkable tile");
    let mut kernel = WorldKernel::from_world(seed, KernelConfig::default());
    kernel.spawn_person(tile).expect("spawn fixture person");
    serde_json::to_value(RenderSnapshot::from_kernel(&kernel).expect("snapshot"))
        .expect("snapshot serializes")
}

#[test]
fn terrain_batch_validates_dimensions_before_cell_count() {
    let base = base_snapshot();
    let cells = base["terrain"]["cells"].clone();
    for width in [0, 127, 129, usize::MAX] {
        let wire = serde_json::json!({"width": width, "height": 128, "cells": cells.clone()});
        assert!(serde_json::from_value::<TerrainBatch>(wire).is_err());
    }
    for cells in [serde_json::json!([]), serde_json::json!(["Ground"])] {
        let wire = serde_json::json!({"width": 128, "height": 128, "cells": cells});
        assert!(serde_json::from_value::<TerrainBatch>(wire).is_err());
    }
    let valid = serde_json::json!({
        "width": 128,
        "height": 128,
        "cells": base["terrain"]["cells"]
    });
    assert!(serde_json::from_value::<TerrainBatch>(valid).is_ok());
}

#[test]
fn person_render_validates_action_state_target_independently() {
    let base = base_snapshot();
    let person = base["persons"][0].clone();
    assert!(serde_json::from_value::<PersonRender>(person.clone()).is_ok());

    let target = serde_json::json!({"x": 1, "y": 1});
    for (action, state) in [
        ("Move", serde_json::json!({"Moving": {"action": "Move"}})),
        ("Eat", serde_json::json!("Eating")),
        ("Sleep", serde_json::json!("Sleeping")),
        ("Work", serde_json::json!("Working")),
    ] {
        let mut wire = person.clone();
        wire["action"] = serde_json::json!(action);
        wire["action_target"] = target.clone();
        wire["action_state"] = state;
        assert!(serde_json::from_value::<PersonRender>(wire).is_ok());
    }

    let mut moving_idle = person.clone();
    moving_idle["action"] = serde_json::json!("Idle");
    moving_idle["action_target"] = target.clone();
    moving_idle["action_state"] = serde_json::json!({"Moving": {"action": "Idle"}});
    assert!(serde_json::from_value::<PersonRender>(moving_idle).is_err());

    for (action, state, target_value) in [
        ("Eat", serde_json::json!("Idle"), serde_json::Value::Null),
        ("Idle", serde_json::json!("Idle"), target.clone()),
        ("Work", serde_json::json!("Eating"), target.clone()),
    ] {
        let mut wire = person.clone();
        wire["action"] = serde_json::json!(action);
        wire["action_state"] = state;
        wire["action_target"] = target_value;
        assert!(serde_json::from_value::<PersonRender>(wire).is_err());
    }
}

#[test]
fn root_decode_rejects_the_same_invalid_person_wire() {
    let mut bad = base_snapshot();
    bad["persons"][0]["action"] = serde_json::json!("Idle");
    bad["persons"][0]["action_target"] = serde_json::json!({"x": 1, "y": 1});
    bad["persons"][0]["action_state"] = serde_json::json!({
        "Moving": {"action": "Idle"}
    });
    assert!(serde_json::from_value::<RenderSnapshot>(bad).is_err());
}
