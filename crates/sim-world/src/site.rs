// Authored by Kimi Code (AI coding agent) — task CHRON-023.
//! Static activity sites: the fixed, walkable affordance points (`Meal`,
//! `Rest`, `Work`) a Person paths to (CHRON-024) and acts at (CHRON-027).
//!
//! A site is a plain value record — a [`LocalCoord`] plus a [`SiteKind`],
//! with a bounded observation [`WorkCounter`] on `Work` sites only. Sites are
//! static: they neither move, spawn, consume, nor produce anything, and the
//! collection stores no `EntityId` or runtime ECS handle (CHRON-023,
//! ADR-0013). There is deliberately no inventory, resource, production,
//! storage, market, construction, or settlement simulation here.
//!
//! Determinism: all arithmetic is integer, collection iteration is row-major,
//! and [`ActivitySites::find_nearest`] resolves equal distances by row-major
//! coordinate order. No floats, clocks, threads, or hash-order leakage.

use core::fmt::{self, Display, Formatter};

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::coord::LocalCoord;
use crate::worldgen::WorldMap;

/// The exact set of Phase 1 activity affordances (CHRON-023).
///
/// Deliberately excluded: combat, trade, production, construction, storage,
/// and every other site kind; the closed Eat/Sleep/Work loop needs nothing
/// else (ADR-0013).
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum SiteKind {
    /// Where a Person eats.
    Meal,
    /// Where a Person sleeps.
    Rest,
    /// Where a Person works; the only kind carrying a [`WorkCounter`].
    Work,
}

/// A bounded, saturating observation count of work performed at one site.
///
/// This is a validation metric only (ADR-0013): it is not a game resource,
/// has no reset semantics, and produces nothing. The value is an integer in
/// `[0, WorkCounter::MAX]`; [`WorkCounter::advance_work`] saturates at
/// [`WorkCounter::MAX`] instead of wrapping. `MAX` leaves roughly a 25×
/// margin over the ≈365,000 observations one site could accrue if every one
/// of the 100 Phase 1 validation NPCs worked the same site once a day for ten
/// years. Serde encodes a plain unsigned integer and rejects values above
/// `MAX`.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct WorkCounter(u64);

impl WorkCounter {
    /// The documented upper bound of the observation count.
    pub const MAX: u64 = 10_000_000;
    /// The zero counter every new `Work` site starts with.
    pub const ZERO: Self = Self(0);

    /// Creates a counter holding `value`, or `None` when `value > MAX`.
    #[must_use]
    pub const fn new(value: u64) -> Option<Self> {
        if value <= Self::MAX {
            Some(Self(value))
        } else {
            None
        }
    }

    /// Returns the current observation count.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }

    /// Records one unit of observed work, saturating at [`WorkCounter::MAX`].
    ///
    /// The counter advances only when called (by the CHRON-027 action state
    /// machine); it never auto-advances.
    pub const fn advance_work(&mut self) {
        if self.0 < Self::MAX {
            self.0 += 1;
        }
    }
}

impl<'de> Deserialize<'de> for WorkCounter {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = u64::deserialize(deserializer)?;
        Self::new(value).ok_or_else(|| {
            serde::de::Error::custom(format_args!(
                "work counter {value} exceeds the maximum {}",
                Self::MAX
            ))
        })
    }
}

/// Errors from fallible activity-site operations.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SiteError {
    /// No site exists at the coordinate.
    UnknownSite(LocalCoord),
    /// The site at the coordinate carries no work counter.
    NotAWorkSite(LocalCoord),
    /// The coordinate is not walkable on the given map.
    UnwalkableSite(LocalCoord),
    /// Two sites were given the same coordinate.
    DuplicateSite(LocalCoord),
}

