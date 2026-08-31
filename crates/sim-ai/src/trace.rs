// Authored by Kimi Code (AI coding agent) — task CHRON-025.
// Extended by Kimi Code (AI coding agent) — task CHRON-026.
//! Decision-trace contract for the Phase 1 Utility AI (CHRON-025, ADR-0014).
//!
//! A [`DecisionTrace`] is the first-class, complete, ordered record of every
//! factor input behind a decision, so Developer Mode's "Why" (Master Spec
//! §72) can show the full weighted calculation and the player-side
//! simplified Why. This module defines the bounded schema. The public
//! CHRON-025 constructors populate only candidate and factor-*input* data:
//! every contribution, total, perturbation, selection, and tie-break field
//! is `None` from them and is filled by CHRON-026 scoring and selection
//! ([`crate::utility`]) through crate-internal constructors
//! ([`FactorEvaluation::scored`], [`CandidateTrace::scored`],
//! [`DecisionTrace::decided`]). No score, weight, "best", tie-break, or RNG
//! is computed in this module, and CHRON-026 must not add hidden inputs
//! beyond the [`FactorId`] set defined here.
//!
//! All factor inputs are bounded integers; no float type appears anywhere in
//! the trace (consistent with CHRON-022 and the Chaos Test, Master Spec
//! §76). Traces are runtime diagnostic data: they are *not* durable Event
//! Store records (ADR-0014).

use serde::{Deserialize, Deserializer, Serialize};
use std::fmt::{Display, Formatter};

use palimpsest_sim_world::ActivitySite;

use crate::action::{
    ActionCandidate, CandidateContext, CandidateSetError, is_reachable, manhattan_distance,
    validate_candidate_set,
};

/// The exact set of Phase 1 decision factors (CHRON-025, ADR-0014).
///
/// Declaration order is the canonical trace order: every candidate's trace
/// lists every factor in exactly this order, with no per-kind factor lists —
/// a uniform complete set keeps the Why Inspector honest and comparable
/// across action kinds. Serde uses the default variant names as stable wire
/// keys.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum FactorId {
    /// Hunger pressure (`0..=1000`), from [`crate::Needs::hunger_pressure`].
    Hunger,
    /// Fatigue pressure (`0..=1000`), from [`crate::Needs::fatigue_pressure`].
    Fatigue,
    /// Manhattan distance from the person's location to the candidate
    /// target; `0` for the targetless `Idle` candidate.
    DistanceToTarget,
    /// `1` when the candidate's target resolves to an existing activity site
    /// reachable under the context's pathfinding budget, or when the
    /// candidate is `Idle` (the baseline is always available); `0` otherwise.
    SiteAvailable,
    /// The target `Work` site's observation counter (bounded by
    /// [`palimpsest_sim_world::WorkCounter::MAX`]) for `Work` candidates; `0`
    /// otherwise.
    WorkProgress,
}

/// All factors in canonical (declaration) order; single source for the
/// uniform trace layout.
const FACTOR_ORDER: [FactorId; 5] = [
    FactorId::Hunger,
    FactorId::Fatigue,
    FactorId::DistanceToTarget,
    FactorId::SiteAvailable,
    FactorId::WorkProgress,
];

/// One bounded integer factor input: what the world looked like for one
/// factor at decision time.
///
/// Inputs are observations, not scores; CHRON-026 turns them into weighted
/// contributions. Serde encodes the two fields as-is.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct FactorInput {
    factor: FactorId,
    input: i64,
}

impl FactorInput {
    /// Creates an input record for one factor.
    #[must_use]
    pub const fn new(factor: FactorId, input: i64) -> Self {
        Self { factor, input }
    }

    /// The factor this input belongs to.
    #[must_use]
    pub const fn factor(&self) -> FactorId {
        self.factor
    }

    /// The bounded integer input value.
    #[must_use]
    pub const fn input(&self) -> i64 {
        self.input
    }
}

/// One factor's input plus its evaluated contribution to the candidate's
/// total.
///
/// `contribution` is always `None` when constructed by CHRON-025; CHRON-026
/// supplies the weighted value during scoring.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct FactorEvaluation {
    input: FactorInput,
    contribution: Option<i64>,
}

