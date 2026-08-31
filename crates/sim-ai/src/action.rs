// Authored by Kimi Code (AI coding agent) — task CHRON-025.
//! Action-candidate contract for the Phase 1 Utility AI (CHRON-025, ADR-0014).
//!
//! [`candidate_actions`] enumerates the finite set of actions currently open
//! to one person — `Move`, `Eat`, `Sleep`, `Work`, and the `Idle` baseline —
//! as an ordered, deduplicated, bounded `Vec` of [`ActionCandidate`] values.
//! Enumeration is pure data construction: it reads the person's location,
//! [`Needs`], the static [`ActivitySites`], and terrain reachability via
//! [`find_path`], and it never scores, selects, tie-breaks, or randomizes
//! (those are CHRON-026 scope; Master Spec §14/§72).
//!
//! Determinism is structural: iteration follows the [`ActionKind`] and
//! [`SiteKind`] declaration orders, ties resolve by Manhattan distance and
//! row-major coordinate order, and no float, clock, thread, or hash-order
//! iteration appears anywhere in the module.

use std::collections::BTreeSet;
use std::fmt::{Display, Formatter};

use serde::{Deserialize, Deserializer, Serialize};

use palimpsest_sim_world::{
    ActivitySite, ActivitySites, LocalCoord, PathConfig, SiteKind, TerrainKind, WorldMap, find_path,
};

use crate::needs::Needs;

/// Maximum number of `Move` candidates [`candidate_actions`] emits: one per
/// [`SiteKind`] (`Meal`, `Rest`, `Work`).
pub const MAX_MOVE_CANDIDATES: usize = 3;

/// The exact set of Phase 1 action kinds (CHRON-025; Master Spec §15/§84).
///
/// Declaration order is the canonical enumeration order used by
/// [`candidate_actions`]. Deliberately absent: combat, socialize, protect,
/// and every long-horizon goal. Serde uses the default variant names
/// (`"Move"`, `"Eat"`, `"Sleep"`, `"Work"`, `"Idle"`); those strings are the
/// stable wire keys and must never change.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum ActionKind {
    /// Travel toward a goal-of-interest site.
    Move,
    /// Eat at a `Meal` site.
    Eat,
    /// Sleep at a `Rest` site.
    Sleep,
    /// Work at a `Work` site.
    Work,
    /// The do-nothing baseline; always available, never has a target.
    Idle,
}

/// One enumerated action open to a person: a kind, an optional target
/// coordinate, and the stable enumeration key `order`.
///
/// The provider assigns `order` from the 0-based enumeration position. An
/// individual candidate may carry any key; selection validates the complete
/// set of keys, independently of vector position. It is a per-call key, not persistent
/// identity, truth, or an event reference (CHRON-025, ADR-0014). Serde
/// encodes the three fields as-is; `target` is `null` for `Idle`.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub struct ActionCandidate {
    kind: ActionKind,
    target: Option<LocalCoord>,
    order: u64,
}

impl ActionCandidate {
    /// Creates a candidate with an explicit enumeration key.
    ///
    /// # Errors
    /// Rejects an Idle target or a missing target for any other action.
    pub const fn new(
        kind: ActionKind,
        target: Option<LocalCoord>,
        order: u64,
    ) -> Result<Self, CandidateError> {
        match (kind, target) {
            (ActionKind::Idle, Some(_)) => Err(CandidateError::IdleHasTarget),
            (ActionKind::Idle, None) | (_, Some(_)) => Ok(Self {
                kind,
                target,
                order,
            }),
            (_, None) => Err(CandidateError::MissingTarget { kind }),
        }
    }

    /// The action kind.
    #[must_use]
    pub const fn kind(&self) -> ActionKind {
        self.kind
    }

    /// The target coordinate; `None` exactly for `Idle`.
    #[must_use]
    pub const fn target(&self) -> Option<LocalCoord> {
        self.target
    }