impl Display for SiteError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownSite(coord) => {
                write!(
                    formatter,
                    "no activity site at ({}, {})",
                    coord.x(),
                    coord.y()
                )
            }
            Self::NotAWorkSite(coord) => write!(
                formatter,
                "activity site at ({}, {}) is not a work site",
                coord.x(),
                coord.y()
            ),
            Self::UnwalkableSite(coord) => write!(
                formatter,
                "activity site coordinate ({}, {}) is not walkable",
                coord.x(),
                coord.y()
            ),
            Self::DuplicateSite(coord) => write!(
                formatter,
                "duplicate activity site at ({}, {})",
                coord.x(),
                coord.y()
            ),
        }
    }
}

impl std::error::Error for SiteError {}

/// One static activity site: a walkable coordinate plus its affordance.
///
/// Invariants:
///
/// - `coord` is walkable on the map the site was built against; construction
///   is the only entry point and validates it, so unwalkable sites are not
///   constructible. Deserialization has no map in scope and therefore
///   re-validates only the work invariant below — wire data is expected to
///   originate from a validated collection.
/// - `work` is `Some` for `Work` sites and `None` for `Meal`/`Rest` sites.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub struct ActivitySite {
    coord: LocalCoord,
    kind: SiteKind,
    work: Option<WorkCounter>,
}

impl ActivitySite {
    /// Creates a site at `coord` when the map terrain there is walkable.
    ///
    /// `Work` sites start with a zero [`WorkCounter`]; `Meal`/`Rest` sites
    /// carry none.
    ///
    /// # Errors
    ///
    /// Returns [`SiteError::UnwalkableSite`] when the map cell at `coord` is
    /// not walkable.
    pub fn new(map: &WorldMap, coord: LocalCoord, kind: SiteKind) -> Result<Self, SiteError> {
        if is_walkable_at(map, coord) {
            Ok(Self::from_validated(coord, kind))
        } else {
            Err(SiteError::UnwalkableSite(coord))
        }
    }

    /// Builds a site whose coordinate was already validated as walkable.
    const fn from_validated(coord: LocalCoord, kind: SiteKind) -> Self {
        let work = match kind {
            SiteKind::Work => Some(WorkCounter::ZERO),
            SiteKind::Meal | SiteKind::Rest => None,
        };
        Self { coord, kind, work }
    }

    /// The site's coordinate.
    #[must_use]
    pub const fn coord(&self) -> LocalCoord {
        self.coord
    }

    /// The site's affordance kind.
    #[must_use]
    pub const fn kind(&self) -> SiteKind {
        self.kind
    }

    /// The work observation counter; `Some` exactly for `Work` sites.
    #[must_use]
    pub const fn work(&self) -> Option<WorkCounter> {
        self.work
    }
}

/// Serde wire form, re-validated on deserialization.
#[derive(Deserialize)]
struct ActivitySiteWire {
    coord: LocalCoord,
    kind: SiteKind,
    work: Option<WorkCounter>,
}

impl<'de> Deserialize<'de> for ActivitySite {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let wire = ActivitySiteWire::deserialize(deserializer)?;
        let consistent = match wire.kind {
            SiteKind::Work => wire.work.is_some(),
            SiteKind::Meal | SiteKind::Rest => wire.work.is_none(),
        };
        if consistent {
            Ok(Self {
                coord: wire.coord,
                kind: wire.kind,
                work: wire.work,
            })
        } else {
            Err(serde::de::Error::custom(format_args!(
                "{:?} site is inconsistent with work counter presence",
                wire.kind
            )))
        }
    }
}

/// A deterministic collection of static activity sites.
///
/// Sites are stored sorted by coordinate in row-major order with at most one
/// site per coordinate, so `sites_of`, `site_at`, and `find_nearest` are
/// deterministic. The collection holds plain values only: no `EntityId`, no
/// runtime ECS handles, no map ownership (CHRON-023, ADR-0013). Serde encodes
/// the bare site list and re-validates coordinate uniqueness and the per-site
/// work invariant on deserialization.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActivitySites {
    sites: Vec<ActivitySite>,
}