impl FactorEvaluation {
    /// Creates an evaluation from an input with no contribution yet
    /// (CHRON-026 populates it during scoring).
    #[must_use]
    pub const fn new(input: FactorInput) -> Self {
        Self {
            input,
            contribution: None,
        }
    }

    /// Creates an evaluation with the weighted contribution populated.
    ///
    /// Crate-internal CHRON-026 scoring path: the public trace schema stays
    /// read-only, and only [`crate::utility`] constructs populated
    /// evaluations.
    #[must_use]
    pub(crate) const fn scored(input: FactorInput, contribution: i64) -> Self {
        Self {
            input,
            contribution: Some(contribution),
        }
    }

    /// The recorded factor input.
    #[must_use]
    pub const fn input(&self) -> FactorInput {
        self.input
    }

    /// The weighted contribution; `None` until CHRON-026 scoring.
    #[must_use]
    pub const fn contribution(&self) -> Option<i64> {
        self.contribution
    }
}

/// The full trace of one candidate: the candidate itself and every factor
/// evaluation in [`FactorId`] declaration order.
///
/// `total` and `perturbation` are always `None` when constructed by the
/// CHRON-025 constructors; CHRON-026 scoring supplies the candidate's total
/// utility and the seeded perturbation value applied to it (ADR-0014).
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CandidateTrace {
    candidate: ActionCandidate,
    factors: Vec<FactorEvaluation>,
    total: Option<i64>,
    perturbation: Option<i64>,
}

impl CandidateTrace {
    /// Creates a candidate trace from factor evaluations with no total and
    /// no perturbation yet (CHRON-026 populates both during scoring).
    #[must_use]
    pub const fn new(candidate: ActionCandidate, factors: Vec<FactorEvaluation>) -> Self {
        Self {
            candidate,
            factors,
            total: None,
            perturbation: None,
        }
    }

    /// Creates a fully populated candidate trace.
    ///
    /// Crate-internal CHRON-026 scoring path: the public trace schema stays
    /// read-only, and only [`crate::utility`] constructs populated traces.
    #[must_use]
    pub(crate) const fn scored(
        candidate: ActionCandidate,
        factors: Vec<FactorEvaluation>,
        total: i64,
        perturbation: i64,
    ) -> Self {
        Self {
            candidate,
            factors,
            total: Some(total),
            perturbation: Some(perturbation),
        }
    }

    /// The traced candidate.
    #[must_use]
    pub const fn candidate(&self) -> ActionCandidate {
        self.candidate
    }

    /// Every factor evaluation, in [`FactorId`] declaration order.
    #[must_use]
    pub fn factors(&self) -> &[FactorEvaluation] {
        &self.factors
    }

    /// The candidate's total utility; `None` until CHRON-026 scoring.
    #[must_use]
    pub const fn total(&self) -> Option<i64> {
        self.total
    }

    /// The seeded perturbation applied to this candidate's base term;
    /// `None` until CHRON-026 scoring (ADR-0014: the perturbation is exposed
    /// in the trace).
    #[must_use]
    pub const fn perturbation(&self) -> Option<i64> {
        self.perturbation
    }
}

/// Why the selected candidate won when scores did not fully separate the
/// field (ADR-0014: "a single stable tie-break rule with documented
/// precedence").
///
/// Populated only by CHRON-026 selection; CHRON-025 defines the variants but
/// never constructs one. Serde uses the default variant names as stable wire
/// keys.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum TieBreakReason {
    /// Exactly one candidate held the maximum total; no tie occurred.
    UniqueMaximum,
    /// Equal maxima resolved by stable enumeration order (lowest
    /// [`ActionCandidate::order`]).
    StableOrder,
}

/// The bounded, cloneable record of one decision: every candidate considered
/// with its complete factor breakdown, plus the selection outcome.
///
/// CHRON-025 constructors produce the unpopulated schema: `selected` and
/// `tie_break` are `None` and every per-candidate contribution and total is
/// `None`. CHRON-026 extends this same crate to fill them during scoring and
/// selection; there are deliberately no mutation methods here. Traces are
/// surfaced read-only to Developer Tools and the Why Inspector and are never
/// mutated back into the simulation (ADR-0014).
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct DecisionTrace {
    candidates: Vec<CandidateTrace>,
    selected: Option<u64>,
    tie_break: Option<TieBreakReason>,
}