    /// The stable 0-based enumeration key within one emitted list.
    #[must_use]
    pub const fn order(&self) -> u64 {
        self.order
    }
}

impl<'de> Deserialize<'de> for ActionCandidate {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        struct Wire {
            kind: ActionKind,
            target: Option<LocalCoord>,
            order: u64,
        }
        let wire = Wire::deserialize(deserializer)?;
        Self::new(wire.kind, wire.target, wire.order).map_err(serde::de::Error::custom)
    }
}

/// Structurally invalid action shape; reachability is a separate concern.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CandidateError {
    /// Idle must be targetless.
    IdleHasTarget,
    /// Every non-Idle action requires a target.
    MissingTarget { kind: ActionKind },
}

impl Display for CandidateError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::IdleHasTarget => write!(f, "Idle must not have a target"),
            Self::MissingTarget { kind } => write!(f, "{kind:?} requires a target"),
        }
    }
}
impl std::error::Error for CandidateError {}

/// Invalid identity within a collection of candidates (ADR-0019).
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
pub enum CandidateSetError {
    /// More than one candidate uses the same enumeration key.
    DuplicateOrder { order: u64 },
    /// A complete selection key must be below the collection length.
    OrderOutOfRange { order: u64, len: usize },
    /// One action/target pair appears more than once.
    DuplicateCandidate {
        kind: ActionKind,
        target: Option<LocalCoord>,
    },
}

impl Display for CandidateSetError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DuplicateOrder { order } => write!(f, "duplicate candidate order {order}"),
            Self::OrderOutOfRange { order, len } => {
                write!(f, "candidate order {order} outside 0..{len}")
            }
            Self::DuplicateCandidate { kind, target } => {
                write!(f, "duplicate candidate {kind:?} at {target:?}")
            }
        }
    }
}
impl std::error::Error for CandidateSetError {}

/// Three passes give deterministic error precedence without scanning up to
/// an input key. Fragments need unique identities, but not contiguous keys.
pub(crate) fn validate_candidate_set(
    candidates: &[ActionCandidate],
    complete: bool,
) -> Result<(), CandidateSetError> {
    let mut orders = BTreeSet::new();
    for candidate in candidates {
        if !orders.insert(candidate.order()) {
            return Err(CandidateSetError::DuplicateOrder {
                order: candidate.order(),
            });
        }
    }
    if complete {
        for candidate in candidates {
            if usize::try_from(candidate.order()).map_or(true, |order| order >= candidates.len()) {
                return Err(CandidateSetError::OrderOutOfRange {
                    order: candidate.order(),
                    len: candidates.len(),
                });
            }
        }
    }
    let mut pairs = BTreeSet::new();
    for candidate in candidates {
        if !pairs.insert((candidate.kind(), candidate.target())) {
            return Err(CandidateSetError::DuplicateCandidate {
                kind: candidate.kind(),
                target: candidate.target(),
            });
        }
    }
    Ok(())
}

/// The complete read-only input for candidate enumeration and trace
/// construction: one person's location and needs, the static activity sites,
/// the local terrain map, and the pathfinding budget.
///
/// The context borrows the site collection and the map; it owns no world
/// truth state. An optional diagnostic counter observes queries only. Equal contexts yield byte-identical candidate
/// lists and traces (CHRON-025 invariant 1).
#[derive(Clone, Copy, Debug)]
pub struct CandidateContext<'a> {
    location: LocalCoord,
    needs: Needs,
    sites: &'a ActivitySites,
    map: &'a WorldMap,
    path_config: PathConfig,
    path_query_counter: Option<&'a std::cell::Cell<u64>>,
}

impl<'a> CandidateContext<'a> {
    /// Creates a context from the person's location and needs plus borrowed
    /// world state and the pathfinding budget.
    #[must_use]
    pub const fn new(
        location: LocalCoord,
        needs: Needs,
        sites: &'a ActivitySites,
        map: &'a WorldMap,
        path_config: PathConfig,
    ) -> Self {
        Self {
            location,
            needs,
            sites,
            map,
            path_config,
            path_query_counter: None,
        }
    }