impl ActivitySites {
    /// Creates a collection from arbitrary sites, sorted row-major.
    ///
    /// # Errors
    ///
    /// Returns [`SiteError::DuplicateSite`] when two sites share a coordinate.
    pub fn new(mut sites: Vec<ActivitySite>) -> Result<Self, SiteError> {
        sites.sort_unstable_by_key(ActivitySite::coord);
        if let Some(duplicate) = sites
            .windows(2)
            .find_map(|pair| (pair[0].coord == pair[1].coord).then_some(pair[0].coord))
        {
            return Err(SiteError::DuplicateSite(duplicate));
        }
        Ok(Self { sites })
    }

    /// Places the deterministic default site set on a generated map.
    ///
    /// Six sites — two of each kind — are spread evenly over the map's
    /// walkable cells in row-major ordinal order (slot `k` of 6 takes the
    /// walkable cell at ordinal `k * (n - 1) / 5` out of `n`), so the default
    /// fixture covers the map instead of clustering. With fewer than six
    /// walkable cells the first cells are used, cycling `Meal`, `Rest`,
    /// `Work`. Every generated [`WorldMap`] guarantees a walkable spawn
    /// clearing (64 cells by default), so each kind is always present on
    /// generated terrain. Equal maps yield equal placements.
    ///
    /// # Panics
    ///
    /// Never in practice: the spread ordinals are strictly increasing over
    /// distinct walkable coordinates, so collection construction cannot fail.
    #[must_use]
    pub fn place_defaults(map: &WorldMap) -> Self {
        /// Affordance of each evenly spread placement slot.
        const KIND_SLOTS: [SiteKind; 6] = [
            SiteKind::Meal,
            SiteKind::Rest,
            SiteKind::Work,
            SiteKind::Meal,
            SiteKind::Rest,
            SiteKind::Work,
        ];
        /// Affordance cycle for the degenerate fewer-than-six-cells case.
        const KIND_CYCLE: [SiteKind; 3] = [SiteKind::Meal, SiteKind::Rest, SiteKind::Work];
        let walkable: Vec<LocalCoord> = map
            .local()
            .coords()
            .filter(|coord| is_walkable_at(map, *coord))
            .collect();
        let count = walkable.len();
        let mut sites = Vec::with_capacity(KIND_SLOTS.len().min(count));
        if count >= KIND_SLOTS.len() {
            let last = count - 1;
            for (slot, kind) in KIND_SLOTS.iter().enumerate() {
                let ordinal = slot * last / (KIND_SLOTS.len() - 1);
                sites.push(ActivitySite::from_validated(walkable[ordinal], *kind));
            }
        } else {
            for (position, coord) in walkable.iter().enumerate() {
                sites.push(ActivitySite::from_validated(
                    *coord,
                    KIND_CYCLE[position % KIND_CYCLE.len()],
                ));
            }
        }
        Self::new(sites).expect("spread ordinals are distinct walkable coordinates")
    }

    /// Returns the number of sites.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.sites.len()
    }

    /// Returns whether the collection holds no sites.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.sites.is_empty()
    }

    /// Iterates the sites of `kind` in row-major coordinate order.
    pub fn sites_of(&self, kind: SiteKind) -> impl Iterator<Item = &ActivitySite> + '_ {
        self.sites.iter().filter(move |site| site.kind == kind)
    }

    /// Returns the site at `coord`, if any.
    #[must_use]
    pub fn site_at(&self, coord: LocalCoord) -> Option<&ActivitySite> {
        self.sites
            .binary_search_by_key(&coord, ActivitySite::coord)
            .ok()
            .map(|index| &self.sites[index])
    }

    /// Returns the coordinate of the nearest site of `kind`, if any.
    ///
    /// Distance is Manhattan (`|Δx| + |Δy|`, integer arithmetic). Ties
    /// resolve to the row-major smallest coordinate: candidates are scanned
    /// in row-major order and a later candidate replaces the incumbent only
    /// when strictly closer. Runs in O(number of sites of `kind`).
    #[must_use]
    pub fn find_nearest(&self, from: LocalCoord, kind: SiteKind) -> Option<LocalCoord> {
        let mut best: Option<(u32, LocalCoord)> = None;
        for site in self.sites_of(kind) {
            let distance = manhattan_distance(from, site.coord);
            if best.is_none_or(|(best_distance, _)| distance < best_distance) {
                best = Some((distance, site.coord));
            }
        }
        best.map(|(_, coord)| coord)
    }

    /// Records one unit of observed work at the `Work` site on `coord` and
    /// returns the new count.
    ///
    /// The counter advances only when called (by the CHRON-027 action state
    /// machine) and saturates at [`WorkCounter::MAX`].
    ///
    /// # Errors
    ///
    /// Returns [`SiteError::UnknownSite`] when no site exists at `coord`, or
    /// [`SiteError::NotAWorkSite`] when the site there is not a `Work` site.
    pub fn record_work(&mut self, coord: LocalCoord) -> Result<u64, SiteError> {
        let index = self
            .sites
            .binary_search_by_key(&coord, ActivitySite::coord)
            .map_err(|_| SiteError::UnknownSite(coord))?;
        let counter = self.sites[index]
            .work
            .as_mut()
            .ok_or(SiteError::NotAWorkSite(coord))?;
        counter.advance_work();
        Ok(counter.get())
    }
}