impl DecisionTrace {
    /// Creates a trace over `candidates` with no selection yet (CHRON-026
    /// populates `selected` and `tie_break` during selection).
    /// Empty and non-contiguous diagnostic fragments are valid.
    ///
    /// # Errors
    /// Rejects duplicate keys or duplicate action/target pairs.
    pub fn new(candidates: Vec<CandidateTrace>) -> Result<Self, TraceValidationError> {
        Self::from_parts(candidates, None, None)
    }

    fn from_parts(
        candidates: Vec<CandidateTrace>,
        selected: Option<u64>,
        tie_break: Option<TieBreakReason>,
    ) -> Result<Self, TraceValidationError> {
        let identities: Vec<_> = candidates.iter().map(CandidateTrace::candidate).collect();
        validate_candidate_set(&identities, selected.is_some())
            .map_err(TraceValidationError::InvalidCandidates)?;
        match (selected, tie_break) {
            (None, Some(_)) => return Err(TraceValidationError::UnexpectedTieBreak),
            (Some(_), None) => return Err(TraceValidationError::MissingTieBreak),
            (Some(order), Some(_)) => {
                if candidates.is_empty() {
                    return Err(TraceValidationError::EmptySelection);
                }
                if !identities
                    .iter()
                    .any(|candidate| candidate.order() == order)
                {
                    return Err(TraceValidationError::SelectedKeyMissing { order });
                }
            }
            (None, None) => {}
        }
        Ok(Self {
            candidates,
            selected,
            tie_break,
        })
    }

    /// Creates a trace with the selection outcome populated.
    ///
    /// Crate-internal CHRON-026 selection path: the public trace schema
    /// stays read-only, and only [`crate::utility`] constructs decided
    /// traces. The caller must have validated the complete candidate set;
    /// the selected key comes from that set, never from external input.
    #[must_use]
    pub(crate) const fn decided(
        candidates: Vec<CandidateTrace>,
        selected: u64,
        tie_break: TieBreakReason,
    ) -> Self {
        Self {
            candidates,
            selected: Some(selected),
            tie_break: Some(tie_break),
        }
    }

    /// Every candidate considered, in input order (keys need not be sorted).
    #[must_use]
    pub fn candidates(&self) -> &[CandidateTrace] {
        &self.candidates
    }

    /// The [`ActionCandidate::order`] key of the chosen candidate; `None`
    /// until CHRON-026 selection.
    #[must_use]
    pub const fn selected(&self) -> Option<u64> {
        self.selected
    }

    /// Why the selected candidate won; `None` until CHRON-026 selection.
    #[must_use]
    pub const fn tie_break(&self) -> Option<TieBreakReason> {
        self.tie_break
    }
}

impl<'de> Deserialize<'de> for DecisionTrace {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        struct Wire {
            candidates: Vec<CandidateTrace>,
            selected: Option<u64>,
            tie_break: Option<TieBreakReason>,
        }
        let wire = Wire::deserialize(deserializer)?;
        Self::from_parts(wire.candidates, wire.selected, wire.tie_break)
            .map_err(serde::de::Error::custom)
    }
}

/// Invalid diagnostic identity/correspondence, not a world-truth check.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TraceValidationError {
    /// Duplicate or invalid collection keys/pairs.
    InvalidCandidates(CandidateSetError),
    /// A decided trace cannot be empty.
    EmptySelection,
    /// The selected key does not identify a candidate.
    SelectedKeyMissing { order: u64 },
    /// Selected traces must explain their tie resolution.
    MissingTieBreak,
    /// Unselected fragments cannot report a tie resolution.
    UnexpectedTieBreak,
    /// Duplicated selection/score/trace fields disagree for a key.
    InconsistentSelection { order: u64 },
    /// The score and trace collections differ in length.
    CandidateCountMismatch,
}