    /// Observe actual reachability queries without changing candidate semantics.
    #[must_use]
    pub const fn with_path_query_counter(mut self, counter: &'a std::cell::Cell<u64>) -> Self {
        self.path_query_counter = Some(counter);
        self
    }

    /// The person's current location on the local grid.
    #[must_use]
    pub const fn location(&self) -> LocalCoord {
        self.location
    }

    /// The person's current needs.
    #[must_use]
    pub const fn needs(&self) -> Needs {
        self.needs
    }

    /// The static activity sites in scope.
    #[must_use]
    pub const fn sites(&self) -> &'a ActivitySites {
        self.sites
    }

    /// The local terrain map used for reachability queries.
    #[must_use]
    pub const fn map(&self) -> &'a WorldMap {
        self.map
    }

    /// The pathfinding budget applied to every reachability query.
    #[must_use]
    pub const fn path_config(&self) -> PathConfig {
        self.path_config
    }
}

/// Enumerates the ordered, deduplicated, bounded candidate set for one
/// person (CHRON-025; ADR-0013/0014).
///
/// Enumeration proceeds in [`ActionKind`] declaration order:
///
/// 1. `Move`: for each [`SiteKind`] in `Meal`, `Rest`, `Work` order, the
///    nearest *reachable* site of that kind yields one candidate (at most
///    [`MAX_MOVE_CANDIDATES`]). Distance ties follow
///    [`ActivitySites::find_nearest`] semantics: Manhattan distance first,
///    then row-major coordinate order.
/// 2. `Eat`: only when `needs.hunger().raw() > 0` (any unmet hunger); one
///    candidate per reachable `Meal` site, ordered by Manhattan distance
///    from the location, then row-major coordinate.
/// 3. `Sleep`: only when `needs.fatigue().raw() > 0`; one candidate per
///    reachable `Rest` site, same ordering.
/// 4. `Work`: one candidate per reachable `Work` site, same ordering; there
///    is deliberately no needs gate.
/// 5. `Idle`: exactly one candidate, target `None`, always last.
///
/// Reachability means [`find_path`] over `map.local()` with
/// [`TerrainKind::is_walkable`] under `context.path_config()` returns `Ok`;
/// an unreachable or budget-limited site is silently skipped (its absence is
/// itself recorded for any fabricated candidate via the `SiteAvailable`
/// trace factor in [`crate::trace_for`]).
///
/// The list is deduplicated by `(kind, target)` keeping the first occurrence
/// (the enumeration rules above already make duplicates impossible; the pass
/// enforces the invariant regardless) and `order` is assigned as the final
/// 0-based position. The count never exceeds
/// `MAX_MOVE_CANDIDATES + context.sites().len() + 1`: at most three `Move`
/// candidates, at most one `Eat`/`Sleep`/`Work` candidate per site, and one
/// `Idle`. Identical contexts produce byte-identical lists.
///
/// # Panics
///
/// Never in practice: the candidate count is bounded by
/// `MAX_MOVE_CANDIDATES + sites + 1`, so the enumeration key always fits
/// `u64`.
#[must_use]
pub fn candidate_actions(context: &CandidateContext<'_>) -> Vec<ActionCandidate> {
    let mut emitted: Vec<(ActionKind, Option<LocalCoord>)> =
        Vec::with_capacity(MAX_MOVE_CANDIDATES + context.sites().len() + 1);

    // 1. Move: nearest reachable goal-of-interest per site kind.
    for kind in [SiteKind::Meal, SiteKind::Rest, SiteKind::Work] {
        if let Some(target) = reachable_sites_by_distance(context, kind).first() {
            emitted.push((ActionKind::Move, Some(*target)));
        }
    }

    // 2./3. Eat and Sleep are gated on any unmet corresponding drive.
    if context.needs().hunger().raw() > 0 {
        for target in reachable_sites_by_distance(context, SiteKind::Meal) {
            emitted.push((ActionKind::Eat, Some(target)));
        }
    }
    if context.needs().fatigue().raw() > 0 {
        for target in reachable_sites_by_distance(context, SiteKind::Rest) {
            emitted.push((ActionKind::Sleep, Some(target)));
        }
    }

    // 4. Work has no needs gate (CHRON-025, ADR-0013).
    for target in reachable_sites_by_distance(context, SiteKind::Work) {
        emitted.push((ActionKind::Work, Some(target)));
    }

    // 5. The do-nothing baseline is always present and always last.
    emitted.push((ActionKind::Idle, None));

    // Deduplicate by (kind, target) keeping the first occurrence, then assign
    // the final 0-based enumeration keys.
    let mut unique: Vec<(ActionKind, Option<LocalCoord>)> = Vec::with_capacity(emitted.len());
    for entry in emitted {
        if !unique.contains(&entry) {
            unique.push(entry);
        }
    }
    unique
        .into_iter()
        .enumerate()
        .map(|(position, (kind, target))| ActionCandidate {
            kind,
            target,
            order: u64::try_from(position).expect("candidate count is bounded and fits u64"),
        })
        .collect()
}

