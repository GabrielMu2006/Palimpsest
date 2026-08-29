// Authored by Kimi Code (AI coding agent) — task CHRON-018.
// Extended by Kimi Code (AI coding agent) — task CHRON-022.
//! Utility-AI domain boundary for the Phase 1 Micro World Kernel.
//!
//! `palimpsest-sim-ai` hosts needs, action and decision-trace contracts, and
//! deterministic utility scoring/selection with auditable decision traces.
//! The crate is headless, Godot-free, and LLM-free, and may depend only on
//! `palimpsest-sim-world`, `palimpsest-sim-entity`, `palimpsest-sim-time`,
//! and `serde` (ADR-0001, ADR-0014, ADR-0017).
//!
//! CHRON-022 landed the bounded integer [`Needs`] model (hunger/fatigue).
//! Later tasks add action and decision-trace contracts (CHRON-025) and
//! utility scoring/selection (CHRON-026).

mod needs;

pub use crate::needs::{
    CRITICAL_PRESSURE, FATIGUE_RATE_PER_SECOND, HUNGER_RATE_PER_SECOND, NEED_MAX, NEED_SCALE,
    NeedValue, NeedValueError, Needs, PRESSURE_MAX,
};