impl Display for TraceValidationError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidCandidates(error) => Display::fmt(error, f),
            Self::EmptySelection => write!(f, "selected candidate set is empty"),
            Self::SelectedKeyMissing { order } => write!(f, "selected key {order} is absent"),
            Self::MissingTieBreak => write!(f, "selected trace requires a tie reason"),
            Self::UnexpectedTieBreak => write!(f, "unselected trace cannot have a tie reason"),
            Self::InconsistentSelection { order } => {
                write!(f, "selection copies disagree for order {order}")
            }
            Self::CandidateCountMismatch => write!(f, "score and trace candidate counts differ"),
        }
    }
}
impl std::error::Error for TraceValidationError {}

#[cfg(test)]
mod validation_tests {
    use super::*;
    use crate::ActionKind;

    fn fragment(order: u64) -> CandidateTrace {
        CandidateTrace::new(
            ActionCandidate::new(ActionKind::Idle, None, order).expect("valid idle"),
            Vec::new(),
        )
    }

    #[test]
    fn fragments_allow_arbitrary_keys_but_not_duplicates() {
        for traces in [Vec::new(), vec![fragment(6)], vec![fragment(u64::MAX)]] {
            let trace = DecisionTrace::new(traces).expect("valid fragment");
            let encoded = serde_json::to_value(&trace).expect("encode");
            assert_eq!(
                serde_json::from_value::<DecisionTrace>(encoded).expect("decode"),
                trace
            );
        }
        let duplicates = vec![fragment(6), fragment(6)];
        assert_eq!(
            DecisionTrace::new(duplicates.clone()),
            Err(TraceValidationError::InvalidCandidates(
                CandidateSetError::DuplicateOrder { order: 6 }
            ))
        );
        let value =
            serde_json::json!({"candidates": duplicates, "selected": null, "tie_break": null});
        assert!(serde_json::from_value::<DecisionTrace>(value).is_err());
        let repeated_pair = vec![fragment(6), fragment(7)];
        assert!(matches!(
            DecisionTrace::new(repeated_pair.clone()),
            Err(TraceValidationError::InvalidCandidates(
                CandidateSetError::DuplicateCandidate { .. }
            ))
        ));
        assert!(serde_json::from_value::<DecisionTrace>(serde_json::json!({"candidates": repeated_pair, "selected": null, "tie_break": null})).is_err());
    }

    #[test]
    fn selected_trace_requires_complete_identity_and_tie_reason() {
        let partial = DecisionTrace::new(vec![fragment(6)]).expect("partial");
        let mut value = serde_json::to_value(partial).expect("encode");
        value["tie_break"] = serde_json::json!("StableOrder");
        assert!(serde_json::from_value::<DecisionTrace>(value.clone()).is_err());
        value["selected"] = serde_json::json!(6);
        assert!(serde_json::from_value::<DecisionTrace>(value).is_err());
        let decided = serde_json::json!({"candidates": [fragment(0)], "selected": 0, "tie_break": "UniqueMaximum"});
        assert!(serde_json::from_value::<DecisionTrace>(decided.clone()).is_ok());
        for (key, replacement) in [
            ("selected", serde_json::json!(1)),
            ("tie_break", serde_json::Value::Null),
            ("candidates", serde_json::json!([])),
        ] {
            let mut invalid = decided.clone();
            invalid[key] = replacement;
            assert!(serde_json::from_value::<DecisionTrace>(invalid).is_err());
        }
    }
}