impl Serialize for ActivitySites {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.sites.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for ActivitySites {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let sites = Vec::<ActivitySite>::deserialize(deserializer)?;
        Self::new(sites).map_err(serde::de::Error::custom)
    }
}

/// Returns whether the map cell at `coord` is walkable terrain.
fn is_walkable_at(map: &WorldMap, coord: LocalCoord) -> bool {
    map.local()
        .get(coord.x(), coord.y())
        .is_some_and(|kind| kind.is_walkable())
}

/// Manhattan distance between two in-bounds coordinates (max 254).
fn manhattan_distance(a: LocalCoord, b: LocalCoord) -> u32 {
    a.x().abs_diff(b.x()) + a.y().abs_diff(b.y())
}

#[cfg(test)]
mod tests {
    use super::{ActivitySite, ActivitySites, SiteError, SiteKind, WorkCounter, is_walkable_at};
    use crate::coord::LocalCoord;
    use crate::worldgen::{WorldGenConfig, WorldMap, WorldSeed};

    /// Locked fixture seed; any seed works because the generator guarantees a
    /// walkable spawn clearing and at least one impassable cell.
    const FIXTURE_SEED: u64 = 23_023;

    fn default_map() -> WorldMap {
        WorldMap::generate(WorldSeed::new(FIXTURE_SEED), WorldGenConfig::default())
    }

    fn coord(x: i32, y: i32) -> LocalCoord {
        LocalCoord::new(x, y).expect("test coordinate in bounds")
    }

    fn walkable_coord(map: &WorldMap) -> LocalCoord {
        map.local()
            .coords()
            .find(|coord| is_walkable_at(map, *coord))
            .expect("generated map has walkable cells")
    }

    fn unwalkable_coord(map: &WorldMap) -> LocalCoord {
        map.local()
            .coords()
            .find(|coord| !is_walkable_at(map, *coord))
            .expect("generated map has an impassable cell")
    }

    /// Origin of a fully walkable 3×3 block, guaranteed by the 8×8 spawn
    /// clearing of the default generator config.
    fn walkable_block_origin(map: &WorldMap) -> LocalCoord {
        map.local()
            .coords()
            .find(|origin| {
                (0..3).all(|dy| {
                    (0..3).all(|dx| {
                        LocalCoord::new(origin.x() + dx, origin.y() + dy)
                            .is_some_and(|coord| is_walkable_at(map, coord))
                    })
                })
            })
            .expect("spawn clearing contains a 3x3 walkable block")
    }

    #[test]
    fn work_counter_advances_exactly_and_saturates_at_max() {
        let mut counter = WorkCounter::ZERO;
        assert_eq!(counter.get(), 0);
        for expected in 1..=3_u64 {
            counter.advance_work();
            assert_eq!(counter.get(), expected);
        }
        let mut full = WorkCounter::new(WorkCounter::MAX).expect("MAX is valid");
        for _ in 0..3 {
            full.advance_work();
        }
        assert_eq!(full.get(), WorkCounter::MAX);
        assert_eq!(WorkCounter::new(0), Some(WorkCounter::ZERO));
        assert!(WorkCounter::new(WorkCounter::MAX + 1).is_none());
    }

