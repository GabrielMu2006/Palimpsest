// Authored by Kimi Code (AI coding agent) — task CHRON-026.
//! Integer utility scoring and deterministic selection for the Phase 1
//! Utility AI (CHRON-026, ADR-0014; Master Spec §2.4/§14/§72).
//!
//! [`score_candidates`] computes `score = base + perturbation` for every
//! [`ActionCandidate`] of one enumeration: `base` is the saturating sum of
//! `weight(kind, factor) × input` over the complete [`FactorId`] set that
//! [`factor_inputs_for`] records, and `perturbation` is an explicit, seeded,
//! bounded integer that may be zero ([`PerturbationRange::Zero`], the Phase 1
//! default per ADR-0014). [`select_action`] picks the highest score, breaks
//! ties by the stable enumeration key ([`ActionCandidate::order`]), and
//! returns the winner together with the populated [`DecisionTrace`] and the
//! ordered per-candidate [`CandidateScore`] list for Developer Mode (§72).
//!
//! Everything is integer-only: no float or NaN can appear by construction,
//! all arithmetic saturates instead of wrapping, and no clock, thread, or
//! hash-order dependence exists. The perturbation derives from an in-crate
//! splitmix64 mixer keyed on the seed and the candidate; it is additive to
//! the base term, bounded by [`MAX_EPSILON`], and never the sole term or a
//! `random_action()` (ADR-0014).

use core::fmt::{self, Display, Formatter};

use serde::{Deserialize, Deserializer, Serialize};

use crate::action::{
    ActionCandidate, ActionKind, CandidateContext, CandidateSetError, validate_candidate_set,
};
use crate::trace::{
    CandidateTrace, DecisionTrace, FactorEvaluation, FactorId, TieBreakReason,
    TraceValidationError, factor_inputs_for,
};

/// A bounded signed integer utility score (CHRON-026, ADR-0014).
///
/// The type-level bounds are the full `i64` range ([`UtilityScore::MIN`],
/// [`UtilityScore::MAX`]); scoring uses saturating arithmetic only, so
/// overflow, silent wrap, and NaN are impossible by construction. With the
/// Phase 1 default [`Weights`] and the bounded [`FactorId`] inputs
/// (pressures `0..=1000`, distance `0..=254`, availability `0`/`1`, and the
/// zero-weighted work progress), the conservative base range is
/// `[-1_270, 10_000]`; with the maximum perturbation (`ε = 100`, see
/// [`MAX_EPSILON`]) the conservative total range is `[-1_370, 10_100]`. Serde
/// encodes the bare `i64`.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(transparent)]
pub struct UtilityScore(i64);

impl UtilityScore {
    /// Type-level minimum (the `i64` lower bound).
    pub const MIN: Self = Self(i64::MIN);
    /// Type-level maximum (the `i64` upper bound).
    pub const MAX: Self = Self(i64::MAX);

    /// The raw integer value.
    #[must_use]
    pub const fn get(self) -> i64 {
        self.0
    }
}

/// The five integer weights of one [`ActionKind`], one per [`FactorId`]
/// (CHRON-026).
///
/// A weight of `0` means the factor is still recorded in the candidate trace
/// but contributes nothing — the complete-trace contract never silently
/// drops a factor (CHRON-025/CHRON-026 invariant 4). Serde encodes the five
/// fields in declaration order with [`FactorId`]-matching `snake_case` keys.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
pub struct FactorWeights {
    hunger: i64,
    fatigue: i64,
    distance_to_target: i64,
    site_available: i64,
    work_progress: i64,
}

impl FactorWeights {
    /// Creates a weight set in [`FactorId`] declaration order.
    #[must_use]
    pub const fn new(
        hunger: i64,
        fatigue: i64,
        distance_to_target: i64,
        site_available: i64,
        work_progress: i64,
    ) -> Self {
        Self {
            hunger,
            fatigue,
            distance_to_target,
            site_available,
            work_progress,
        }
    }

    /// The weight of one factor.
    #[must_use]
    pub const fn weight(&self, factor: FactorId) -> i64 {
        match factor {
            FactorId::Hunger => self.hunger,
            FactorId::Fatigue => self.fatigue,
            FactorId::DistanceToTarget => self.distance_to_target,
            FactorId::SiteAvailable => self.site_available,
            FactorId::WorkProgress => self.work_progress,
        }
    }
}

/// The per-([`ActionKind`] × [`FactorId`]) weight table (CHRON-026).
///
/// The table is keyed per action kind, not per factor alone: the Hunger and
/// Fatigue inputs are identical for every candidate of a person, so a single
/// global weight per [`FactorId`] could never make high hunger select `Eat`
/// over `Sleep` — the ranking would be needs-independent (see
/// `docs/tasks/SPEC_CONFLICT_LOG.md` SC-008).
///
/// [`Weights::default`] is the documented Phase 1 table:
///
/// | kind  | Hunger | Fatigue | Distance | `SiteAvailable` | `WorkProgress` |
/// |-------|--------|---------|----------|-----------------|----------------|
/// | Move  | 0      | 0       | −5       | +10             | 0              |
/// | Eat   | +10    | 0       | −5       | 0               | 0              |
/// | Sleep | 0      | +10     | −5       | 0               | 0              |
/// | Work  | 0      | 0       | −5       | `+2_300`        | 0              |
/// | Idle  | 0      | 0       | 0        | −50             | 0              |
///
/// Rationale: the needs-driven kinds score pressure × 10 minus distance,
/// while Work's `+2_300` availability baseline makes a reachable work site
/// outrank the `Idle` baseline through low need pressure. [`FactorId::WorkProgress`] is
/// recorded but weight 0 — a complete trace with zero contribution, per the
/// task contract.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
// The field names are the documented per-kind wire keys of the table; the
// shared postfix mirrors the `FactorWeights` element type by design.
#[allow(clippy::struct_field_names)]
pub struct Weights {
    move_weights: FactorWeights,
    eat_weights: FactorWeights,
    sleep_weights: FactorWeights,
    work_weights: FactorWeights,
    idle_weights: FactorWeights,
}

impl Weights {
    /// Creates a table from the five per-kind weight sets in [`ActionKind`]
    /// declaration order.
    #[must_use]
    pub const fn new(
        move_weights: FactorWeights,
        eat_weights: FactorWeights,
        sleep_weights: FactorWeights,
        work_weights: FactorWeights,
        idle_weights: FactorWeights,
    ) -> Self {
        Self {
            move_weights,
            eat_weights,
            sleep_weights,
            work_weights,
            idle_weights,
        }
    }

    /// The weight set of one action kind.
    #[must_use]
    pub const fn weights_for(&self, kind: ActionKind) -> FactorWeights {
        match kind {
            ActionKind::Move => self.move_weights,
            ActionKind::Eat => self.eat_weights,
            ActionKind::Sleep => self.sleep_weights,
            ActionKind::Work => self.work_weights,
            ActionKind::Idle => self.idle_weights,
        }
    }

    /// The weight of one factor for one action kind.
    #[must_use]
    pub const fn weight(&self, kind: ActionKind, factor: FactorId) -> i64 {
        self.weights_for(kind).weight(factor)
    }
}

impl Default for Weights {
    /// The documented Phase 1 table (see the struct documentation).
    fn default() -> Self {
        Self::new(
            FactorWeights::new(0, 0, -5, 10, 0),
            FactorWeights::new(10, 0, -5, 0, 0),
            FactorWeights::new(0, 10, -5, 0, 0),
            FactorWeights::new(0, 0, -5, 2_300, 0),
            FactorWeights::new(0, 0, 0, -50, 0),
        )
    }
}

/// Maximum perturbation half-width ε (CHRON-026).
///
/// The perturbation lives in `[-ε, +ε]`; 100 keeps it at tie-break scale —
/// it can reorder near-ties (a few distance steps apart) but stays far below
/// the availability and pressure terms of the default [`Weights`]
/// (thousands), so it never becomes the decision mechanism (ADR-0014).
pub const MAX_EPSILON: i64 = 100;

/// The perturbation strength: exactly zero, or a bounded symmetric range
/// (CHRON-026).
///
/// Serde uses the default variant names (`"Zero"`, `{"Bounded": ε}`); those
/// are the stable wire keys and must never change.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize)]
pub enum PerturbationRange {
    /// No perturbation at all: the fully deterministic mode (ADR-0014
    /// default in Phase 1).
    Zero,
    /// Uniform integer perturbation in `[-ε, +ε]`. [`PerturbationSpec::new`]
    /// rejects ε outside `0..=MAX_EPSILON`. Native code can form an invalid
    /// raw request, but cannot use it to construct an execution spec.
    /// Deserialization rejects invalid ranges too; nothing is clamped.
    Bounded(i64),
}

impl<'de> Deserialize<'de> for PerturbationRange {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        enum Wire {
            Zero,
            Bounded(i64),
        }
        let range = match Wire::deserialize(deserializer)? {
            Wire::Zero => Self::Zero,
            Wire::Bounded(epsilon) => Self::Bounded(epsilon),
        };
        PerturbationSpec::new(0, range)
            .map(|spec| spec.range())
            .map_err(serde::de::Error::custom)
    }
}

