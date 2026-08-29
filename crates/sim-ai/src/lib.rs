// Authored by Kimi Code (AI coding agent) — task CHRON-018.
//! Utility-AI domain boundary for the Phase 1 Micro World Kernel.
//!
//! `palimpsest-sim-ai` hosts needs, action and decision-trace contracts, and
//! deterministic utility scoring/selection with auditable decision traces.
//! The crate is headless, Godot-free, and LLM-free, and may depend only on
//! `palimpsest-sim-world`, `palimpsest-sim-entity`, `palimpsest-sim-time`,
//! and `serde` (ADR-0001, ADR-0014, ADR-0017).
//!
//! CHRON-018 establishes the crate boundary only: there is no public domain
//! API yet, by design, so no speculative marker types can harden into
//! accidental contracts. Later tasks populate the crate — needs (CHRON-022),
//! action and decision-trace contracts (CHRON-025), and utility
//! scoring/selection (CHRON-026).