    #[test]
    fn work_counter_serde_round_trips_and_rejects_out_of_range() {
        let counter = WorkCounter::new(42).expect("in range");
        let encoded = serde_json::to_string(&counter).expect("serialize counter");
        assert_eq!(encoded, "42");
        assert_eq!(
            serde_json::from_str::<WorkCounter>(&encoded).expect("deserialize counter"),
            counter
        );
        assert!(serde_json::from_str::<WorkCounter>(&WorkCounter::MAX.to_string()).is_ok());
        assert!(serde_json::from_str::<WorkCounter>(&(WorkCounter::MAX + 1).to_string()).is_err());
    }

    #[test]
    fn site_kind_serde_round_trips_and_rejects_unknown() {
        for kind in [SiteKind::Meal, SiteKind::Rest, SiteKind::Work] {
            let encoded = serde_json::to_string(&kind).expect("serialize kind");
            assert_eq!(
                serde_json::from_str::<SiteKind>(&encoded).expect("deserialize kind"),
                kind
            );
        }
        assert!(serde_json::from_str::<SiteKind>("\"Trade\"").is_err());
        assert!(serde_json::from_str::<SiteKind>("\"Storage\"").is_err());
    }

    #[test]
    fn construction_enforces_walkability_and_the_work_invariant() {
        let map = default_map();
        let walkable = walkable_coord(&map);
        let work_site = ActivitySite::new(&map, walkable, SiteKind::Work).expect("walkable");
        assert_eq!(work_site.work(), Some(WorkCounter::ZERO));
        for kind in [SiteKind::Meal, SiteKind::Rest] {
            let site = ActivitySite::new(&map, walkable, kind).expect("walkable");
            assert_eq!(site.coord(), walkable);
            assert_eq!(site.kind(), kind);
            assert_eq!(site.work(), None);
        }
        let blocked = unwalkable_coord(&map);
        for kind in [SiteKind::Meal, SiteKind::Rest, SiteKind::Work] {
            assert_eq!(
                ActivitySite::new(&map, blocked, kind),
                Err(SiteError::UnwalkableSite(blocked))
            );
        }
    }

    #[test]
    fn site_serde_round_trips_and_rejects_kind_counter_mismatch() {
        let map = default_map();
        let walkable = walkable_coord(&map);
        for kind in [SiteKind::Meal, SiteKind::Rest, SiteKind::Work] {
            let site = ActivitySite::new(&map, walkable, kind).expect("walkable");
            let encoded = serde_json::to_string(&site).expect("serialize site");
            assert_eq!(
                serde_json::from_str::<ActivitySite>(&encoded).expect("deserialize site"),
                site
            );
        }
        // A `Work` site without a counter is rejected, as are `Meal`/`Rest`
        // sites carrying one (the constructor makes both unrepresentable).
        let work_without_counter = "{\"coord\":{\"x\":1,\"y\":1},\"kind\":\"Work\",\"work\":null}";
        assert!(serde_json::from_str::<ActivitySite>(work_without_counter).is_err());
        let meal_with_counter = "{\"coord\":{\"x\":1,\"y\":1},\"kind\":\"Meal\",\"work\":5}";
        assert!(serde_json::from_str::<ActivitySite>(meal_with_counter).is_err());
        let rest_with_counter = "{\"coord\":{\"x\":1,\"y\":1},\"kind\":\"Rest\",\"work\":0}";
        assert!(serde_json::from_str::<ActivitySite>(rest_with_counter).is_err());
        let valid_work = "{\"coord\":{\"x\":1,\"y\":1},\"kind\":\"Work\",\"work\":7}";
        let site = serde_json::from_str::<ActivitySite>(valid_work).expect("valid work site");
        assert_eq!(site.work(), WorkCounter::new(7));
    }