/// An invalid perturbation request, rejected before execution.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PerturbationError {
    /// Epsilon must be in the inclusive interval 0..=100.
    EpsilonOutOfRange { epsilon: i64 },
}

impl Display for PerturbationError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EpsilonOutOfRange { epsilon } => {
                write!(f, "epsilon {epsilon} outside 0..={MAX_EPSILON}")
            }
        }
    }
}
impl std::error::Error for PerturbationError {}

/// The explicit perturbation input of one decision: a seed and a range
/// (CHRON-026, ADR-0014).
///
/// The per-candidate value is a deterministic function of the seed and the
/// candidate (kind, target, enumeration key) via an in-crate splitmix64
/// mixer — no external PRNG dependency, no platform dependence. The default
/// is [`PerturbationSpec::ZERO`]: seed 0, [`PerturbationRange::Zero`]
/// (ADR-0014: "default 0 in Phase 1").
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize)]
pub struct PerturbationSpec {
    seed: u64,
    range: PerturbationRange,
}

impl PerturbationSpec {
    /// The zero-perturbation spec: seed 0, [`PerturbationRange::Zero`].
    pub const ZERO: Self = Self {
        seed: 0,
        range: PerturbationRange::Zero,
    };

    /// Returns [`PerturbationSpec::ZERO`].
    #[must_use]
    pub const fn zero() -> Self {
        Self::ZERO
    }

    /// Creates a spec, rejecting ε outside `0..=MAX_EPSILON`.
    ///
    /// # Errors
    /// Returns a typed error for epsilon outside the permitted interval.
    pub const fn new(seed: u64, range: PerturbationRange) -> Result<Self, PerturbationError> {
        match range {
            PerturbationRange::Zero => Ok(Self { seed, range }),
            PerturbationRange::Bounded(epsilon) => {
                if epsilon < 0 || epsilon > MAX_EPSILON {
                    Err(PerturbationError::EpsilonOutOfRange { epsilon })
                } else {
                    Ok(Self { seed, range })
                }
            }
        }
    }

    /// The perturbation seed.
    #[must_use]
    pub const fn seed(&self) -> u64 {
        self.seed
    }

    /// The perturbation range.
    #[must_use]
    pub const fn range(&self) -> PerturbationRange {
        self.range
    }
}

impl<'de> Deserialize<'de> for PerturbationSpec {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        struct Wire {
            seed: u64,
            range: PerturbationRange,
        }
        let wire = Wire::deserialize(deserializer)?;
        Self::new(wire.seed, wire.range).map_err(serde::de::Error::custom)
    }
}

impl Default for PerturbationSpec {
    /// The ADR-0014 Phase 1 default: [`PerturbationSpec::ZERO`].
    fn default() -> Self {
        Self::ZERO
    }
}

/// One scored candidate: the candidate, its base term, the perturbation
/// applied, the total score, and the populated per-candidate trace
/// (CHRON-026).
///
/// `score == base + perturbation` holds with saturating addition; the trace
/// records every factor contribution, the total, and the perturbation.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CandidateScore {
    candidate: ActionCandidate,
    base: UtilityScore,
    perturbation: i64,
    score: UtilityScore,
    trace: CandidateTrace,
}

impl CandidateScore {
    /// The scored candidate.
    #[must_use]
    pub const fn candidate(&self) -> ActionCandidate {
        self.candidate
    }

    /// The weighted base term before perturbation.
    #[must_use]
    pub const fn base(&self) -> UtilityScore {
        self.base
    }

    /// The seeded perturbation applied to the base term.
    #[must_use]
    pub const fn perturbation(&self) -> i64 {
        self.perturbation
    }

    /// The total utility: `base + perturbation` (saturating).
    #[must_use]
    pub const fn score(&self) -> UtilityScore {
        self.score
    }

    /// The populated per-candidate trace.
    #[must_use]
    pub const fn trace(&self) -> &CandidateTrace {
        &self.trace
    }
}

/// The outcome of one decision: the winning candidate and score, the fully
/// populated [`DecisionTrace`], and every candidate's score in enumeration
/// order for Developer Mode (Master Spec §72).
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Selection {
    candidate: ActionCandidate,
    score: UtilityScore,
    trace: DecisionTrace,
    all_scores: Vec<CandidateScore>,
}

impl Selection {
    // Identity/copy consistency only: imported diagnostics do not establish
    // historical world truth or authorize action execution.
    fn validate(&self) -> Result<(), TraceValidationError> {
        let identities: Vec<_> = self
            .all_scores
            .iter()
            .map(CandidateScore::candidate)
            .collect();
        if identities.is_empty() {
            return Err(TraceValidationError::EmptySelection);
        }
        validate_candidate_set(&identities, true)
            .map_err(TraceValidationError::InvalidCandidates)?;
        let order = self.candidate.order();
        let inconsistent = TraceValidationError::InconsistentSelection { order };
        if self.trace.selected() != Some(order) {
            return Err(inconsistent);
        }
        if self.trace.candidates().len() != self.all_scores.len() {
            return Err(TraceValidationError::CandidateCountMismatch);
        }
        let chosen = self
            .all_scores
            .iter()
            .find(|entry| entry.candidate.order() == order)
            .ok_or(TraceValidationError::SelectedKeyMissing { order })?;
        if chosen.candidate != self.candidate || chosen.score != self.score {
            return Err(inconsistent);
        }
        for entry in &self.all_scores {
            let order = entry.candidate.order();
            let trace = self
                .trace
                .candidates()
                .iter()
                .find(|trace| trace.candidate().order() == order)
                .ok_or(TraceValidationError::SelectedKeyMissing { order })?;
            if entry.trace.candidate() != entry.candidate
                || entry.trace.total() != Some(entry.score.get())
                || entry.trace.perturbation() != Some(entry.perturbation)
                || trace != &entry.trace
            {
                return Err(TraceValidationError::InconsistentSelection { order });
            }
        }
        Ok(())
    }

    /// The selected candidate.
    #[must_use]
    pub const fn candidate(&self) -> ActionCandidate {
        self.candidate
    }

    /// The winner's total utility.
    #[must_use]
    pub const fn score(&self) -> UtilityScore {
        self.score
    }

    /// The complete decision trace, selection outcome included.
    #[must_use]
    pub const fn trace(&self) -> &DecisionTrace {
        &self.trace
    }

    /// Every scored candidate in enumeration (input) order.
    #[must_use]
    pub fn all_scores(&self) -> &[CandidateScore] {
        &self.all_scores
    }
}

impl<'de> Deserialize<'de> for Selection {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        struct Wire {
            candidate: ActionCandidate,
            score: UtilityScore,
            trace: DecisionTrace,
            all_scores: Vec<CandidateScore>,
        }
        let wire = Wire::deserialize(deserializer)?;
        let selection = Self {
            candidate: wire.candidate,
            score: wire.score,
            trace: wire.trace,
            all_scores: wire.all_scores,
        };
        selection.validate().map_err(serde::de::Error::custom)?;
        Ok(selection)
    }
}

/// Errors from action selection (CHRON-026).
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
pub enum DecisionError {
    /// The candidate set was empty; no action is synthesized in that case
    /// (invariant 5).
    EmptyCandidates,
    /// A selection requires unique, contiguous keys and unique action/target pairs.
    InvalidCandidates(CandidateSetError),
}

impl Display for DecisionError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidCandidates(error) => Display::fmt(error, formatter),
            Self::EmptyCandidates => {
                write!(
                    formatter,
                    "candidate set is empty; no action can be selected"
                )
            }
        }
    }
}

impl std::error::Error for DecisionError {}

/// Scores every candidate in input order with the documented integer
/// arithmetic (CHRON-026).
///
/// For each candidate the base term is `Σ saturating(weight(kind, factor) ×
/// input)` over the complete [`factor_inputs_for`] set, the perturbation is
/// the seeded [`PerturbationSpec`] value, and the total is the saturating
/// sum. Every factor contribution, the total, and the perturbation are
/// recorded in the candidate's trace, so the full calculation is auditable
/// (Master Spec §72). Equal inputs yield identical, identically ordered
/// output (invariant 2).
/// Diagnostic subsets may carry arbitrary keys; this function does not
/// certify a complete selection set or populate a selected key.
#[must_use]
pub fn score_candidates(
    candidates: &[ActionCandidate],
    context: &CandidateContext<'_>,
    weights: &Weights,
    spec: &PerturbationSpec,
) -> Vec<CandidateScore> {
    candidates
        .iter()
        .map(|candidate| {
            let kind = candidate.kind();
            let inputs = factor_inputs_for(candidate, context);
            let mut base = 0_i64;
            let mut factors = Vec::with_capacity(inputs.len());
            for input in inputs {
                let contribution = weights
                    .weight(kind, input.factor())
                    .saturating_mul(input.input());
                base = base.saturating_add(contribution);
                factors.push(FactorEvaluation::scored(input, contribution));
            }
            let perturbation = perturbation_for(spec, candidate);
            let total = base.saturating_add(perturbation);
            let trace = CandidateTrace::scored(*candidate, factors, total, perturbation);
            CandidateScore {
                candidate: *candidate,
                base: UtilityScore(base),
                perturbation,
                score: UtilityScore(total),
                trace,
            }
        })
        .collect()
}

