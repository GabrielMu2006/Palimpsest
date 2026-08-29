// Authored by Kimi Code (AI coding agent) — task CHRON-018.
//! World domain boundary for the Phase 1 Micro World Kernel.
//!
//! `palimpsest-sim-world` hosts the local tile grid, typed coordinates,
//! terrain, deterministic world generation, activity sites, and deterministic
//! local-grid pathfinding. The crate is headless, Godot-free, and LLM-free,
//! and may depend only on `palimpsest-sim-entity`, `palimpsest-sim-time`, and
//! `serde` (ADR-0001, ADR-0017).
//!
//! CHRON-018 establishes the crate boundary only: there is no public domain
//! API yet, by design, so no speculative marker types can harden into
//! accidental contracts. Later tasks populate the crate — coordinates and the
//! local grid (CHRON-019), terrain and world generation (CHRON-020), activity
//! sites (CHRON-023), and pathfinding (CHRON-024).