    #[test]
    fn collection_rejects_duplicate_coordinates() {
        let map = default_map();
        let origin = walkable_block_origin(&map);
        let first = ActivitySite::new(&map, origin, SiteKind::Meal).expect("walkable");
        let second = ActivitySite::new(&map, origin, SiteKind::Work).expect("walkable");
        assert_eq!(
            ActivitySites::new(vec![first, second]),
            Err(SiteError::DuplicateSite(origin))
        );
        let wire = "[{\"coord\":{\"x\":1,\"y\":1},\"kind\":\"Meal\",\"work\":null},\
                    {\"coord\":{\"x\":1,\"y\":1},\"kind\":\"Rest\",\"work\":null}]";
        assert!(serde_json::from_str::<ActivitySites>(wire).is_err());
    }

    #[test]
    fn sites_of_filters_and_iterates_row_major() {
        let map = default_map();
        let origin = walkable_block_origin(&map);
        let (ox, oy) = (origin.x(), origin.y());
        let north_west = ActivitySite::new(&map, coord(ox, oy), SiteKind::Meal).expect("walkable");
        let north_east =
            ActivitySite::new(&map, coord(ox + 2, oy), SiteKind::Meal).expect("walkable");
        let south_west =
            ActivitySite::new(&map, coord(ox, oy + 2), SiteKind::Meal).expect("walkable");
        let center =
            ActivitySite::new(&map, coord(ox + 1, oy + 1), SiteKind::Work).expect("walkable");
        // Deliberately unordered input; the collection sorts row-major.
        let sites = ActivitySites::new(vec![south_west, center, north_east, north_west])
            .expect("distinct coords");
        assert_eq!(sites.len(), 4);
        assert!(!sites.is_empty());
        let meal_coords: Vec<LocalCoord> = sites
            .sites_of(SiteKind::Meal)
            .map(ActivitySite::coord)
            .collect();
        assert_eq!(
            meal_coords,
            vec![coord(ox, oy), coord(ox + 2, oy), coord(ox, oy + 2)]
        );
        assert_eq!(sites.sites_of(SiteKind::Work).count(), 1);
        assert_eq!(sites.sites_of(SiteKind::Rest).count(), 0);
        assert_eq!(sites.site_at(coord(ox + 1, oy + 1)), Some(&center));
        assert_eq!(sites.site_at(coord(ox + 1, oy)), None);
    }

    #[test]
    fn find_nearest_prefers_closer_and_breaks_ties_row_major() {
        let map = default_map();
        let origin = walkable_block_origin(&map);
        let (ox, oy) = (origin.x(), origin.y());
        // All four corners are Manhattan distance 2 from the center; the
        // row-major smallest (north-west) must win.
        let sites = ActivitySites::new(vec![
            ActivitySite::new(&map, coord(ox + 2, oy + 2), SiteKind::Meal).expect("walkable"),
            ActivitySite::new(&map, coord(ox, oy + 2), SiteKind::Meal).expect("walkable"),
            ActivitySite::new(&map, coord(ox + 2, oy), SiteKind::Meal).expect("walkable"),
            ActivitySite::new(&map, coord(ox, oy), SiteKind::Meal).expect("walkable"),
        ])
        .expect("distinct coords");
        let center = coord(ox + 1, oy + 1);
        assert_eq!(
            sites.find_nearest(center, SiteKind::Meal),
            Some(coord(ox, oy))
        );
        // Strictly closer wins; the equal-distance east pair breaks row-major.
        assert_eq!(
            sites.find_nearest(coord(ox + 2, oy + 1), SiteKind::Meal),
            Some(coord(ox + 2, oy))
        );
        // A query exactly on a site has distance zero.
        assert_eq!(
            sites.find_nearest(coord(ox, oy + 2), SiteKind::Meal),
            Some(coord(ox, oy + 2))
        );
        // Absent kinds yield `None`, as does the empty collection.
        assert_eq!(sites.find_nearest(center, SiteKind::Work), None);
        let empty = ActivitySites::new(Vec::new()).expect("empty is valid");
        assert!(empty.is_empty());
        assert_eq!(empty.find_nearest(center, SiteKind::Meal), None);
    }