/// Selects the highest-utility candidate with a documented, stable
/// tie-break (CHRON-026, ADR-0014).
///
/// The winner is the maximum under (score descending,
/// [`ActionCandidate::order`] ascending): `order` is CHRON-025's stable
/// enumeration key, so equal totals resolve identically on every platform
/// with no hash, insertion-order, clock, or thread dependence.
/// [`TieBreakReason::UniqueMaximum`] is recorded when the winner's score
/// strictly exceeds every other candidate's, and
/// [`TieBreakReason::StableOrder`] otherwise. The returned [`Selection`]
/// carries the populated [`DecisionTrace`] and the full score list in
/// enumeration order.
///
/// # Errors
///
/// Returns [`DecisionError::EmptyCandidates`] when `candidates` is empty; no
/// untraced action is synthesized (invariant 5).
/// Returns [`DecisionError::InvalidCandidates`] for duplicate keys/pairs or
/// keys outside `0..len`. Input permutation is valid; the order key, not
/// vector position, resolves ties. No candidate is renumbered or dropped.
///
/// # Panics
///
/// Never in practice: the empty case returns early, so the winner lookup
/// always succeeds.
pub fn select_action(
    candidates: &[ActionCandidate],
    context: &CandidateContext<'_>,
    weights: &Weights,
    spec: &PerturbationSpec,
) -> Result<Selection, DecisionError> {
    if candidates.is_empty() {
        return Err(DecisionError::EmptyCandidates);
    }
    validate_candidate_set(candidates, true).map_err(DecisionError::InvalidCandidates)?;
    let all_scores = score_candidates(candidates, context, weights, spec);
    let winner_index = all_scores
        .iter()
        .enumerate()
        .max_by(|(_, left), (_, right)| {
            left.score
                .cmp(&right.score)
                .then(right.candidate.order().cmp(&left.candidate.order()))
        })
        .map(|(index, _)| index)
        .expect("the candidate set is non-empty");
    let winner_candidate = all_scores[winner_index].candidate;
    let winner_score = all_scores[winner_index].score;
    let tie_break = if all_scores
        .iter()
        .filter(|entry| entry.score == winner_score)
        .count()
        == 1
    {
        TieBreakReason::UniqueMaximum
    } else {
        TieBreakReason::StableOrder
    };
    let trace = DecisionTrace::decided(
        all_scores.iter().map(|entry| entry.trace.clone()).collect(),
        winner_candidate.order(),
        tie_break,
    );
    Ok(Selection {
        candidate: winner_candidate,
        score: winner_score,
        trace,
        all_scores,
    })
}

/// The deterministic per-candidate perturbation value (CHRON-026).
///
/// The seed, the candidate's [`ActionKind`] index (0..=4), the target's
/// row-major index (`u64::MAX` for the targetless `Idle`), and the
/// enumeration key are folded through [`splitmix64`]; the result is mapped
/// into `[-ε, +ε]` via `h % (2ε + 1) − ε`. [`PerturbationRange::Zero`]
/// yields 0. Wrapping `u64` arithmetic makes the value platform-independent.
fn perturbation_for(spec: &PerturbationSpec, candidate: &ActionCandidate) -> i64 {
    let PerturbationRange::Bounded(epsilon) = spec.range() else {
        return 0;
    };
    // Both native construction and deserialization validate this value.
    let kind_key = match candidate.kind() {
        ActionKind::Move => 0_u64,
        ActionKind::Eat => 1,
        ActionKind::Sleep => 2,
        ActionKind::Work => 3,
        ActionKind::Idle => 4,
    };
    let target_key = candidate.target().map_or(u64::MAX, |target| {
        u64::try_from(target.index()).expect("LocalCoord::index is below 16_384")
    });
    let mut hash = splitmix64(spec.seed());
    hash = splitmix64(hash ^ kind_key);
    hash = splitmix64(hash ^ target_key);
    hash = splitmix64(hash ^ candidate.order());
    let width = (2 * epsilon + 1).cast_unsigned();
    (hash % width).cast_signed() - epsilon
}

