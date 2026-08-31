// Authored by Kimi Code (AI coding agent) — task CHRON-018.
// Extended by Kimi Code (AI coding agent) — task CHRON-022.
// Extended by Kimi Code (AI coding agent) — task CHRON-025.
// Extended by Kimi Code (AI coding agent) — task CHRON-026.
//! Utility-AI domain boundary for the Phase 1 Micro World Kernel.
//!
//! `palimpsest-sim-ai` hosts needs, action and decision-trace contracts, and
//! deterministic utility scoring/selection with auditable decision traces.
//! The crate is headless, Godot-free, and LLM-free, and may depend only on
//! `palimpsest-sim-world`, `palimpsest-sim-entity`, `palimpsest-sim-time`,
//! and `serde` (ADR-0001, ADR-0014, ADR-0017).
//!
//! CHRON-022 landed the bounded integer [`Needs`] model (hunger/fatigue).
//! CHRON-025 landed the action-candidate and decision-trace contracts:
//! [`candidate_actions`] enumerates the ordered, deduplicated, bounded
//! [`ActionCandidate`] set and [`trace_for`] records the complete, ordered
//! [`FactorInput`] set per candidate with all scoring fields unset.
//! CHRON-026 landed integer utility scoring and selection:
//! [`score_candidates`] computes each candidate's saturating weighted base
//! term plus an explicit, seeded, bounded [`PerturbationSpec`] value, and
//! [`select_action`] picks the winner with a documented stable tie-break,
//! returning the populated [`DecisionTrace`] and the full [`CandidateScore`]
//! list.

mod action;
mod needs;
mod trace;
mod utility;

pub use crate::action::{
    ActionCandidate, ActionKind, CandidateContext, CandidateError, CandidateSetError,
    MAX_MOVE_CANDIDATES, candidate_actions,
};
pub use crate::needs::{
    CRITICAL_PRESSURE, FATIGUE_RATE_PER_SECOND, HUNGER_RATE_PER_SECOND, NEED_MAX, NEED_SCALE,
    NeedValue, NeedValueError, Needs, PRESSURE_MAX,
};
pub use crate::trace::{
    CandidateTrace, DecisionTrace, FactorEvaluation, FactorId, FactorInput, TieBreakReason,
    TraceValidationError, factor_inputs_for, trace_for,
};
pub use crate::utility::{
    CandidateScore, DecisionError, FactorWeights, MAX_EPSILON, PerturbationError,
    PerturbationRange, PerturbationSpec, Selection, UtilityScore, Weights, score_candidates,
    select_action,
};