/// Records every factor input for `candidate` under `context`, in [`FactorId`]
/// declaration order (CHRON-025).
///
/// The set is uniform across all action kinds — every candidate, including
/// `Idle`, reports all five factors:
///
/// - [`FactorId::Hunger`]: `context.needs().hunger_pressure()` (`0..=1000`).
/// - [`FactorId::Fatigue`]: `context.needs().fatigue_pressure()` (`0..=1000`).
/// - [`FactorId::DistanceToTarget`]: Manhattan distance from
///   `context.location()` to the candidate target; `0` when the candidate
///   has no target (`Idle`).
/// - [`FactorId::SiteAvailable`]: `1` when the candidate has a target that
///   resolves to an existing site in `context.sites()` reachable via
///   `find_path` under `context.path_config()`; `0` otherwise (no site at
///   the target, or no path). A targetless `Idle` candidate records `1`:
///   the do-nothing baseline is always available.
/// - [`FactorId::WorkProgress`]: for a `Work` candidate whose target site
///   carries a work counter, that counter value (bounded by
///   [`palimpsest_sim_world::WorkCounter::MAX`]); `0` otherwise.
///
/// This function never computes a score, weight, contribution, or total, and
/// CHRON-026 must not add hidden inputs beyond this set.
#[must_use]
pub fn factor_inputs_for(
    candidate: &ActionCandidate,
    context: &CandidateContext<'_>,
) -> Vec<FactorInput> {
    FACTOR_ORDER
        .iter()
        .map(|factor| factor_input(*factor, candidate, context))
        .collect()
}

/// Builds the single-candidate [`DecisionTrace`] for `candidate` under
/// `context` (CHRON-025).
///
/// The trace lists every factor input in [`FactorId`] declaration order with
/// all scoring fields unset: every contribution is `None`, the candidate
/// total is `None`, and `selected`/`tie_break` are `None`. No score or
/// selection is computed; CHRON-026 supplies weights, contributions, totals,
/// and the winner over the multi-candidate trace built from
/// [`crate::candidate_actions`].
#[must_use]
pub fn trace_for(candidate: &ActionCandidate, context: &CandidateContext<'_>) -> DecisionTrace {
    let factors = factor_inputs_for(candidate, context)
        .into_iter()
        .map(FactorEvaluation::new)
        .collect();
    // A single validated candidate always forms a valid partial fragment,
    // irrespective of its enumeration key. No public input can panic here.
    DecisionTrace {
        candidates: vec![CandidateTrace::new(*candidate, factors)],
        selected: None,
        tie_break: None,
    }
}

/// Computes one factor input under the uniform semantics documented on
/// [`factor_inputs_for`].
fn factor_input(
    factor: FactorId,
    candidate: &ActionCandidate,
    context: &CandidateContext<'_>,
) -> FactorInput {
    let input = match factor {
        FactorId::Hunger => context.needs().hunger_pressure(),
        FactorId::Fatigue => context.needs().fatigue_pressure(),
        FactorId::DistanceToTarget => candidate.target().map_or(0, |target| {
            i64::from(manhattan_distance(context.location(), target))
        }),
        FactorId::SiteAvailable => i64::from(site_available(candidate, context)),
        FactorId::WorkProgress => work_progress(candidate, context),
    };
    FactorInput::new(factor, input)
}

/// Returns whether the candidate's target is an existing, reachable site.
///
/// `Idle` (no target) is the always-available baseline and records `true`.
/// Any other candidate requires both a site at the target coordinate and a
/// `find_path` path to it under the context's budget.
fn site_available(candidate: &ActionCandidate, context: &CandidateContext<'_>) -> bool {
    match candidate.target() {
        None => true,
        Some(target) => context.sites().site_at(target).is_some() && is_reachable(context, target),
    }
}

/// Returns the target `Work` site's observation counter, or zero.
///
/// Only a `Work` candidate whose target is an existing site carrying a
/// counter reports it; every other candidate records `0`.
fn work_progress(candidate: &ActionCandidate, context: &CandidateContext<'_>) -> i64 {
    if candidate.kind() != crate::action::ActionKind::Work {
        return 0;
    }
    let Some(target) = candidate.target() else {
        return 0;
    };
    context
        .sites()
        .site_at(target)
        .and_then(ActivitySite::work)
        .map_or(0, |counter| {
            i64::try_from(counter.get()).expect("WorkCounter::MAX fits i64")
        })
}