/// Reachable sites of `kind`, ordered by Manhattan distance from the context
/// location, then row-major coordinate order.
///
/// `sites_of` iterates row-major and the sort key `(distance, coord)` is
/// total (coordinates are unique), so the result is platform-independent.
fn reachable_sites_by_distance(context: &CandidateContext<'_>, kind: SiteKind) -> Vec<LocalCoord> {
    let mut keyed: Vec<(u32, LocalCoord)> = context
        .sites()
        .sites_of(kind)
        .map(ActivitySite::coord)
        .map(|coord| (manhattan_distance(context.location(), coord), coord))
        .filter(|&(_, coord)| is_reachable(context, coord))
        .collect();
    keyed.sort_unstable();
    keyed.into_iter().map(|(_, coord)| coord).collect()
}

/// Returns whether `find_path` connects the context location to `target`
/// under the context's pathfinding budget.
pub(crate) fn is_reachable(context: &CandidateContext<'_>, target: LocalCoord) -> bool {
    if let Some(counter) = context.path_query_counter {
        counter.set(counter.get().saturating_add(1));
    }
    find_path(
        context.map().local(),
        (context.location().x(), context.location().y()),
        (target.x(), target.y()),
        TerrainKind::is_walkable,
        context.path_config(),
    )
    .is_ok()
}

/// Manhattan distance between two in-bounds coordinates (max 254).
pub(crate) fn manhattan_distance(a: LocalCoord, b: LocalCoord) -> u32 {
    a.x().abs_diff(b.x()) + a.y().abs_diff(b.y())
}

