// Authored by Kimi Code (AI coding agent) — task CHRON-018.
//! Workspace dependency-direction audit for ADR-0001 and ADR-0017.
//!
//! These tests run `cargo metadata --no-deps` against the workspace and
//! assert that the Phase 1 domain crates stay inside their allowed dependency
//! sets and that no workspace crate other than `palimpsest-godot-bridge`
//! reaches Godot, the bridge, or an LLM library. Only normal (non-dev,
//! non-build) dependencies are audited; dev-dependencies such as this test's
//! own `serde_json` are tooling, not library surface.

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::process::Command;

use serde_json::Value;

fn workspace_metadata() -> Value {
    let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|crates_dir| crates_dir.parent())
        .expect("sim-ai sits two levels below the workspace root")
        .to_owned();
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_owned());
    let output = Command::new(cargo)
        .args([
            "metadata",
            "--format-version",
            "1",
            "--no-deps",
            "--offline",
        ])
        .current_dir(workspace_root)
        .output()
        .expect("cargo metadata runs");
    assert!(
        output.status.success(),
        "cargo metadata failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("cargo metadata prints JSON")
}

fn packages(metadata: &Value) -> &[Value] {
    metadata["packages"].as_array().expect("packages array")
}

fn package_by_name<'a>(metadata: &'a Value, name: &str) -> &'a Value {
    packages(metadata)
        .iter()
        .find(|package| package["name"] == name)
        .unwrap_or_else(|| panic!("workspace package {name} not found"))
}

/// Names of the normal (non-dev, non-build) dependencies of `package`.
fn normal_dependency_names(package: &Value) -> BTreeSet<String> {
    package["dependencies"]
        .as_array()
        .expect("dependencies array")
        .iter()
        .filter(|dependency| dependency["kind"].is_null())
        .map(|dependency| {
            dependency["name"]
                .as_str()
                .expect("dependency name is a string")
                .to_owned()
        })
        .collect()
}

fn allow_set(names: &[&str]) -> BTreeSet<String> {
    names.iter().map(|name| (*name).to_owned()).collect()
}

#[test]
fn phase_1_crates_are_workspace_members() {
    let metadata = workspace_metadata();
    let members: Vec<&str> = metadata["workspace_members"]
        .as_array()
        .expect("workspace_members array")
        .iter()
        .map(|id| id.as_str().expect("workspace member id is a string"))
        .collect();
    for name in ["palimpsest-sim-world", "palimpsest-sim-ai"] {
        let id = package_by_name(&metadata, name)["id"]
            .as_str()
            .expect("package id is a string");
        assert!(members.contains(&id), "{name} is not a workspace member");
    }
}

#[test]
fn sim_world_respects_its_dependency_allow_set() {
    let metadata = workspace_metadata();
    let allowed = allow_set(&["palimpsest-sim-entity", "palimpsest-sim-time", "serde"]);
    let actual = normal_dependency_names(package_by_name(&metadata, "palimpsest-sim-world"));
    assert!(
        actual.is_subset(&allowed),
        "palimpsest-sim-world dependencies {actual:?} exceed the ADR-0017 allow-set {allowed:?}"
    );
}

#[test]
fn sim_ai_respects_its_dependency_allow_set() {
    let metadata = workspace_metadata();
    let allowed = allow_set(&[
        "palimpsest-sim-world",
        "palimpsest-sim-entity",
        "palimpsest-sim-time",
        "serde",
    ]);
    let actual = normal_dependency_names(package_by_name(&metadata, "palimpsest-sim-ai"));
    assert!(
        actual.is_subset(&allowed),
        "palimpsest-sim-ai dependencies {actual:?} exceed the ADR-0017 allow-set {allowed:?}"
    );
}

#[test]
fn only_the_bridge_crate_reaches_godot_and_nothing_reaches_llm() {
    let metadata = workspace_metadata();
    for package in packages(&metadata) {
        let name = package["name"].as_str().expect("package name is a string");
        if name == "palimpsest-godot-bridge" {
            continue;
        }
        for dependency in normal_dependency_names(package) {
            let lowered = dependency.to_ascii_lowercase();
            assert!(
                dependency != "palimpsest-godot-bridge"
                    && !lowered.starts_with("godot")
                    && !lowered.contains("llm"),
                "{name} must not depend on {dependency} (ADR-0001/ADR-0017)"
            );
        }
    }
}