#[cfg(test)]
mod tests {
    use super::{
        CandidateTrace, DecisionTrace, FactorEvaluation, FactorId, FactorInput, TieBreakReason,
        factor_inputs_for, trace_for,
    };
    use crate::action::{ActionCandidate, ActionKind, CandidateContext, candidate_actions};
    use crate::needs::{NeedValue, Needs};
    use palimpsest_sim_world::{
        ActivitySite, ActivitySites, LocalCoord, PathConfig, SiteKind, WorkCounter, WorldGenConfig,
        WorldMap, WorldSeed,
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

    fn inputs_by_factor(
        candidate: &ActionCandidate,
        context: &CandidateContext<'_>,
    ) -> Vec<(FactorId, i64)> {
        factor_inputs_for(candidate, context)
            .iter()
            .map(|input| (input.factor(), input.input()))
            .collect()
    }

    #[test]
    fn factor_id_is_exactly_the_five_phase_1_factors_in_order() {
        // An exhaustive match without a wildcard fails to compile if a sixth
        // variant is ever added.
        let all = [
            FactorId::Hunger,
            FactorId::Fatigue,
            FactorId::DistanceToTarget,
            FactorId::SiteAvailable,
            FactorId::WorkProgress,
        ];
        let mut count = 0;
        for factor in all {
            match factor {
                FactorId::Hunger
                | FactorId::Fatigue
                | FactorId::DistanceToTarget
                | FactorId::SiteAvailable
                | FactorId::WorkProgress => count += 1,
            }
        }
        assert_eq!(count, 5);
        let keys = [
            (FactorId::Hunger, "\"Hunger\""),
            (FactorId::Fatigue, "\"Fatigue\""),
            (FactorId::DistanceToTarget, "\"DistanceToTarget\""),
            (FactorId::SiteAvailable, "\"SiteAvailable\""),
            (FactorId::WorkProgress, "\"WorkProgress\""),
        ];
        for (factor, expected) in keys {
            let encoded = serde_json::to_string(&factor).expect("serialize factor");
            assert_eq!(encoded, expected, "wire key changed for {factor:?}");
            assert_eq!(
                serde_json::from_str::<FactorId>(&encoded).expect("deserialize factor"),
                factor
            );
        }
    }

    #[test]
    fn factor_inputs_are_uniform_and_in_declaration_order_for_every_kind() {
        let (map, sites, origin) = fixture();
        let context = context(origin, needs_with(50_000, 25_000), &sites, &map);
        let candidates = candidate_actions(&context);
        assert_eq!(candidates.len(), 7, "fixture yields the full closed loop");
        for candidate in &candidates {
            let factors: Vec<FactorId> = factor_inputs_for(candidate, &context)
                .iter()
                .map(FactorInput::factor)
                .collect();
            assert_eq!(
                factors,
                vec![
                    FactorId::Hunger,
                    FactorId::Fatigue,
                    FactorId::DistanceToTarget,
                    FactorId::SiteAvailable,
                    FactorId::WorkProgress,
                ],
                "every candidate kind reports the complete factor set in order"
            );
        }
    }

    #[test]
    fn factor_inputs_record_the_documented_semantics() {
        let (map, sites, origin) = fixture();
        let (ox, oy) = (origin.x(), origin.y());
        let context = context(origin, needs_with(50_000, 25_000), &sites, &map);

        // Eat at the Meal site two steps east: pressure inputs come straight
        // from Needs (50_000/100_000 -> 500; 25_000 -> 250), distance is the
        // Manhattan distance, the site is reachable, and only Work carries a
        // progress counter.
        let eat = ActionCandidate::new(ActionKind::Eat, Some(coord(ox + 2, oy)), 0)
            .expect("valid diagnostic fixture");
        assert_eq!(
            inputs_by_factor(&eat, &context),
            vec![
                (FactorId::Hunger, 500),
                (FactorId::Fatigue, 250),
                (FactorId::DistanceToTarget, 2),
                (FactorId::SiteAvailable, 1),
                (FactorId::WorkProgress, 0),
            ]
        );

        // The Work candidate reports the site's recorded counter value.
        let work = ActionCandidate::new(ActionKind::Work, Some(coord(ox + 2, oy + 2)), 1)
            .expect("valid diagnostic fixture");
        assert_eq!(
            inputs_by_factor(&work, &context),
            vec![
                (FactorId::Hunger, 500),
                (FactorId::Fatigue, 250),
                (FactorId::DistanceToTarget, 4),
                (FactorId::SiteAvailable, 1),
                (FactorId::WorkProgress, 3),
            ]
        );

        // Idle: no target, zero distance, always-available baseline, no
        // progress.
        let idle =
            ActionCandidate::new(ActionKind::Idle, None, 6).expect("valid diagnostic fixture");
        assert_eq!(
            inputs_by_factor(&idle, &context),
            vec![
                (FactorId::Hunger, 500),
                (FactorId::Fatigue, 250),
                (FactorId::DistanceToTarget, 0),
                (FactorId::SiteAvailable, 1),
                (FactorId::WorkProgress, 0),
            ]
        );
    }

    #[test]
    fn site_available_distinguishes_unknown_unreachable_and_baseline() {
        let (map, sites, origin) = fixture();
        let (ox, oy) = (origin.x(), origin.y());
        let context = context(origin, Needs::default(), &sites, &map);

        // A walkable, reachable coordinate without a site is unavailable.
        let no_site = ActionCandidate::new(ActionKind::Move, Some(coord(ox + 1, oy)), 0)
            .expect("valid diagnostic fixture");
        assert_eq!(
            inputs_by_factor(&no_site, &context),
            vec![
                (FactorId::Hunger, 0),
                (FactorId::Fatigue, 0),
                (FactorId::DistanceToTarget, 1),
                (FactorId::SiteAvailable, 0),
                (FactorId::WorkProgress, 0),
            ]
        );

        // An existing site beyond the path budget is unavailable.
        let capped = CandidateContext::new(
            origin,
            Needs::default(),
            &sites,
            &map,
            PathConfig::new(usize::MAX, 2),
        );
        let out_of_budget = ActionCandidate::new(ActionKind::Eat, Some(coord(ox + 2, oy)), 0)
            .expect("valid diagnostic fixture");
        assert_eq!(
            inputs_by_factor(&out_of_budget, &capped)[3],
            (FactorId::SiteAvailable, 0)
        );

        // Idle records the always-available baseline even with a zero budget.
        let idle =
            ActionCandidate::new(ActionKind::Idle, None, 0).expect("valid diagnostic fixture");
        assert_eq!(
            inputs_by_factor(&idle, &capped)[3],
            (FactorId::SiteAvailable, 1)
        );
    }

    #[test]
    fn trace_for_leaves_every_scoring_field_unset() {
        let (map, sites, origin) = fixture();
        let context = context(origin, needs_with(50_000, 25_000), &sites, &map);
        for candidate in candidate_actions(&context) {
            let trace = trace_for(&candidate, &context);
            assert_eq!(trace.selected(), None);
            assert_eq!(trace.tie_break(), None);
            assert_eq!(trace.candidates().len(), 1);
            let candidate_trace = &trace.candidates()[0];
            assert_eq!(candidate_trace.candidate(), candidate);
            assert_eq!(candidate_trace.total(), None);
            assert_eq!(candidate_trace.perturbation(), None);
            assert_eq!(candidate_trace.factors().len(), 5);
            for evaluation in candidate_trace.factors() {
                assert_eq!(evaluation.contribution(), None);
            }
        }
    }

    #[test]
    fn trace_is_deterministic_and_byte_identical() {
        let (map, sites, origin) = fixture();
        let context = context(origin, needs_with(50_000, 25_000), &sites, &map);
        let candidate = candidate_actions(&context)[0];
        let first = trace_for(&candidate, &context);
        for _ in 0..8 {
            assert_eq!(trace_for(&candidate, &context), first);
        }
        let first_json = serde_json::to_vec(&first).expect("serialize trace");
        let second_json =
            serde_json::to_vec(&trace_for(&candidate, &context)).expect("serialize trace");
        assert_eq!(
            first_json, second_json,
            "serialization must be byte-identical"
        );
    }

    #[test]
    fn traces_are_integer_only() {
        let (map, sites, origin) = fixture();
        let context = context(origin, needs_with(50_000, 25_000), &sites, &map);
        for candidate in candidate_actions(&context) {
            let trace = trace_for(&candidate, &context);
            let value = serde_json::to_value(&trace).expect("serialize trace");
            assert_no_floats(&value);
        }
    }

    /// Recursively asserts that no JSON number in `value` is a float.
    fn assert_no_floats(value: &serde_json::Value) {
        match value {
            serde_json::Value::Number(number) => {
                assert!(!number.is_f64(), "trace contains a float: {number}");
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
    fn trace_types_serde_round_trip() {
        let input = FactorInput::new(FactorId::DistanceToTarget, 42);
        let encoded = serde_json::to_string(&input).expect("serialize input");
        assert_eq!(encoded, "{\"factor\":\"DistanceToTarget\",\"input\":42}");
        assert_eq!(
            serde_json::from_str::<FactorInput>(&encoded).expect("deserialize input"),
            input
        );

        let evaluation = FactorEvaluation::new(input);
        let encoded = serde_json::to_string(&evaluation).expect("serialize evaluation");
        assert_eq!(
            encoded,
            "{\"input\":{\"factor\":\"DistanceToTarget\",\"input\":42},\"contribution\":null}"
        );
        assert_eq!(
            serde_json::from_str::<FactorEvaluation>(&encoded).expect("deserialize evaluation"),
            evaluation
        );

        let candidate = ActionCandidate::new(ActionKind::Work, Some(coord(1, 2)), 3)
            .expect("valid diagnostic fixture");
        let candidate_trace = CandidateTrace::new(candidate, vec![evaluation]);
        let encoded = serde_json::to_string(&candidate_trace).expect("serialize candidate trace");
        assert_eq!(
            serde_json::from_str::<CandidateTrace>(&encoded).expect("deserialize candidate trace"),
            candidate_trace
        );

        let trace = DecisionTrace::new(vec![candidate_trace]).expect("valid diagnostic fixture");
        let encoded = serde_json::to_string(&trace).expect("serialize decision trace");
        assert!(
            encoded.contains("\"selected\":null"),
            "selection stays unset: {encoded}"
        );
        assert!(encoded.contains("\"tie_break\":null"));
        assert!(
            encoded.contains("\"perturbation\":null"),
            "perturbation stays unset: {encoded}"
        );
        assert_eq!(
            serde_json::from_str::<DecisionTrace>(&encoded).expect("deserialize decision trace"),
            trace
        );

        // The tie-break vocabulary serializes but CHRON-025 never populates it.
        let encoded = serde_json::to_string(&TieBreakReason::UniqueMaximum).expect("serialize");
        assert_eq!(encoded, "\"UniqueMaximum\"");
        let encoded = serde_json::to_string(&TieBreakReason::StableOrder).expect("serialize");
        assert_eq!(encoded, "\"StableOrder\"");
        assert_eq!(
            serde_json::from_str::<TieBreakReason>("\"StableOrder\"").expect("deserialize"),
            TieBreakReason::StableOrder
        );
    }

    #[test]
    fn trace_ordering_is_stable_without_hash_iteration() {
        // The trace schema is built from Vecs and fixed-size arrays only;
        // serializing the same trace twice must give byte-identical output on
        // any platform (no HashMap/BTreeMap iteration anywhere in the type).
        let (map, sites, origin) = fixture();
        let context = context(origin, needs_with(1, 2), &sites, &map);
        let traces: Vec<DecisionTrace> = candidate_actions(&context)
            .iter()
            .map(|candidate| trace_for(candidate, &context))
            .collect();
        let first = serde_json::to_vec(&traces).expect("serialize traces");
        let second = serde_json::to_vec(&traces).expect("serialize traces");
        assert_eq!(first, second);
        // WorkProgress stays bounded by WorkCounter::MAX through the schema.
        let work = sites
            .sites_of(SiteKind::Work)
            .next()
            .expect("fixture has a work site");
        let candidate = ActionCandidate::new(ActionKind::Work, Some(work.coord()), 0)
            .expect("valid diagnostic fixture");
        let progress = factor_inputs_for(&candidate, &context)[4].input();
        assert!((0..=i64::try_from(WorkCounter::MAX).expect("MAX fits i64")).contains(&progress));
    }
}