#[cfg(test)]
mod tests {
    use super::{
        ActionCandidate, ActionKind, CandidateContext, MAX_MOVE_CANDIDATES, candidate_actions,
        manhattan_distance,
    };
    use crate::needs::{NeedValue, Needs};
    use palimpsest_sim_world::{
        ActivitySite, ActivitySites, LocalCoord, PathConfig, SiteKind, WorldGenConfig, WorldMap,
        WorldSeed,
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

    /// Fixture: one person at the walkable-block origin with one site of each
    /// kind inside the block (all reachable under the default path config).
    fn fixture() -> (WorldMap, ActivitySites, LocalCoord) {
        let map = default_map();
        let origin = walkable_block_origin(&map);
        let (ox, oy) = (origin.x(), origin.y());
        let sites = ActivitySites::new(vec![
            ActivitySite::new(&map, coord(ox + 2, oy), SiteKind::Meal).expect("walkable"),
            ActivitySite::new(&map, coord(ox, oy + 2), SiteKind::Rest).expect("walkable"),
            ActivitySite::new(&map, coord(ox + 2, oy + 2), SiteKind::Work).expect("walkable"),
        ])
        .expect("distinct coords");
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

    #[test]
    fn query_observation_preserves_candidates_and_counts_failed_queries() {
        let (map, sites, origin) = fixture();
        let plain = context(origin, needs_with(50_000, 50_000), &sites, &map);
        let count = std::cell::Cell::new(0);
        let observed = plain.with_path_query_counter(&count);
        assert_eq!(candidate_actions(&plain), candidate_actions(&observed));
        assert!(count.get() > 0);
        let candidates = candidate_actions(&plain);
        let weights = crate::Weights::default();
        let spec = crate::PerturbationSpec::default();
        assert_eq!(
            crate::select_action(&candidates, &plain, &weights, &spec),
            crate::select_action(&candidates, &observed, &weights, &spec)
        );
        let before = count.get();
        assert!(super::is_reachable(&observed, origin));
        assert_eq!(count.get(), before + 1);
        let blocked = map
            .local()
            .coords()
            .find(|c| !map.local().get(c.x(), c.y()).unwrap().is_walkable())
            .unwrap();
        assert!(!super::is_reachable(&observed, blocked));
        assert_eq!(count.get(), before + 2);
    }

    #[test]
    fn action_kind_is_exactly_the_five_phase_1_kinds() {
        // An exhaustive match without a wildcard fails to compile if a sixth
        // variant is ever added.
        let all = [
            ActionKind::Move,
            ActionKind::Eat,
            ActionKind::Sleep,
            ActionKind::Work,
            ActionKind::Idle,
        ];
        let mut count = 0;
        for kind in all {
            match kind {
                ActionKind::Move
                | ActionKind::Eat
                | ActionKind::Sleep
                | ActionKind::Work
                | ActionKind::Idle => count += 1,
            }
        }
        assert_eq!(count, 5);
        // Declaration order is the canonical enumeration order.
        assert!(ActionKind::Move < ActionKind::Eat);
        assert!(ActionKind::Eat < ActionKind::Sleep);
        assert!(ActionKind::Sleep < ActionKind::Work);
        assert!(ActionKind::Work < ActionKind::Idle);
    }

    #[test]
    fn action_kind_serde_wire_keys_are_stable() {
        let keys = [
            (ActionKind::Move, "\"Move\""),
            (ActionKind::Eat, "\"Eat\""),
            (ActionKind::Sleep, "\"Sleep\""),
            (ActionKind::Work, "\"Work\""),
            (ActionKind::Idle, "\"Idle\""),
        ];
        for (kind, expected) in keys {
            let encoded = serde_json::to_string(&kind).expect("serialize kind");
            assert_eq!(encoded, expected, "wire key changed for {kind:?}");
            assert_eq!(
                serde_json::from_str::<ActionKind>(&encoded).expect("deserialize kind"),
                kind
            );
        }
        assert!(serde_json::from_str::<ActionKind>("\"Combat\"").is_err());
        assert!(serde_json::from_str::<ActionKind>("\"Socialize\"").is_err());
    }

    #[test]
    fn action_candidate_serde_round_trips() {
        let with_target = ActionCandidate::new(ActionKind::Eat, Some(coord(3, 4)), 2)
            .expect("valid diagnostic fixture");
        let encoded = serde_json::to_string(&with_target).expect("serialize candidate");
        assert_eq!(
            encoded,
            "{\"kind\":\"Eat\",\"target\":{\"x\":3,\"y\":4},\"order\":2}"
        );
        assert_eq!(
            serde_json::from_str::<ActionCandidate>(&encoded).expect("deserialize candidate"),
            with_target
        );

        let idle =
            ActionCandidate::new(ActionKind::Idle, None, 9).expect("valid diagnostic fixture");
        let encoded = serde_json::to_string(&idle).expect("serialize idle");
        assert_eq!(encoded, "{\"kind\":\"Idle\",\"target\":null,\"order\":9}");
        assert_eq!(
            serde_json::from_str::<ActionCandidate>(&encoded).expect("deserialize idle"),
            idle
        );
    }

    #[test]
    fn malformed_candidate_wire_is_rejected() {
        for encoded in [
            r#"{"kind":"Idle","target":{"x":3,"y":4},"order":0}"#,
            r#"{"kind":"Work","target":null,"order":0}"#,
        ] {
            assert!(serde_json::from_str::<ActionCandidate>(encoded).is_err());
        }
    }

    #[test]
    fn native_and_wire_candidate_shape_matrix_agree() {
        for kind in [
            ActionKind::Move,
            ActionKind::Eat,
            ActionKind::Sleep,
            ActionKind::Work,
            ActionKind::Idle,
        ] {
            for target in [None, Some(coord(3, 4))] {
                for order in [0, 1, u64::MAX] {
                    let native = ActionCandidate::new(kind, target, order);
                    let wire = serde_json::from_value::<ActionCandidate>(serde_json::json!({
                        "kind": kind, "target": target, "order": order
                    }));
                    let valid = (kind == ActionKind::Idle) == target.is_none();
                    assert_eq!(native.is_ok(), valid);
                    assert_eq!(wire.is_ok(), valid);
                    if valid {
                        assert_eq!(native.expect("valid shape"), wire.expect("valid wire"));
                    } else {
                        let expected = if kind == ActionKind::Idle {
                            super::CandidateError::IdleHasTarget
                        } else {
                            super::CandidateError::MissingTarget { kind }
                        };
                        assert_eq!(native, Err(expected));
                    }
                }
            }
        }
        assert!(
            serde_json::from_str::<ActionCandidate>(
                r#"{"kind":"Eat","target":{"x":128,"y":4},"order":0}"#
            )
            .is_err()
        );
    }

    #[test]
    fn enumeration_is_deterministic_and_byte_identical() {
        let (map, sites, origin) = fixture();
        let context = context(origin, needs_with(50_000, 25_000), &sites, &map);
        let first = candidate_actions(&context);
        assert!(!first.is_empty());
        for _ in 0..8 {
            assert_eq!(candidate_actions(&context), first);
        }
        let first_json = serde_json::to_vec(&first).expect("serialize candidates");
        let second_json =
            serde_json::to_vec(&candidate_actions(&context)).expect("serialize candidates");
        assert_eq!(
            first_json, second_json,
            "serialization must be byte-identical"
        );
        // Enumeration keys are the final 0-based positions.
        for (position, candidate) in first.iter().enumerate() {
            assert_eq!(
                candidate.order(),
                u64::try_from(position).expect("position fits u64")
            );
        }
    }

    #[test]
    fn enumeration_follows_action_kind_declaration_order() {
        let (map, sites, origin) = fixture();
        let context = context(origin, needs_with(50_000, 25_000), &sites, &map);
        let candidates = candidate_actions(&context);
        let kinds: Vec<ActionKind> = candidates.iter().map(ActionCandidate::kind).collect();
        assert_eq!(
            kinds,
            vec![
                ActionKind::Move,
                ActionKind::Move,
                ActionKind::Move,
                ActionKind::Eat,
                ActionKind::Sleep,
                ActionKind::Work,
                ActionKind::Idle,
            ]
        );
        // Move targets follow the Meal, Rest, Work site-kind order.
        assert_eq!(
            candidates[0].target(),
            Some(coord(origin.x() + 2, origin.y()))
        );
        assert_eq!(
            candidates[1].target(),
            Some(coord(origin.x(), origin.y() + 2))
        );
        assert_eq!(
            candidates[2].target(),
            Some(coord(origin.x() + 2, origin.y() + 2))
        );
        // Idle is always present, always last, never targeted.
        let last = candidates.last().expect("idle baseline exists");
        assert_eq!(last.kind(), ActionKind::Idle);
        assert_eq!(last.target(), None);
    }

    #[test]
    fn closed_loop_gates_and_presence_hold() {
        let (map, sites, origin) = fixture();

        // Hungry and tired: every closed-loop action is present with the
        // reachable site target.
        let hungry_tired = context(origin, needs_with(1, 1), &sites, &map);
        let candidates = candidate_actions(&hungry_tired);
        let eat = candidates
            .iter()
            .find(|candidate| candidate.kind() == ActionKind::Eat)
            .expect("hunger above zero enumerates Eat");
        assert_eq!(eat.target(), Some(coord(origin.x() + 2, origin.y())));
        assert!(
            candidates
                .iter()
                .any(|candidate| candidate.kind() == ActionKind::Sleep)
        );
        assert!(
            candidates
                .iter()
                .any(|candidate| candidate.kind() == ActionKind::Work)
        );
        assert_eq!(
            candidates.last().map(ActionCandidate::kind),
            Some(ActionKind::Idle)
        );

        // Satisfied drives suppress Eat and Sleep; Move and Work stay.
        let satisfied = context(origin, Needs::default(), &sites, &map);
        let kinds: Vec<ActionKind> = candidate_actions(&satisfied)
            .iter()
            .map(ActionCandidate::kind)
            .collect();
        assert_eq!(
            kinds,
            vec![
                ActionKind::Move,
                ActionKind::Move,
                ActionKind::Move,
                ActionKind::Work,
                ActionKind::Idle,
            ]
        );

        // Hunger alone yields Eat but no Sleep, and vice versa.
        let hungry_only = context(origin, needs_with(5_000, 0), &sites, &map);
        let kinds: Vec<ActionKind> = candidate_actions(&hungry_only)
            .iter()
            .map(ActionCandidate::kind)
            .collect();
        assert!(kinds.contains(&ActionKind::Eat));
        assert!(!kinds.contains(&ActionKind::Sleep));
        let tired_only = context(origin, needs_with(0, 5_000), &sites, &map);
        let kinds: Vec<ActionKind> = candidate_actions(&tired_only)
            .iter()
            .map(ActionCandidate::kind)
            .collect();
        assert!(!kinds.contains(&ActionKind::Eat));
        assert!(kinds.contains(&ActionKind::Sleep));
    }

    #[test]
    fn absent_site_kind_yields_no_candidates_of_its_actions() {
        let map = default_map();
        let origin = walkable_block_origin(&map);
        let (ox, oy) = (origin.x(), origin.y());
        // No Work site at all: Work must be absent even though it is ungated.
        let sites = ActivitySites::new(vec![
            ActivitySite::new(&map, coord(ox + 1, oy), SiteKind::Meal).expect("walkable"),
            ActivitySite::new(&map, coord(ox, oy + 1), SiteKind::Rest).expect("walkable"),
        ])
        .expect("distinct coords");
        let context = context(origin, needs_with(50_000, 50_000), &sites, &map);
        let candidates = candidate_actions(&context);
        assert!(
            !candidates
                .iter()
                .any(|candidate| candidate.kind() == ActionKind::Work)
        );
        // Only two Move candidates exist (Meal, Rest), then Eat, Sleep, Idle.
        let kinds: Vec<ActionKind> = candidates.iter().map(ActionCandidate::kind).collect();
        assert_eq!(
            kinds,
            vec![
                ActionKind::Move,
                ActionKind::Move,
                ActionKind::Eat,
                ActionKind::Sleep,
                ActionKind::Idle,
            ]
        );
    }

    #[test]
    fn unreachable_sites_are_skipped_and_idle_remains() {
        let (map, sites, origin) = fixture();
        // A one-cell path cap makes every off-cell site unreachable without
        // touching terrain; enumeration must degrade to the Idle baseline.
        let context = CandidateContext::new(
            origin,
            needs_with(50_000, 50_000),
            &sites,
            &map,
            PathConfig::new(usize::MAX, 1),
        );
        let candidates = candidate_actions(&context);
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].kind(), ActionKind::Idle);
        assert_eq!(candidates[0].target(), None);
    }