/// One splitmix64 mixing round (Stafford's variant-13 constants): a
/// deterministic, platform-independent `u64 → u64` hash. Used only to derive
/// the seeded perturbation — never as a behavior driver (ADR-0014).
const fn splitmix64(value: u64) -> u64 {
    let mut mixed = value.wrapping_add(0x9E37_79B9_7F4A_7C15);
    mixed = (mixed ^ (mixed >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    mixed = (mixed ^ (mixed >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    mixed ^ (mixed >> 31)
}

#[cfg(test)]
mod tests {
    use super::{
        CandidateScore, DecisionError, FactorWeights, MAX_EPSILON, PerturbationError,
        PerturbationRange, PerturbationSpec, Selection, UtilityScore, Weights, score_candidates,
        select_action,
    };
    use crate::action::{ActionCandidate, ActionKind, CandidateContext, candidate_actions};
    use crate::needs::{NeedValue, Needs};
    use crate::trace::{FactorId, FactorInput, TieBreakReason};
    use palimpsest_sim_time::SimDuration;
    use palimpsest_sim_world::{
        ActivitySite, ActivitySites, LocalCoord, PathConfig, SiteKind, TerrainKind, WorldGenConfig,
        WorldMap, WorldSeed, find_path,
    };

    /// Locked fixture seed; any seed works because the generator guarantees a
    /// walkable spawn clearing (8×8 by default).
    const FIXTURE_SEED: u64 = 25_025;

    fn default_map() -> WorldMap {
        WorldMap::generate(WorldSeed::new(FIXTURE_SEED), WorldGenConfig::default())
    }

    fn coord(x: i32, y: i32) -> LocalCoord {
        LocalCoord::new(x, y).expect("test coordinate in bounds")
    }

    fn needs_with(hunger: i64, fatigue: i64) -> Needs {
        Needs::new(
            NeedValue::from_raw(hunger).expect("in range"),
            NeedValue::from_raw(fatigue).expect("in range"),
        )
    }

    /// Origin of a fully walkable 3×3 block, guaranteed by the spawn
    /// clearing of the default generator config.
    fn walkable_block_origin(map: &WorldMap) -> LocalCoord {
        map.local()
            .coords()
            .find(|origin| {
                (0..3).all(|dy| {
                    (0..3).all(|dx| {
                        LocalCoord::new(origin.x() + dx, origin.y() + dy).is_some_and(|coord| {
                            map.local()
                                .get(coord.x(), coord.y())
                                .is_some_and(|kind| kind.is_walkable())
                        })
                    })
                })
            })
            .expect("spawn clearing contains a 3x3 walkable block")
    }

    fn distant_reachable_coord(map: &WorldMap, origin: LocalCoord) -> LocalCoord {
        map.local()
            .coords()
            .filter(|target| {
                (target.x() - origin.x()).abs() + (target.y() - origin.y()).abs() >= 20
            })
            .find(|target| {
                find_path(
                    map.local(),
                    (origin.x(), origin.y()),
                    (target.x(), target.y()),
                    TerrainKind::is_walkable,
                    PathConfig::default(),
                )
                .is_ok()
            })
            .expect("generated map has a distant reachable walkable coordinate")
    }

    /// Fixture: person at the block origin; Meal, Rest, Work sites inside
    /// the block. The Work site carries a recorded counter of 3.
    fn fixture() -> (WorldMap, ActivitySites, LocalCoord) {
        let map = default_map();
        let origin = walkable_block_origin(&map);
        let (ox, oy) = (origin.x(), origin.y());
        let work = coord(ox + 2, oy + 2);
        let mut sites = ActivitySites::new(vec![
            ActivitySite::new(&map, coord(ox + 2, oy), SiteKind::Meal).expect("walkable"),
            ActivitySite::new(&map, coord(ox, oy + 2), SiteKind::Rest).expect("walkable"),
            ActivitySite::new(&map, work, SiteKind::Work).expect("walkable"),
        ])
        .expect("distinct coords");
        for expected in 1..=3_u64 {
            assert_eq!(sites.record_work(work), Ok(expected));
        }
        (map, sites, origin)
    }

    fn context<'a>(
        location: LocalCoord,
        needs: Needs,
        sites: &'a ActivitySites,
        map: &'a WorldMap,
    ) -> CandidateContext<'a> {
        CandidateContext::new(location, needs, sites, map, PathConfig::default())
    }

    /// Every factor of every kind weighted `value` (saturation fixture).
    fn uniform_weights(value: i64) -> Weights {
        let set = FactorWeights::new(value, value, value, value, value);
        Weights::new(set, set, set, set, set)
    }

    /// All-zero weights: every candidate's base term is 0.
    fn flat_weights() -> Weights {
        uniform_weights(0)
    }

    #[test]
    fn utility_score_is_a_bounded_i64_newtype() {
        const { assert!(size_of::<UtilityScore>() == size_of::<i64>()) };
        assert_eq!(UtilityScore::MIN.get(), i64::MIN);
        assert_eq!(UtilityScore::MAX.get(), i64::MAX);
        assert!(UtilityScore::MIN < UtilityScore(0));
        assert!(UtilityScore(0) < UtilityScore::MAX);
        let score = UtilityScore(-4_990);
        let encoded = serde_json::to_string(&score).expect("serialize score");
        assert_eq!(encoded, "-4990", "the wire form is the bare i64");
        assert_eq!(
            serde_json::from_str::<UtilityScore>(&encoded).expect("deserialize score"),
            score
        );
    }

    #[test]
    fn default_weights_match_the_documented_phase_1_table() {
        let weights = Weights::default();
        assert_eq!(
            weights.weights_for(ActionKind::Move),
            FactorWeights::new(0, 0, -5, 10, 0)
        );
        assert_eq!(
            weights.weights_for(ActionKind::Eat),
            FactorWeights::new(10, 0, -5, 0, 0)
        );
        assert_eq!(
            weights.weights_for(ActionKind::Sleep),
            FactorWeights::new(0, 10, -5, 0, 0)
        );
        assert_eq!(
            weights.weights_for(ActionKind::Work),
            FactorWeights::new(0, 0, -5, 2_300, 0)
        );
        assert_eq!(
            weights.weights_for(ActionKind::Idle),
            FactorWeights::new(0, 0, 0, -50, 0)
        );
        // The (kind, factor) accessor agrees with the per-kind sets.
        let kinds = [
            ActionKind::Move,
            ActionKind::Eat,
            ActionKind::Sleep,
            ActionKind::Work,
            ActionKind::Idle,
        ];
        let factors = [
            FactorId::Hunger,
            FactorId::Fatigue,
            FactorId::DistanceToTarget,
            FactorId::SiteAvailable,
            FactorId::WorkProgress,
        ];
        for kind in kinds {
            for factor in factors {
                assert_eq!(
                    weights.weight(kind, factor),
                    weights.weights_for(kind).weight(factor)
                );
            }
        }
    }

    #[test]
    fn default_weights_achievable_range_is_as_documented() {
        // Input bounds (trace.rs): pressures 0..=1000, distance 0..=254
        // (128×128 Manhattan), availability 0..=1, work progress bounded by
        // WorkCounter::MAX (weight 0 under the defaults).
        let bounds = [
            (FactorId::Hunger, 1_000),
            (FactorId::Fatigue, 1_000),
            (FactorId::DistanceToTarget, 254),
            (FactorId::SiteAvailable, 1),
            (FactorId::WorkProgress, 10_000_000),
        ];
        let kinds = [
            ActionKind::Move,
            ActionKind::Eat,
            ActionKind::Sleep,
            ActionKind::Work,
            ActionKind::Idle,
        ];
        let weights = Weights::default();
        let mut min_base = i64::MAX;
        let mut max_base = i64::MIN;
        for kind in kinds {
            let mut low = 0_i64;
            let mut high = 0_i64;
            for (factor, bound) in bounds {
                let weight = weights.weight(kind, factor);
                if weight >= 0 {
                    high += weight * bound;
                } else {
                    low += weight * bound;
                }
            }
            min_base = min_base.min(low);
            max_base = max_base.max(high);
        }
        assert_eq!((min_base, max_base), (-1_270, 10_000), "base range");
        assert_eq!(
            (min_base - MAX_EPSILON, max_base + MAX_EPSILON),
            (-1_370, 10_100),
            "total range with maximal perturbation"
        );
    }

    #[test]
    fn base_term_matches_the_documented_arithmetic() {
        let (map, sites, origin) = fixture();
        let context = context(origin, needs_with(50_000, 25_000), &sites, &map);
        let candidates = candidate_actions(&context);
        assert_eq!(candidates.len(), 7, "fixture enumerates the closed loop");
        let scores = score_candidates(
            &candidates,
            &context,
            &Weights::default(),
            &PerturbationSpec::ZERO,
        );
        assert_eq!(scores.len(), 7);
        // Pressures 500/250; distances 2/2/4; every fixture site is
        // available; the Work site carries progress 3 (recorded, weight 0).
        let expected = [
            (ActionKind::Move, 0_i64),  // −5·2 + 10·1
            (ActionKind::Move, 0),      // −5·2 + 10·1
            (ActionKind::Move, -10),    // −5·4 + 10·1
            (ActionKind::Eat, 4_990),   // 10·500 − 5·2
            (ActionKind::Sleep, 2_490), // 10·250 − 5·2
            (ActionKind::Work, 2_280),  // −5·4 + 2_300·1 (+ 0·3)
            (ActionKind::Idle, -50),    // −50·1
        ];
        for (entry, (kind, base)) in scores.iter().zip(expected) {
            assert_eq!(entry.candidate().kind(), kind);
            assert_eq!(entry.base().get(), base, "base term for {kind:?}");
            assert_eq!(entry.perturbation(), 0, "zero spec applies nothing");
            assert_eq!(entry.score().get(), base);
            assert_eq!(entry.trace().total(), Some(base));
            assert_eq!(entry.trace().perturbation(), Some(0));
            assert_eq!(entry.trace().candidate(), entry.candidate());
            // The trace lists every factor in FactorId declaration order,
            // and the recorded contributions sum to the base term.
            let ids: Vec<FactorId> = entry
                .trace()
                .factors()
                .iter()
                .map(|evaluation| evaluation.input().factor())
                .collect();
            assert_eq!(
                ids,
                vec![
                    FactorId::Hunger,
                    FactorId::Fatigue,
                    FactorId::DistanceToTarget,
                    FactorId::SiteAvailable,
                    FactorId::WorkProgress,
                ]
            );
            let contributions: i64 = entry
                .trace()
                .factors()
                .iter()
                .map(|evaluation| {
                    evaluation
                        .contribution()
                        .expect("scoring populates contributions")
                })
                .sum();
            assert_eq!(contributions, base, "contributions sum to the base");
        }
        // A weight-0 factor is still recorded and contributes exactly 0:
        // Hunger on Move (input 500), WorkProgress on Work (input 3).
        let move_hunger = &scores[0].trace().factors()[0];
        assert_eq!(move_hunger.input(), FactorInput::new(FactorId::Hunger, 500));
        assert_eq!(move_hunger.contribution(), Some(0));
        let work_progress = &scores[5].trace().factors()[4];
        assert_eq!(
            work_progress.input(),
            FactorInput::new(FactorId::WorkProgress, 3)
        );
        assert_eq!(work_progress.contribution(), Some(0));
        // Idle has no target and the flat −50 baseline.
        assert_eq!(scores[6].candidate().target(), None);
    }

    #[test]
    fn extreme_weights_saturate_instead_of_wrapping() {
        let (map, sites, origin) = fixture();
        let context = context(origin, needs_with(50_000, 25_000), &sites, &map);
        let candidates = candidate_actions(&context);
        // Every candidate has at least one nonzero input (pressures are
        // nonzero in this fixture), so saturating_mul hits the bounds and
        // saturating_add keeps them there — no wrap, no panic.
        let first = score_candidates(
            &candidates,
            &context,
            &uniform_weights(i64::MAX),
            &PerturbationSpec::ZERO,
        );
        for entry in &first {
            assert_eq!(entry.base().get(), i64::MAX);
            assert_eq!(entry.score().get(), i64::MAX);
            assert_eq!(entry.trace().total(), Some(i64::MAX));
        }
        let second = score_candidates(
            &candidates,
            &context,
            &uniform_weights(i64::MAX),
            &PerturbationSpec::ZERO,
        );
        assert_eq!(first, second, "saturation is deterministic");

        let negative = score_candidates(
            &candidates,
            &context,
            &uniform_weights(i64::MIN),
            &PerturbationSpec::ZERO,
        );
        for entry in &negative {
            assert_eq!(entry.base().get(), i64::MIN);
            assert_eq!(entry.score().get(), i64::MIN);
        }
    }

    #[test]
    fn scoring_and_selection_are_integer_only() {
        let (map, sites, origin) = fixture();
        let context = context(origin, needs_with(50_000, 25_000), &sites, &map);
        let candidates = candidate_actions(&context);
        let spec =
            PerturbationSpec::new(7, PerturbationRange::Bounded(MAX_EPSILON)).expect("in range");
        let selection =
            select_action(&candidates, &context, &Weights::default(), &spec).expect("non-empty");
        let value = serde_json::to_value(&selection).expect("serialize selection");
        assert_no_floats(&value);
    }

    /// Recursively asserts that no JSON number in `value` is a float.
    fn assert_no_floats(value: &serde_json::Value) {
        match value {
            serde_json::Value::Number(number) => {
                assert!(!number.is_f64(), "selection contains a float: {number}");
                assert!(number.is_i64() || number.is_u64());
            }
            serde_json::Value::Array(items) => {
                for item in items {
                    assert_no_floats(item);
                }
            }
            serde_json::Value::Object(map) => {
                for item in map.values() {
                    assert_no_floats(item);
                }
            }
            serde_json::Value::Null | serde_json::Value::Bool(_) | serde_json::Value::String(_) => {
            }
        }
    }

    #[test]
    fn zero_perturbation_mode_ignores_the_seed() {
        let (map, sites, origin) = fixture();
        let context = context(origin, needs_with(50_000, 25_000), &sites, &map);
        let candidates = candidate_actions(&context);
        let weights = Weights::default();
        // The pure base-term argmax under the documented tie-break.
        let base_scores =
            score_candidates(&candidates, &context, &weights, &PerturbationSpec::ZERO);
        let base_winner = base_scores
            .iter()
            .max_by(|left, right| {
                left.base()
                    .cmp(&right.base())
                    .then(right.candidate().order().cmp(&left.candidate().order()))
            })
            .expect("non-empty")
            .candidate();
        for seed in [0, 1, u64::MAX] {
            let spec = PerturbationSpec::new(seed, PerturbationRange::Zero).expect("zero is valid");
            let selection = select_action(&candidates, &context, &weights, &spec)
                .expect("fixture candidates are non-empty");
            assert_eq!(
                selection.candidate(),
                base_winner,
                "seed {seed} must not change the zero-mode winner"
            );
            assert_eq!(selection.candidate().kind(), ActionKind::Eat);
            for entry in selection.all_scores() {
                assert_eq!(entry.perturbation(), 0);
                assert_eq!(entry.score(), entry.base());
            }
        }
        assert_eq!(PerturbationSpec::default(), PerturbationSpec::ZERO);
        assert_eq!(PerturbationSpec::zero(), PerturbationSpec::ZERO);
        assert_eq!(PerturbationSpec::ZERO.seed(), 0);
        assert_eq!(PerturbationSpec::ZERO.range(), PerturbationRange::Zero);
    }

    #[test]
    fn perturbation_spec_validates_the_range() {
        let spec = PerturbationSpec::new(7, PerturbationRange::Bounded(25)).expect("in range");
        assert_eq!(spec.seed(), 7);
        assert_eq!(spec.range(), PerturbationRange::Bounded(25));
        assert_eq!(
            PerturbationSpec::new(7, PerturbationRange::Bounded(-1)),
            Err(PerturbationError::EpsilonOutOfRange { epsilon: -1 }),
            "negative ε is rejected"
        );
        assert_eq!(
            PerturbationSpec::new(7, PerturbationRange::Bounded(MAX_EPSILON + 1)),
            Err(PerturbationError::EpsilonOutOfRange {
                epsilon: MAX_EPSILON + 1
            }),
            "ε above MAX_EPSILON is rejected"
        );
        assert!(PerturbationSpec::new(7, PerturbationRange::Bounded(MAX_EPSILON)).is_ok());
        assert!(PerturbationSpec::new(7, PerturbationRange::Bounded(0)).is_ok());
        // Bounded(0) behaves exactly like Zero.
        let (map, sites, origin) = fixture();
        let context = context(origin, needs_with(50_000, 25_000), &sites, &map);
        let candidates = candidate_actions(&context);
        let bounded_zero =
            PerturbationSpec::new(99, PerturbationRange::Bounded(0)).expect("in range");
        let scores = score_candidates(&candidates, &context, &Weights::default(), &bounded_zero);
        assert!(scores.iter().all(|entry| entry.perturbation() == 0));
    }

    #[test]
    fn malformed_perturbation_wire_is_rejected() {
        for encoded in [
            r#"{"seed":42,"range":{"Bounded":-1}}"#,
            r#"{"seed":42,"range":{"Bounded":101}}"#,
        ] {
            assert!(serde_json::from_str::<PerturbationSpec>(encoded).is_err());
        }
    }

    #[test]
    fn native_and_wire_epsilon_boundaries_agree() {
        #[derive(serde::Deserialize)]
        struct Request {
            spec: PerturbationSpec,
        }
        for epsilon in [i64::MIN, -1, 0, 100, 101, i64::MAX] {
            let native = PerturbationSpec::new(42, PerturbationRange::Bounded(epsilon));
            let range = serde_json::json!({"Bounded": epsilon});
            let spec = serde_json::json!({"seed": 42, "range": range});
            let decoded = serde_json::from_value::<PerturbationSpec>(spec.clone());
            let request = serde_json::from_value::<Request>(serde_json::json!({"spec": spec}));
            let valid = (0..=MAX_EPSILON).contains(&epsilon);
            assert_eq!(native.is_ok(), valid);
            assert_eq!(decoded.is_ok(), valid);
            assert_eq!(request.is_ok(), valid);
            assert_eq!(
                serde_json::from_value::<PerturbationRange>(range).is_ok(),
                valid
            );
            if valid {
                let native = native.expect("valid epsilon");
                assert_eq!(decoded.expect("valid wire"), native);
                assert_eq!(request.expect("valid nested wire").spec, native);
            } else {
                assert_eq!(
                    native,
                    Err(PerturbationError::EpsilonOutOfRange { epsilon })
                );
            }
        }
        for number in ["0.5", "18446744073709551616", "-9223372036854775809"] {
            let encoded = format!(r#"{{"seed":42,"range":{{"Bounded":{number}}}}}"#);
            assert!(serde_json::from_str::<PerturbationSpec>(&encoded).is_err());
            assert!(
                serde_json::from_str::<PerturbationRange>(&format!(r#"{{"Bounded":{number}}}"#))
                    .is_err()
            );
        }
        let zero = PerturbationSpec::new(42, PerturbationRange::Zero).expect("zero");
        let bounded =
            PerturbationSpec::new(42, PerturbationRange::Bounded(0)).expect("bounded zero");
        assert_ne!(
            serde_json::to_value(zero).expect("encode"),
            serde_json::to_value(bounded).expect("encode")
        );
    }

    #[test]
    fn selection_keys_are_a_set_not_vector_positions() {
        use crate::CandidateSetError::{DuplicateCandidate, DuplicateOrder, OrderOutOfRange};
        let (map, sites, origin) = fixture();
        let context = context(origin, Needs::default(), &sites, &map);
        let movement = |order| {
            ActionCandidate::new(ActionKind::Move, Some(origin), order).expect("valid shape")
        };
        let idle =
            |order| ActionCandidate::new(ActionKind::Idle, None, order).expect("valid shape");
        let forward = [movement(0), idle(1)];
        let reverse = [idle(1), movement(0)];
        let choose = |candidates: &[ActionCandidate]| {
            select_action(
                candidates,
                &context,
                &flat_weights(),
                &PerturbationSpec::ZERO,
            )
        };
        let first = choose(&forward).expect("valid complete set");
        let second = choose(&reverse).expect("valid permutation");
        assert_eq!(first.candidate(), second.candidate());
        assert_eq!(first.trace().tie_break(), Some(TieBreakReason::StableOrder));
        assert_eq!(first.trace().tie_break(), second.trace().tie_break());
        assert_eq!(
            second
                .all_scores()
                .iter()
                .map(CandidateScore::candidate)
                .collect::<Vec<_>>(),
            reverse
        );
        assert_eq!(
            second
                .trace()
                .candidates()
                .iter()
                .map(crate::CandidateTrace::candidate)
                .collect::<Vec<_>>(),
            reverse
        );
        for (input, error) in [
            (vec![movement(0), idle(0)], DuplicateOrder { order: 0 }),
            (
                vec![movement(0), idle(2)],
                OrderOutOfRange { order: 2, len: 2 },
            ),
            (vec![idle(1)], OrderOutOfRange { order: 1, len: 1 }),
            (
                vec![movement(0), idle(u64::MAX)],
                OrderOutOfRange {
                    order: u64::MAX,
                    len: 2,
                },
            ),
            (
                vec![movement(u64::MAX), idle(u64::MAX)],
                DuplicateOrder { order: u64::MAX },
            ),
            (
                vec![movement(2), movement(0)],
                OrderOutOfRange { order: 2, len: 2 },
            ),
            (
                vec![movement(0), movement(1)],
                DuplicateCandidate {
                    kind: ActionKind::Move,
                    target: Some(origin),
                },
            ),
        ] {
            assert_eq!(choose(&input), Err(DecisionError::InvalidCandidates(error)));
        }
        // Scoring a diagnostic fragment deliberately does not require a complete set.
        let scores = score_candidates(
            &[idle(u64::MAX)],
            &context,
            &Weights::default(),
            &PerturbationSpec::ZERO,
        );
        assert_eq!(scores.len(), 1);
        assert_eq!(scores[0].candidate().order(), u64::MAX);
    }

    #[test]
    fn decoded_inputs_preserve_selection_for_zero_and_seeded_modes() {
        let (map, sites, origin) = fixture();
        let context = context(origin, needs_with(50_000, 25_000), &sites, &map);
        let candidates = candidate_actions(&context);
        let decoded: Vec<ActionCandidate> =
            serde_json::from_value(serde_json::to_value(&candidates).expect("encode"))
                .expect("valid provider output");
        for spec in [
            PerturbationSpec::ZERO,
            PerturbationSpec::new(42, PerturbationRange::Bounded(100)).expect("valid epsilon"),
        ] {
            let decoded_spec = serde_json::from_value(serde_json::to_value(spec).expect("encode"))
                .expect("valid spec");
            let first =
                select_action(&candidates, &context, &Weights::default(), &spec).expect("select");
            let second = select_action(&decoded, &context, &Weights::default(), &decoded_spec)
                .expect("select decoded");
            assert_eq!(first, second);
            let mut reversed = candidates.clone();
            reversed.reverse();
            let reordered = select_action(&reversed, &context, &Weights::default(), &spec)
                .expect("valid permutation");
            assert_eq!(reordered.candidate(), first.candidate());
            assert_eq!(reordered.score(), first.score());
            assert_eq!(reordered.trace().tie_break(), first.trace().tie_break());
            assert_eq!(
                serde_json::from_value::<Selection>(
                    serde_json::to_value(&reordered).expect("encode")
                )
                .expect("decode reordered selection"),
                reordered
            );
        }
    }

    #[test]
    fn selection_wire_rejects_ambiguous_or_conflicting_copies() {
        let (map, sites, origin) = fixture();
        let context = context(origin, needs_with(50_000, 25_000), &sites, &map);
        let selection = select_action(
            &candidate_actions(&context),
            &context,
            &Weights::default(),
            &PerturbationSpec::ZERO,
        )
        .expect("valid selection");
        let valid = serde_json::to_value(&selection).expect("encode");
        assert_eq!(
            serde_json::from_value::<Selection>(valid.clone()).expect("decode valid"),
            selection
        );
        for (pointer, value) in [
            ("/trace/selected", serde_json::json!(u64::MAX)),
            ("/trace/selected", serde_json::Value::Null),
            ("/trace/tie_break", serde_json::Value::Null),
            ("/trace/candidates/1/candidate/order", serde_json::json!(0)),
            ("/all_scores/1/candidate/order", serde_json::json!(0)),
            ("/candidate/order", serde_json::json!(u64::MAX)),
            ("/score", serde_json::json!(selection.score().get() + 1)),
            ("/all_scores/0/score", serde_json::json!(123_456)),
            ("/all_scores/0/trace/total", serde_json::json!(123_456)),
            (
                "/all_scores/0/trace/candidate/order",
                serde_json::json!(u64::MAX),
            ),
            ("/all_scores/0/perturbation", serde_json::json!(1)),
            ("/trace/candidates/0/total", serde_json::json!(123_456)),
            (
                "/trace/candidates/0/factors/0/input/input",
                serde_json::json!(1),
            ),
            ("/all_scores", serde_json::json!([])),
        ] {
            let mut invalid = valid.clone();
            *invalid.pointer_mut(pointer).expect("existing field") = value;
            assert!(
                serde_json::from_value::<Selection>(invalid).is_err(),
                "must reject corruption at {pointer}"
            );
        }
        let mut missing_tie = valid;
        missing_tie["trace"]
            .as_object_mut()
            .expect("trace object")
            .remove("tie_break");
        assert!(serde_json::from_value::<Selection>(missing_tie).is_err());
    }

    #[test]
    fn nonzero_perturbation_is_bounded_deterministic_and_seed_dependent() {
        let (map, sites, origin) = fixture();
        let context = context(origin, needs_with(50_000, 25_000), &sites, &map);
        let candidates = candidate_actions(&context);
        let spec = PerturbationSpec::new(42, PerturbationRange::Bounded(25)).expect("in range");
        let first = score_candidates(&candidates, &context, &Weights::default(), &spec);
        let second = score_candidates(&candidates, &context, &Weights::default(), &spec);
        assert_eq!(first, second, "the same seed reproduces identical scores");
        for entry in &first {
            assert!(
                (-25..=25).contains(&entry.perturbation()),
                "perturbation {} within [-25, 25]",
                entry.perturbation()
            );
            assert_eq!(
                entry.score().get(),
                entry.base().get().saturating_add(entry.perturbation())
            );
            assert_eq!(entry.trace().perturbation(), Some(entry.perturbation()));
        }
        assert!(
            first
                .iter()
                .any(|entry| entry.perturbation() != first[0].perturbation()),
            "the perturbation varies across candidates"
        );
        // Different seeds can reorder a tied field: with a flat weight table
        // every base is 0, so the winner is perturbation-decided.
        let winners: Vec<ActionCandidate> = (0..64)
            .map(|seed| {
                let spec =
                    PerturbationSpec::new(seed, PerturbationRange::Bounded(25)).expect("in range");
                select_action(&candidates, &context, &flat_weights(), &spec)
                    .expect("non-empty")
                    .candidate()
            })
            .collect();
        assert!(
            winners.iter().any(|winner| *winner != winners[0]),
            "64 seeds must reorder the tied field at least once"
        );
    }

    #[test]
    fn ties_break_by_stable_order_and_strict_maxima_are_unique() {
        let (map, sites, origin) = fixture();
        let context = context(origin, needs_with(50_000, 25_000), &sites, &map);
        let candidates = candidate_actions(&context);
        // Flat weights + zero perturbation: every candidate scores 0, so the
        // lowest enumeration key wins and the tie is reported.
        let tied = select_action(
            &candidates,
            &context,
            &flat_weights(),
            &PerturbationSpec::ZERO,
        )
        .expect("non-empty");
        assert_eq!(tied.candidate().order(), 0);
        assert_eq!(tied.candidate().kind(), ActionKind::Move);
        assert_eq!(tied.score().get(), 0);
        assert_eq!(tied.trace().tie_break(), Some(TieBreakReason::StableOrder));
        assert_eq!(tied.trace().selected(), Some(0));
        for _ in 0..8 {
            assert_eq!(
                select_action(
                    &candidates,
                    &context,
                    &flat_weights(),
                    &PerturbationSpec::ZERO
                )
                .expect("non-empty"),
                tied,
                "the tie break is reproducible"
            );
        }
        // The default-weights fixture has a strict maximum (Eat at 4_990).
        let unique = select_action(
            &candidates,
            &context,
            &Weights::default(),
            &PerturbationSpec::ZERO,
        )
        .expect("non-empty");
        assert_eq!(unique.candidate().kind(), ActionKind::Eat);
        assert_eq!(
            unique.trace().tie_break(),
            Some(TieBreakReason::UniqueMaximum)
        );
        assert_eq!(unique.trace().selected(), Some(unique.candidate().order()));
    }

    #[test]
    fn closed_loop_selects_the_documented_phase_1_actions() {
        let (map, sites, origin) = fixture();
        let (ox, oy) = (origin.x(), origin.y());
        let weights = Weights::default();

        // High hunger selects Eat at the reachable Meal site.
        let hungry = context(origin, needs_with(100_000, 0), &sites, &map);
        let candidates = candidate_actions(&hungry);
        let selection = select_action(&candidates, &hungry, &weights, &PerturbationSpec::ZERO)
            .expect("non-empty");
        assert_eq!(selection.candidate().kind(), ActionKind::Eat);
        assert_eq!(selection.candidate().target(), Some(coord(ox + 2, oy)));
        assert_eq!(selection.score().get(), 9_990);

        // High fatigue selects Sleep at the reachable Rest site.
        let tired = context(origin, needs_with(0, 100_000), &sites, &map);
        let candidates = candidate_actions(&tired);
        let selection = select_action(&candidates, &tired, &weights, &PerturbationSpec::ZERO)
            .expect("non-empty");
        assert_eq!(selection.candidate().kind(), ActionKind::Sleep);
        assert_eq!(selection.candidate().target(), Some(coord(ox, oy + 2)));
        assert_eq!(selection.score().get(), 9_990);

        // Satisfied drives: the reachable Work site outranks the Idle
        // baseline (2_280 vs −50 under the default table).
        let satisfied = context(origin, Needs::default(), &sites, &map);
        let candidates = candidate_actions(&satisfied);
        let selection = select_action(&candidates, &satisfied, &weights, &PerturbationSpec::ZERO)
            .expect("non-empty");
        assert_eq!(selection.candidate().kind(), ActionKind::Work);
        assert_eq!(selection.score().get(), 2_280);
        let idle = selection
            .all_scores()
            .iter()
            .find(|entry| entry.candidate().kind() == ActionKind::Idle)
            .expect("the Idle baseline is always scored");
        assert_eq!(idle.score().get(), -50);
        assert!(idle.score() < selection.score());
    }

    #[test]
    fn one_second_advance_still_selects_work() {
        let (map, sites, origin) = fixture();
        // Fresh Needs advance to raw hunger=1/fatigue=2 and integer pressure
        // 0/0; this must still prefer the reachable Work site.
        let needs =
            Needs::default().advance(SimDuration::from_seconds(1).expect("one second is valid"));
        assert_eq!((needs.hunger().raw(), needs.fatigue().raw()), (1, 2));
        assert_eq!(needs.hunger_pressure(), 0);
        assert_eq!(needs.fatigue_pressure(), 0);
        let fresh_context = context(origin, needs, &sites, &map);
        let candidates = candidate_actions(&fresh_context);
        let selection = select_action(
            &candidates,
            &fresh_context,
            &Weights::default(),
            &PerturbationSpec::ZERO,
        )
        .expect("non-empty");
        assert_eq!(selection.candidate().kind(), ActionKind::Work);

        // Keep the additional near-boundary elapsed-time regression.
        let near = needs_with(19_900, 0)
            .advance(SimDuration::from_seconds(1).expect("one second is valid"));
        let near_context = context(origin, near, &sites, &map);
        assert_eq!(near.hunger_pressure(), 199);
        assert_eq!(
            select_action(
                &candidate_actions(&near_context),
                &near_context,
                &Weights::default(),
                &PerturbationSpec::ZERO,
            )
            .expect("non-empty")
            .candidate()
            .kind(),
            ActionKind::Work
        );
    }

    #[test]
    fn approved_need_work_threshold_sweep_and_ties() {
        let (map, sites, origin) = fixture();
        let weights = Weights::default();
        let raw = |pressure: i64| NeedValue::from_raw(pressure * 100).expect("pressure bound");
        for pressure in 0..=1_000 {
            let needs = Needs::new(raw(pressure), NeedValue::MIN);
            let context = context(origin, needs, &sites, &map);
            let selection = select_action(
                &candidate_actions(&context),
                &context,
                &weights,
                &PerturbationSpec::ZERO,
            )
            .expect("work is always available");
            assert_eq!(
                select_action(
                    &candidate_actions(&context),
                    &context,
                    &weights,
                    &PerturbationSpec::ZERO
                )
                .expect("repeat selection"),
                selection
            );
            let expected_score = if pressure <= 228 {
                2_280
            } else {
                10 * pressure - 10
            };
            assert_eq!(selection.score().get(), expected_score);
            if pressure <= 228 {
                assert_eq!(selection.candidate().kind(), ActionKind::Work);
                assert_eq!(
                    selection.trace().tie_break(),
                    Some(TieBreakReason::UniqueMaximum)
                );
            } else if pressure == 229 {
                assert_eq!(selection.candidate().kind(), ActionKind::Eat);
                assert_eq!(
                    selection.trace().tie_break(),
                    Some(TieBreakReason::StableOrder)
                );
                assert_eq!(selection.score().get(), 2_280);
            } else {
                assert_eq!(selection.candidate().kind(), ActionKind::Eat);
                assert_eq!(
                    selection.trace().tie_break(),
                    Some(TieBreakReason::UniqueMaximum)
                );
            }
        }
    }

    #[test]
    fn fatigue_work_threshold_sweep_and_ties() {
        let (map, sites, origin) = fixture();
        let weights = Weights::default();
        for pressure in 0..=1_000 {
            let needs = Needs::new(
                NeedValue::MIN,
                NeedValue::from_raw(pressure * 100).expect("pressure bound"),
            );
            let context = context(origin, needs, &sites, &map);
            let selection = select_action(
                &candidate_actions(&context),
                &context,
                &weights,
                &PerturbationSpec::ZERO,
            )
            .expect("work is always available");
            assert_eq!(
                select_action(
                    &candidate_actions(&context),
                    &context,
                    &weights,
                    &PerturbationSpec::ZERO
                )
                .expect("repeat selection"),
                selection
            );
            let expected_score = if pressure <= 228 {
                2_280
            } else {
                10 * pressure - 10
            };
            assert_eq!(selection.score().get(), expected_score);
            assert_eq!(
                selection.candidate().kind(),
                if pressure <= 228 {
                    ActionKind::Work
                } else {
                    ActionKind::Sleep
                }
            );
            assert_eq!(
                selection.trace().tie_break(),
                if pressure == 229 {
                    Some(TieBreakReason::StableOrder)
                } else {
                    Some(TieBreakReason::UniqueMaximum)
                }
            );
        }
    }

    #[test]
    fn low_need_margin_exceeds_every_allowed_pairwise_perturbation() {
        let (map, sites, origin) = fixture();
        let weights = Weights::default();
        let p200 = context(origin, needs_with(20_000, 20_000), &sites, &map);
        let p200_selection = select_action(
            &candidate_actions(&p200),
            &p200,
            &weights,
            &PerturbationSpec::ZERO,
        )
        .expect("non-empty");
        let score_for = |kind| {
            p200_selection
                .all_scores()
                .iter()
                .find(|entry| entry.candidate().kind() == kind)
                .expect("reachable candidate")
                .score()
                .get()
        };
        let work_score = score_for(ActionKind::Work);
        let need_score = score_for(ActionKind::Eat);
        assert_eq!(score_for(ActionKind::Sleep), need_score);
        assert_eq!(
            (work_score, need_score, work_score - need_score),
            (2_280, 1_990, 290)
        );
        assert!(work_score - need_score > 2 * MAX_EPSILON);
        for _ in 0..8 {
            assert_eq!(
                select_action(
                    &candidate_actions(&p200),
                    &p200,
                    &weights,
                    &PerturbationSpec::ZERO
                )
                .expect("non-empty"),
                p200_selection
            );
        }
    }

    #[test]
    fn both_low_needs_and_raw_values_select_work() {
        let (map, sites, origin) = fixture();
        let weights = Weights::default();
        for hunger_pressure in [0, 1, 199, 200] {
            for fatigue_pressure in [0, 1, 199, 200] {
                let context = context(
                    origin,
                    needs_with(hunger_pressure * 100, fatigue_pressure * 100),
                    &sites,
                    &map,
                );
                let selected = select_action(
                    &candidate_actions(&context),
                    &context,
                    &weights,
                    &PerturbationSpec::ZERO,
                )
                .expect("reachable work");
                assert_eq!(selected.candidate().kind(), ActionKind::Work);
                assert_eq!(
                    selected.trace().tie_break(),
                    Some(TieBreakReason::UniqueMaximum)
                );
                assert_eq!(selected.score().get(), 2_280);
            }
        }
        // These are raw values (not pressure); keep the earlier small-input
        // cases, including all 1/2/99 pairs, in addition to the pressure grid.
        for (hunger, fatigue) in [
            (0, 0),
            (0, 1),
            (0, 199),
            (0, 200),
            (1, 0),
            (199, 0),
            (200, 0),
            (1, 1),
            (1, 2),
            (1, 99),
            (2, 1),
            (2, 2),
            (2, 99),
            (99, 1),
            (99, 2),
            (99, 99),
        ] {
            let needs = needs_with(hunger, fatigue);
            let context = context(origin, needs, &sites, &map);
            let selection = select_action(
                &candidate_actions(&context),
                &context,
                &weights,
                &PerturbationSpec::ZERO,
            )
            .expect("work is always available");
            assert_eq!(selection.candidate().kind(), ActionKind::Work);
            assert_eq!(
                selection.trace().tie_break(),
                Some(TieBreakReason::UniqueMaximum)
            );
        }
    }

    #[test]
    fn high_need_axes_and_equal_ties_are_stable() {
        let (map, sites, origin) = fixture();
        let weights = Weights::default();
        for pressure in [699, 700, 900, 1_000] {
            let equal = context(
                origin,
                needs_with(pressure * 100, pressure * 100),
                &sites,
                &map,
            );
            let selected = select_action(
                &candidate_actions(&equal),
                &equal,
                &weights,
                &PerturbationSpec::ZERO,
            )
            .expect("non-empty");
            assert_eq!(selected.candidate().kind(), ActionKind::Eat);
            assert_eq!(
                selected.trace().tie_break(),
                Some(TieBreakReason::StableOrder)
            );
            let hunger_high = context(
                origin,
                needs_with(pressure * 100, (pressure - 1) * 100),
                &sites,
                &map,
            );
            assert_eq!(
                select_action(
                    &candidate_actions(&hunger_high),
                    &hunger_high,
                    &weights,
                    &PerturbationSpec::ZERO
                )
                .expect("non-empty")
                .candidate()
                .kind(),
                ActionKind::Eat
            );
            let fatigue_high = context(
                origin,
                needs_with((pressure - 1) * 100, pressure * 100),
                &sites,
                &map,
            );
            assert_eq!(
                select_action(
                    &candidate_actions(&fatigue_high),
                    &fatigue_high,
                    &weights,
                    &PerturbationSpec::ZERO
                )
                .expect("non-empty")
                .candidate()
                .kind(),
                ActionKind::Sleep
            );
        }
    }

    #[test]
    fn reachable_distance_and_no_site_idle_are_scored() {
        let (map, sites, origin) = fixture();
        let fixture_context = context(origin, Needs::default(), &sites, &map);
        let scored = score_candidates(
            &candidate_actions(&fixture_context),
            &fixture_context,
            &Weights::default(),
            &PerturbationSpec::ZERO,
        );
        let work = scored
            .iter()
            .find(|entry| entry.candidate().kind() == ActionKind::Work)
            .expect("reachable work site");
        assert_eq!(work.trace().factors()[2].input().input(), 4);
        assert_eq!(work.trace().factors()[3].input().input(), 1);
        assert_eq!(work.score().get(), 2_280);

        let empty_sites = ActivitySites::new(Vec::new()).expect("empty site set");
        let empty_context = context(origin, Needs::default(), &empty_sites, &map);
        let selected = select_action(
            &candidate_actions(&empty_context),
            &empty_context,
            &Weights::default(),
            &PerturbationSpec::ZERO,
        )
        .expect("idle baseline");
        assert_eq!(selected.candidate().kind(), ActionKind::Idle);
        assert_eq!(selected.score().get(), -50);

        // Use the real generated-map/pathfinding provider to show distance
        // changes the 229-pressure crossover, not a fabricated unreachable
        // candidate. The reference Meal is distance 2; a farther reachable
        // Meal loses to Work at the same pressure.
        let far_target = distant_reachable_coord(&map, origin);
        let far_sites = ActivitySites::new(vec![
            ActivitySite::new(&map, far_target, SiteKind::Meal).expect("far target walkable"),
            ActivitySite::new(&map, coord(origin.x() + 2, origin.y() + 2), SiteKind::Work)
                .expect("work target walkable"),
        ])
        .expect("distinct sites");
        let far_needs = needs_with(22_900, 0);
        let far_context = context(origin, far_needs, &far_sites, &map);
        let far_selection = select_action(
            &candidate_actions(&far_context),
            &far_context,
            &Weights::default(),
            &PerturbationSpec::ZERO,
        )
        .expect("far candidates are reachable");
        assert!(far_target.x().abs_diff(origin.x()) + far_target.y().abs_diff(origin.y()) >= 20);
        assert_eq!(far_selection.candidate().kind(), ActionKind::Work);
        let far_eat = far_selection
            .all_scores()
            .iter()
            .find(|entry| entry.candidate().kind() == ActionKind::Eat)
            .expect("far reachable Eat");
        assert_eq!(far_eat.trace().factors()[3].input().input(), 1);
        let distance = far_eat.trace().factors()[2].input().input();
        assert!(distance >= 20);
        assert_eq!(far_eat.score().get(), 2_290 - 5 * distance);
        assert!(far_eat.score().get() < 2_280);

        // Even a distant reachable critical need beats Work. Exercise both
        // needs through real sites/provider output, not a fabricated target.
        for (site_kind, action_kind, needs) in [
            (SiteKind::Meal, ActionKind::Eat, needs_with(90_000, 0)),
            (SiteKind::Rest, ActionKind::Sleep, needs_with(0, 90_000)),
        ] {
            let critical_sites = ActivitySites::new(vec![
                ActivitySite::new(&map, far_target, site_kind).expect("reachable far site"),
                ActivitySite::new(&map, coord(origin.x() + 2, origin.y() + 2), SiteKind::Work)
                    .expect("reachable work"),
            ])
            .expect("distinct sites");
            let critical_context = context(origin, needs, &critical_sites, &map);
            let selected = select_action(
                &candidate_actions(&critical_context),
                &critical_context,
                &Weights::default(),
                &PerturbationSpec::ZERO,
            )
            .expect("critical candidate exists");
            assert_eq!(selected.candidate().kind(), action_kind);
            assert_eq!(selected.candidate().target(), Some(far_target));
            let critical = selected
                .all_scores()
                .iter()
                .find(|entry| entry.candidate().kind() == action_kind)
                .expect("critical need trace");
            assert_eq!(critical.trace().factors()[2].input().input(), distance);
            assert_eq!(critical.trace().factors()[3].input().input(), 1);
            assert_eq!(critical.score().get(), 9_000 - 5 * distance);
            // Worst legal Manhattan distance is 254. This bound proves the
            // Work comparison for every permitted perturbation, not sampled seeds.
            assert!(critical.score().get() >= 7_730);
            assert!(critical.score().get() - 2_300 > 2 * MAX_EPSILON);
        }
    }

    #[test]
    fn selection_is_byte_identical_across_repeated_runs() {
        let (map, sites, origin) = fixture();
        let context = context(origin, needs_with(50_000, 25_000), &sites, &map);
        let candidates = candidate_actions(&context);
        let spec = PerturbationSpec::new(u64::MAX, PerturbationRange::Bounded(MAX_EPSILON))
            .expect("in range");
        let first =
            select_action(&candidates, &context, &Weights::default(), &spec).expect("non-empty");
        for _ in 0..8 {
            assert_eq!(
                select_action(&candidates, &context, &Weights::default(), &spec)
                    .expect("non-empty"),
                first
            );
        }
        let first_json = serde_json::to_vec(&first).expect("serialize selection");
        let second =
            select_action(&candidates, &context, &Weights::default(), &spec).expect("non-empty");
        let second_json = serde_json::to_vec(&second).expect("serialize selection");
        assert_eq!(
            first_json, second_json,
            "serialization must be byte-identical"
        );
    }

    #[test]
    fn empty_candidate_set_returns_the_documented_error() {
        let (map, sites, origin) = fixture();
        let context = context(origin, Needs::default(), &sites, &map);
        assert_eq!(
            select_action(&[], &context, &Weights::default(), &PerturbationSpec::ZERO),
            Err(DecisionError::EmptyCandidates),
            "no untraced action is synthesized"
        );
        assert!(
            score_candidates(&[], &context, &Weights::default(), &PerturbationSpec::ZERO)
                .is_empty()
        );
        assert_eq!(
            DecisionError::EmptyCandidates.to_string(),
            "candidate set is empty; no action can be selected"
        );
        let error: &dyn std::error::Error = &DecisionError::EmptyCandidates;
        assert!(!error.to_string().is_empty());
    }

    #[test]
    fn all_scores_cover_every_candidate_in_order_and_select_the_maximum() {
        let (map, sites, origin) = fixture();
        let context = context(origin, needs_with(50_000, 25_000), &sites, &map);
        let candidates = candidate_actions(&context);
        let spec = PerturbationSpec::new(3, PerturbationRange::Bounded(50)).expect("in range");
        let selection =
            select_action(&candidates, &context, &Weights::default(), &spec).expect("non-empty");
        // One entry per candidate, in input order.
        assert_eq!(selection.all_scores().len(), candidates.len());
        for (entry, candidate) in selection.all_scores().iter().zip(&candidates) {
            assert_eq!(entry.candidate(), *candidate);
            assert_eq!(
                entry.score().get(),
                entry.base().get().saturating_add(entry.perturbation()),
                "score == base + perturbation"
            );
        }
        // The winner is the maximum under the documented tie-break.
        let expected = selection
            .all_scores()
            .iter()
            .max_by(|left, right| {
                left.score()
                    .cmp(&right.score())
                    .then(right.candidate().order().cmp(&left.candidate().order()))
            })
            .expect("non-empty");
        assert_eq!(selection.candidate(), expected.candidate());
        assert_eq!(selection.score(), expected.score());
        // The full trace mirrors the score list and the outcome.
        assert_eq!(selection.trace().candidates().len(), candidates.len());
        for (trace, entry) in selection
            .trace()
            .candidates()
            .iter()
            .zip(selection.all_scores())
        {
            assert_eq!(trace.candidate(), entry.candidate());
            assert_eq!(trace.total(), Some(entry.score().get()));
            assert_eq!(trace.perturbation(), Some(entry.perturbation()));
        }
        assert_eq!(
            selection.trace().selected(),
            Some(selection.candidate().order())
        );
    }

    #[test]
    fn utility_types_serde_round_trip() {
        let score = UtilityScore(-12_345);
        let encoded = serde_json::to_string(&score).expect("serialize score");
        assert_eq!(
            serde_json::from_str::<UtilityScore>(&encoded).expect("deserialize score"),
            score
        );

        for range in [PerturbationRange::Zero, PerturbationRange::Bounded(100)] {
            let encoded = serde_json::to_string(&range).expect("serialize range");
            assert_eq!(
                serde_json::from_str::<PerturbationRange>(&encoded).expect("deserialize range"),
                range
            );
        }
        assert_eq!(
            serde_json::to_string(&PerturbationRange::Zero).expect("serialize"),
            "\"Zero\""
        );
        assert_eq!(
            serde_json::to_string(&PerturbationRange::Bounded(25)).expect("serialize"),
            "{\"Bounded\":25}"
        );

        let spec = PerturbationSpec::new(7, PerturbationRange::Bounded(25)).expect("in range");
        let encoded = serde_json::to_string(&spec).expect("serialize spec");
        assert_eq!(encoded, "{\"seed\":7,\"range\":{\"Bounded\":25}}");
        let restored: PerturbationSpec = serde_json::from_str(&encoded).expect("deserialize spec");
        assert_eq!(restored.seed(), 7);
        assert_eq!(restored.range(), PerturbationRange::Bounded(25));

        let factor_weights = FactorWeights::new(1, -2, 3, -4, 5);
        let encoded = serde_json::to_string(&factor_weights).expect("serialize factor weights");
        assert_eq!(
            encoded,
            "{\"hunger\":1,\"fatigue\":-2,\"distance_to_target\":3,\
             \"site_available\":-4,\"work_progress\":5}"
        );
        assert_eq!(
            serde_json::from_str::<FactorWeights>(&encoded).expect("deserialize factor weights"),
            factor_weights
        );

        let weights = Weights::default();
        let encoded = serde_json::to_string(&weights).expect("serialize weights");
        assert!(
            encoded.contains("\"eat_weights\"") && encoded.contains("\"idle_weights\""),
            "per-kind keys are stable: {encoded}"
        );
        assert_eq!(
            serde_json::from_str::<Weights>(&encoded).expect("deserialize weights"),
            weights
        );

        let (map, sites, origin) = fixture();
        let context = context(origin, needs_with(50_000, 25_000), &sites, &map);
        let candidates = candidate_actions(&context);
        let spec = PerturbationSpec::new(11, PerturbationRange::Bounded(10)).expect("in range");
        let selection = select_action(&candidates, &context, &weights, &spec).expect("non-empty");
        let encoded = serde_json::to_string(&selection).expect("serialize selection");
        assert_eq!(
            serde_json::from_str::<Selection>(&encoded).expect("deserialize selection"),
            selection
        );
        let candidate_score = &selection.all_scores()[0];
        let encoded = serde_json::to_string(candidate_score).expect("serialize candidate score");
        assert_eq!(
            serde_json::from_str::<CandidateScore>(&encoded).expect("deserialize candidate score"),
            *candidate_score
        );
    }
}