    #[test]
    fn find_nearest_is_repeatably_deterministic() {
        let map = default_map();
        let sites = ActivitySites::place_defaults(&map);
        let query = walkable_coord(&map);
        let first = sites.find_nearest(query, SiteKind::Work);
        assert!(first.is_some());
        for _ in 0..10 {
            assert_eq!(sites.find_nearest(query, SiteKind::Work), first);
        }
    }

    #[test]
    fn record_work_is_checked_and_updates_by_coordinate() {
        let map = default_map();
        let origin = walkable_block_origin(&map);
        let (ox, oy) = (origin.x(), origin.y());
        let work_coord = coord(ox + 1, oy + 1);
        let meal_coord = coord(ox, oy);
        let free_coord = coord(ox + 1, oy);
        let mut sites = ActivitySites::new(vec![
            ActivitySite::new(&map, meal_coord, SiteKind::Meal).expect("walkable"),
            ActivitySite::new(&map, work_coord, SiteKind::Work).expect("walkable"),
        ])
        .expect("distinct coords");
        assert_eq!(sites.record_work(work_coord), Ok(1));
        assert_eq!(sites.record_work(work_coord), Ok(2));
        assert_eq!(
            sites.record_work(meal_coord),
            Err(SiteError::NotAWorkSite(meal_coord))
        );
        assert_eq!(
            sites.record_work(free_coord),
            Err(SiteError::UnknownSite(free_coord))
        );
        let work_site = sites.site_at(work_coord).expect("site exists");
        assert_eq!(work_site.work().map(WorkCounter::get), Some(2));
    }

    #[test]
    fn record_work_saturates_at_the_documented_max() {
        // The wire form is the only way to obtain a near-max counter without
        // advancing it in a loop; deserialization re-validates the invariants.
        let wire = format!(
            "[{{\"coord\":{{\"x\":1,\"y\":1}},\"kind\":\"Work\",\"work\":{}}}]",
            WorkCounter::MAX
        );
        let mut sites = serde_json::from_str::<ActivitySites>(&wire).expect("valid wire");
        let target = coord(1, 1);
        assert_eq!(sites.record_work(target), Ok(WorkCounter::MAX));
        assert_eq!(sites.record_work(target), Ok(WorkCounter::MAX));
    }

    #[test]
    fn place_defaults_covers_each_kind_on_walkable_ground() {
        let map = default_map();
        let sites = ActivitySites::place_defaults(&map);
        assert_eq!(sites.len(), 6);
        for kind in [SiteKind::Meal, SiteKind::Rest, SiteKind::Work] {
            assert!(
                sites.sites_of(kind).count() >= 1,
                "default placement must include {kind:?}"
            );
            for site in sites.sites_of(kind) {
                assert!(is_walkable_at(&map, site.coord()));
            }
        }
    }

    #[test]
    fn place_defaults_is_deterministic() {
        let map = default_map();
        let first = ActivitySites::place_defaults(&map);
        let second = ActivitySites::place_defaults(&map);
        assert_eq!(first, second);
        let twin_map = WorldMap::generate(WorldSeed::new(FIXTURE_SEED), WorldGenConfig::default());
        assert_eq!(first, ActivitySites::place_defaults(&twin_map));
    }

    #[test]
    fn collection_serde_round_trips_with_work_counts() {
        let map = default_map();
        let mut sites = ActivitySites::place_defaults(&map);
        let work_coord = sites
            .sites_of(SiteKind::Work)
            .next()
            .expect("default placement has a work site")
            .coord();
        for expected in 1..=5_u64 {
            assert_eq!(sites.record_work(work_coord), Ok(expected));
        }
        let encoded = serde_json::to_string(&sites).expect("serialize sites");
        let restored: ActivitySites = serde_json::from_str(&encoded).expect("deserialize sites");
        assert_eq!(restored, sites);
        assert_eq!(
            restored
                .site_at(work_coord)
                .and_then(ActivitySite::work)
                .map(WorkCounter::get),
            Some(5)
        );
    }
}
