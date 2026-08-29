// Authored by Kimi Code (AI coding agent) — task CHRON-018.
// Extended by Kimi Code (AI coding agent) — tasks CHRON-019, CHRON-020, CHRON-023, CHRON-024.
//! World domain boundary for the Phase 1 Micro World Kernel.
//!
//! `palimpsest-sim-world` hosts the local tile grid, typed coordinates,
//! terrain, deterministic world generation, activity sites, and deterministic
//! local-grid pathfinding. The crate is headless, Godot-free, and LLM-free,
//! and may depend only on `palimpsest-sim-entity`, `palimpsest-sim-time`, and
//! `serde` (ADR-0001, ADR-0017).
//!
//! CHRON-019 landed the typed [`LocalCoord`] and the boundary-safe 128×128
//! [`LocalGrid`]/[`WorldGrid`] containers (ADR-0012); CHRON-020 landed
//! [`TerrainKind`] and deterministic [`WorldMap`] generation; CHRON-023 landed
//! static [`ActivitySites`]; CHRON-024 landed deterministic [`find_path`].

mod coord;
mod grid;
mod pathfinding;
mod site;
mod terrain;
mod worldgen;

pub use crate::coord::LocalCoord;
pub use crate::grid::{
    GridError, LOCAL_GRID_CELL_COUNT, LOCAL_GRID_HEIGHT, LOCAL_GRID_WIDTH, LocalGrid, WorldGrid,
};
pub use crate::pathfinding::{Path, PathConfig, PathError, find_path};
pub use crate::site::{ActivitySite, ActivitySites, SiteError, SiteKind, WorkCounter};
pub use crate::terrain::TerrainKind;
pub use crate::worldgen::{WorldGenConfig, WorldMap, WorldSeed};