    #[test]
    fn per_kind_candidates_are_distance_then_coord_ordered() {
        let map = default_map();
        let origin = walkable_block_origin(&map);
        let (ox, oy) = (origin.x(), origin.y());
        // Two Meal sites: distances 2 (east) and 4 (south-east).
        let near = coord(ox + 2, oy);
        let far = coord(ox + 2, oy + 2);
        let sites = ActivitySites::new(vec![
            ActivitySite::new(&map, far, SiteKind::Meal).expect("walkable"),
            ActivitySite::new(&map, near, SiteKind::Meal).expect("walkable"),
        ])
        .expect("distinct coords");
        let context = context(origin, needs_with(1, 0), &sites, &map);
        let eat_targets: Vec<Option<LocalCoord>> = candidate_actions(&context)
            .iter()
            .filter(|candidate| candidate.kind() == ActionKind::Eat)
            .map(ActionCandidate::target)
            .collect();
        assert_eq!(eat_targets, vec![Some(near), Some(far)]);
        // Move picks the nearest reachable Meal site only.
        let move_target = candidate_actions(&context)
            .iter()
            .find(|candidate| candidate.kind() == ActionKind::Move)
            .and_then(ActionCandidate::target);
        assert_eq!(move_target, Some(near));
    }

    #[test]
    fn candidate_set_is_deduplicated_and_bounded() {
        let (map, sites, origin) = fixture();
        let full = context(origin, needs_with(50_000, 50_000), &sites, &map);
        let candidates = candidate_actions(&full);
        // No (kind, target) pair repeats.
        for (index, candidate) in candidates.iter().enumerate() {
            assert!(
                !candidates[..index]
                    .iter()
                    .any(|other| other.kind() == candidate.kind()
                        && other.target() == candidate.target()),
                "duplicate (kind, target) at {index}"
            );
        }
        // The documented bound: MAX_MOVE_CANDIDATES + sites + 1 (Idle).
        let bound = MAX_MOVE_CANDIDATES + sites.len() + 1;
        assert!(candidates.len() <= bound);
        assert_eq!(bound, 3 + 3 + 1);
        // The fixed three-site fixture yields exactly 3 Move + 1 Eat + 1
        // Sleep + 1 Work + 1 Idle.
        assert_eq!(candidates.len(), 7);

        // The empty site collection yields exactly the Idle baseline.
        let empty = ActivitySites::new(Vec::new()).expect("empty is valid");
        let empty_context = context(origin, needs_with(50_000, 50_000), &empty, &map);
        let candidates = candidate_actions(&empty_context);
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].kind(), ActionKind::Idle);
    }

    #[test]
    fn move_picks_nearest_reachable_not_just_nearest() {
        let map = default_map();
        let origin = walkable_block_origin(&map);
        let (ox, oy) = (origin.x(), origin.y());
        // The nearest Meal site sits one step east; a one-cell path cap makes
        // exactly the adjacent cell reachable, the two-step site not.
        let adjacent = coord(ox + 1, oy);
        let two_steps = coord(ox + 2, oy);
        let sites = ActivitySites::new(vec![
            ActivitySite::new(&map, two_steps, SiteKind::Meal).expect("walkable"),
            ActivitySite::new(&map, adjacent, SiteKind::Meal).expect("walkable"),
        ])
        .expect("distinct coords");
        let context = CandidateContext::new(
            origin,
            Needs::default(),
            &sites,
            &map,
            PathConfig::new(usize::MAX, 2),
        );
        let move_target = candidate_actions(&context)
            .iter()
            .find(|candidate| candidate.kind() == ActionKind::Move)
            .and_then(ActionCandidate::target);
        assert_eq!(move_target, Some(adjacent));
    }

    #[test]
    fn manhattan_distance_matches_site_semantics() {
        assert_eq!(manhattan_distance(coord(0, 0), coord(0, 0)), 0);
        assert_eq!(manhattan_distance(coord(1, 2), coord(4, 6)), 7);
        assert_eq!(manhattan_distance(coord(127, 127), coord(0, 0)), 254);
    }
}
