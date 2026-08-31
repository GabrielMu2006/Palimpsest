// Authored by Kimi Code (AI coding agent) — task CHRON-024.
//! Deterministic A* pathfinding over the single 128×128 local grid.
//!
//! Movement is 4-directional with a uniform integer step cost of 1;
//! diagonal movement is deliberately absent, keeping the map-square-cell
//! contract (Master Spec §29). The heuristic is the admissible, consistent
//! Manhattan distance, so a returned path is always a shortest walkable
//! path under the query predicate.
//!
//! Determinism (ADR-0002/0003/0004, Master Spec §63/§70/§76) is structural:
//!
//! - all keys and costs are integers — no floats, no threads, no
//!   wall-clock, and no randomized hashing anywhere in the search;
//! - the open set is a binary heap ordered by the total key
//!   `(f, h, coord)`: lowest `f` first, then lowest `h` (preferring the
//!   cell closest to the goal), then [`LocalCoord`] row-major order
//!   (`y`, then `x`). No two distinct entries compare equal, so pop order
//!   never depends on push order or platform;
//! - neighbours are enqueued in the fixed order east, south, west, north;
//!   the order cannot affect the result because the heap key is total;
//! - each cell is closed at most once, so equal-cost parent ties resolve
//!   to the parent expanded first under the same total order.
//!
//! Every query is bounded: [`PathConfig::max_nodes`] caps node expansions,
//! [`PathConfig::max_path_len`] caps the returned path length in cells,
//! and even unlimited budgets terminate because each of the 16,384 cells
//! is expanded at most once. No input panics: out-of-bounds or
//! non-walkable endpoints, grids without walkable cells, walled-off goals,
//! and zero budgets all produce documented terminal [`PathError`] outcomes.
//! Cross-region pathfinding, dynamic avoidance, weighted terrain costs,
//! and path smoothing are explicitly out of scope (CHRON-024).

use std::collections::BinaryHeap;

use core::cmp::{Ordering, Reverse};
use core::fmt::{self, Display, Formatter};

use crate::coord::LocalCoord;
use crate::grid::{LOCAL_GRID_CELL_COUNT, LocalGrid};
use crate::terrain::TerrainKind;

/// `came_from` sentinel: no parent recorded for this cell.
const NO_PARENT: u32 = u32::MAX;

/// Configuration bounds for one pathfinding query.
///
/// Both budgets are hard caps that no map contents can exceed. The
/// [`Default`] is *complete* on any 128×128 grid — the expansion cap
/// equals the cell count (a search closes each cell at most once) and the
/// path cap admits every simple path — while still proving that no query
/// can perform unbounded work.
///
/// Degenerate budgets are legal and documented: a `max_nodes` of zero
/// forbids every non-trivial search (the trivial start-equals-goal path is
/// still returned), and a `max_path_len` of zero admits no path at all, so
/// every query yields [`PathError::Unreachable`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PathConfig {
    max_nodes: usize,
    max_path_len: usize,
}

impl PathConfig {
    /// Creates a config with the given expansion and path-length budgets.
    #[must_use]
    pub const fn new(max_nodes: usize, max_path_len: usize) -> Self {
        Self {
            max_nodes,
            max_path_len,
        }
    }

    /// Maximum number of node expansions (non-goal pops) per query.
    #[must_use]
    pub const fn max_nodes(self) -> usize {
        self.max_nodes
    }

    /// Maximum returned path length in cells, including start and goal.
    #[must_use]
    pub const fn max_path_len(self) -> usize {
        self.max_path_len
    }
}

impl Default for PathConfig {
    fn default() -> Self {
        Self {
            max_nodes: LOCAL_GRID_CELL_COUNT,
            max_path_len: LOCAL_GRID_CELL_COUNT,
        }
    }
}

/// A bounded, ordered grid path produced by [`find_path`].
///
/// `coords` runs from the query start to the goal inclusive; consecutive
/// cells are 4-directionally adjacent, in bounds, and walkable under the
/// query predicate, and no cell repeats. `cost` is the integer number of
/// steps (uniform cost 1 per step), so `cost == len() - 1`. The length
/// never exceeds the query's `max_path_len`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Path {
    coords: Vec<LocalCoord>,
    cost: u32,
}

impl Path {
    /// Number of cells in the path, including start and goal; always ≥ 1
    /// for a returned path.
    #[must_use]
    pub fn len(&self) -> usize {
        self.coords.len()
    }

    /// Whether the path has no cells; always `false` for a returned path.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.coords.is_empty()
    }

    /// The ordered cells from start to goal, both inclusive.
    #[must_use]
    pub fn coords(&self) -> &[LocalCoord] {
        &self.coords
    }

    /// Total integer cost: the number of steps, `len() - 1`.
    #[must_use]
    pub const fn cost(&self) -> u32 {
        self.cost
    }
}

/// Terminal, non-panicking outcomes of a failed [`find_path`] query.
///
/// Endpoint validation runs start first: when both endpoints are invalid,
/// the start's error is returned.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PathError {
    /// The start or goal coordinate lay outside the 128×128 grid.
    OutOfBounds {
        /// The rejected raw `(x, y)` coordinates.
        coord: (i32, i32),
    },
    /// The start or goal cell was in bounds but rejected by the
    /// walkability predicate.
    NonWalkable {
        /// The rejected raw `(x, y)` coordinates.
        coord: (i32, i32),
    },
    /// Both endpoints were valid and walkable, but no path within the
    /// `max_path_len` cap connects them: a goal sealed off by impassable
    /// cells, disconnected walkable regions, or a zero path cap. The
    /// search terminated because the open set ran empty.
    Unreachable,
    /// The expansion budget was fully consumed before the goal was
    /// reached.
    LimitExceeded {
        /// Expansions performed when the search stopped; equals `budget`.
        nodes: usize,
        /// The configured `max_nodes` budget that was consumed.
        budget: usize,
    },
}

impl Display for PathError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::OutOfBounds { coord } => {
                write!(
                    formatter,
                    "path endpoint out of bounds: ({}, {})",
                    coord.0, coord.1
                )
            }
            Self::NonWalkable { coord } => {
                write!(
                    formatter,
                    "path endpoint on a non-walkable cell: ({}, {})",
                    coord.0, coord.1
                )
            }
            Self::Unreachable => write!(formatter, "goal is unreachable"),
            Self::LimitExceeded { nodes, budget } => {
                write!(
                    formatter,
                    "pathfinding expansion budget exhausted: {nodes}/{budget} nodes"
                )
            }
        }
    }
}

impl std::error::Error for PathError {}

/// Finds a deterministic shortest path from `start` to `goal` over `grid`.
///
/// `start` and `goal` are raw `(x, y)` pairs so that out-of-bounds input
/// is a documented terminal outcome rather than a caller-side construction
/// failure; the returned [`Path`] carries validated [`LocalCoord`] cells.
/// The walkability `predicate` is queried per candidate cell and must be
/// pure and deterministic; pass [`TerrainKind::is_walkable`] for standard
/// terrain movement.
///
/// Equal `(grid, start, goal, predicate, config)` inputs always return the
/// identical path and cost on every platform; the tie-break rule is
/// documented at module level. The trivial start-equals-goal query returns
/// a one-cell zero-cost path without consuming any expansion budget.
///
/// # Errors
///
/// - [`PathError::OutOfBounds`] — `start` or `goal` lies outside the
///   128×128 grid (the start is reported first when both are invalid).
/// - [`PathError::NonWalkable`] — `start` or `goal` is in bounds but the
///   predicate rejects its cell (the start is reported first).
/// - [`PathError::Unreachable`] — both endpoints are valid and walkable,
///   but no path within the `max_path_len` cap connects them; also
///   returned for a `max_path_len` of zero.
/// - [`PathError::LimitExceeded`] — the `max_nodes` expansion budget was
///   fully consumed before the goal was reached; `nodes` equals `budget`.
pub fn find_path(
    grid: &LocalGrid<TerrainKind>,
    start: (i32, i32),
    goal: (i32, i32),
    walkable: impl Fn(TerrainKind) -> bool,
    config: PathConfig,
) -> Result<Path, PathError> {
    let start = validate_endpoint(grid, start, &walkable)?;
    let goal = validate_endpoint(grid, goal, &walkable)?;
    if config.max_path_len() == 0 {
        return Err(PathError::Unreachable);
    }
    if start == goal {
        return Ok(Path {
            coords: vec![start],
            cost: 0,
        });
    }
    search(grid, start, goal, &walkable, config)
}

/// Validates one raw endpoint: in bounds and accepted by the predicate.
fn validate_endpoint(
    grid: &LocalGrid<TerrainKind>,
    raw: (i32, i32),
    walkable: &impl Fn(TerrainKind) -> bool,
) -> Result<LocalCoord, PathError> {
    let coord = LocalCoord::new(raw.0, raw.1).ok_or(PathError::OutOfBounds { coord: raw })?;
    let cell = grid
        .get_index(coord.index())
        .expect("a validated LocalCoord always indexes in range");
    if !walkable(*cell) {
        return Err(PathError::NonWalkable { coord: raw });
    }
    Ok(coord)
}

/// The A* search proper; both endpoints are validated and distinct.
fn search(
    grid: &LocalGrid<TerrainKind>,
    start: LocalCoord,
    goal: LocalCoord,
    walkable: &impl Fn(TerrainKind) -> bool,
    config: PathConfig,
) -> Result<Path, PathError> {
    // Steps beyond this cap would produce a path longer than max_path_len.
    let step_cap = u32::try_from(config.max_path_len() - 1).unwrap_or(u32::MAX);
    let mut g_score = vec![u32::MAX; LOCAL_GRID_CELL_COUNT];
    let mut came_from = vec![NO_PARENT; LOCAL_GRID_CELL_COUNT];
    let mut closed = vec![false; LOCAL_GRID_CELL_COUNT];
    let mut open = BinaryHeap::new();
    g_score[start.index()] = 0;
    open.push(Reverse(QueueEntry::new(0, manhattan(start, goal), start)));
    let mut expanded = 0_usize;
    while let Some(Reverse(entry)) = open.pop() {
        let cell = entry.coord;
        let index = cell.index();
        if closed[index] {
            // Stale duplicate enqueued before the cell was closed.
            continue;
        }
        if cell == goal {
            return Ok(reconstruct(&came_from, start, goal));
        }
        if expanded >= config.max_nodes() {
            return Err(PathError::LimitExceeded {
                nodes: expanded,
                budget: config.max_nodes(),
            });
        }
        expanded += 1;
        closed[index] = true;
        let next_g = g_score[index] + 1;
        for next in neighbours(cell) {
            let next_index = next.index();
            if closed[next_index] {
                continue;
            }
            let terrain = grid
                .get_index(next_index)
                .expect("an in-bounds neighbour always indexes in range");
            if !walkable(*terrain) {
                continue;
            }
            if next_g > step_cap {
                continue;
            }
            if next_g < g_score[next_index] {
                g_score[next_index] = next_g;
                came_from[next_index] = u32::try_from(index).expect("a cell index always fits u32");
                open.push(Reverse(QueueEntry::new(
                    next_g,
                    manhattan(next, goal),
                    next,
                )));
            }
        }
    }
    Err(PathError::Unreachable)
}

/// Rebuilds the start-to-goal path by walking recorded parents backwards.
/// Parent `g` values strictly decrease along the chain, so the walk always
/// terminates at `start`.
fn reconstruct(came_from: &[u32], start: LocalCoord, goal: LocalCoord) -> Path {
    let mut coords = vec![goal];
    let mut cursor = goal;
    while cursor != start {
        let parent = came_from[cursor.index()];
        cursor = LocalCoord::from_index(usize::try_from(parent).expect("cell index fits usize"))
            .expect("a recorded parent index is always a valid cell");
        coords.push(cursor);
    }
    coords.reverse();
    let cost = u32::try_from(coords.len() - 1).expect("path length fits u32");
    Path { coords, cost }
}

/// In-bounds 4-directional neighbours in the fixed enqueue order east,
/// south, west, north.
fn neighbours(coord: LocalCoord) -> impl Iterator<Item = LocalCoord> {
    const DIRECTIONS: [(i32, i32); 4] = [(1, 0), (0, 1), (-1, 0), (0, -1)];
    DIRECTIONS
        .into_iter()
        .filter_map(move |(dx, dy)| LocalCoord::new(coord.x() + dx, coord.y() + dy))
}

/// The admissible, consistent Manhattan-distance heuristic (unit steps).
fn manhattan(a: LocalCoord, b: LocalCoord) -> u32 {
    a.x().abs_diff(b.x()) + a.y().abs_diff(b.y())
}

/// Open-set entry ordered by the total deterministic key `(f, h, coord)`:
/// lowest `f = g + h` first, then lowest `h`, then [`LocalCoord`] row-major
/// order. The key is total — equal keys would require the same cell at the
/// same `g`, which the strict-improvement push rule makes impossible — so
/// pop order never depends on push order, heap internals, or platform.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct QueueEntry {
    f: u32,
    h: u32,
    coord: LocalCoord,
}

impl QueueEntry {
    fn new(g: u32, h: u32, coord: LocalCoord) -> Self {
        Self { f: g + h, h, coord }
    }
}

impl Ord for QueueEntry {
    fn cmp(&self, other: &Self) -> Ordering {
        (self.f, self.h, self.coord).cmp(&(other.f, other.h, other.coord))
    }
}

impl PartialOrd for QueueEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::{Path, PathConfig, PathError, find_path, manhattan};
    use crate::coord::LocalCoord;
    use crate::grid::{LOCAL_GRID_CELL_COUNT, LocalGrid};
    use crate::terrain::TerrainKind;
    use crate::worldgen::{WorldGenConfig, WorldMap, WorldSeed};

    /// A fully walkable grid.
    fn open_grid() -> LocalGrid<TerrainKind> {
        LocalGrid::filled_with(TerrainKind::Ground)
    }

    /// Asserts every path invariant from the task contract: endpoints,
    /// 4-adjacency, walkability, no repeated cells, the `max_path_len`
    /// bound, and `cost == len - 1`.
    fn assert_valid_path(
        grid: &LocalGrid<TerrainKind>,
        start: (i32, i32),
        goal: (i32, i32),
        path: &Path,
        max_path_len: usize,
    ) {
        let coords = path.coords();
        assert!(
            !coords.is_empty(),
            "a returned path holds at least one cell"
        );
        assert!(path.len() <= max_path_len, "path exceeds the length cap");
        assert_eq!(
            path.cost(),
            u32::try_from(path.len() - 1).expect("length fits u32"),
            "uniform step cost must equal steps"
        );
        assert_eq!(coords.first().map(|c| (c.x(), c.y())), Some(start));
        assert_eq!(coords.last().map(|c| (c.x(), c.y())), Some(goal));
        let mut seen = HashSet::new();
        for cell in coords {
            assert!(
                grid.get_index(cell.index())
                    .expect("path cells are in bounds")
                    .is_walkable(),
                "path cell must be walkable"
            );
            assert!(seen.insert(*cell), "path cells must not repeat");
        }
        for pair in coords.windows(2) {
            assert_eq!(
                manhattan(pair[0], pair[1]),
                1,
                "consecutive cells must be 4-adjacent"
            );
        }
    }

    #[test]
    fn found_path_on_open_grid_is_valid_and_optimal() {
        let grid = open_grid();
        let config = PathConfig::default();
        let path = find_path(&grid, (2, 3), (100, 90), TerrainKind::is_walkable, config)
            .expect("open grid endpoints are reachable");
        assert_valid_path(&grid, (2, 3), (100, 90), &path, config.max_path_len());
        let start = LocalCoord::new(2, 3).expect("in bounds");
        let goal = LocalCoord::new(100, 90).expect("in bounds");
        assert_eq!(path.cost(), manhattan(start, goal), "A* must be optimal");
        assert!(!path.is_empty());
    }

    #[test]
    fn path_detours_around_wall_at_minimum_cost() {
        let mut grid = open_grid();
        for y in 0..10 {
            grid.set(5, y, TerrainKind::Rock).expect("in bounds");
        }
        let config = PathConfig::default();
        let path = find_path(&grid, (2, 5), (8, 5), TerrainKind::is_walkable, config)
            .expect("a detour around the wall exists");
        assert_valid_path(&grid, (2, 5), (8, 5), &path, config.max_path_len());
        // The wall blocks column 5 in rows 0..=9, so the shortest crossing
        // goes via row 10: 5 down + 6 east + 5 up.
        assert_eq!(path.cost(), 16);
    }

    #[test]
    fn equal_f_ties_follow_the_documented_break_order() {
        let grid = open_grid();
        let config = PathConfig::default();
        // Equal (f, h) candidates resolve by row-major coord: (1, 0) sits in
        // row 0 and therefore expands before (0, 1).
        let diagonal =
            find_path(&grid, (0, 0), (1, 1), TerrainKind::is_walkable, config).expect("reachable");
        let coords: Vec<(i32, i32)> = diagonal.coords().iter().map(|c| (c.x(), c.y())).collect();
        assert_eq!(coords, vec![(0, 0), (1, 0), (1, 1)]);

        let reversed =
            find_path(&grid, (1, 1), (0, 0), TerrainKind::is_walkable, config).expect("reachable");
        let coords: Vec<(i32, i32)> = reversed.coords().iter().map(|c| (c.x(), c.y())).collect();
        assert_eq!(coords, vec![(1, 1), (1, 0), (0, 0)]);

        let longer =
            find_path(&grid, (0, 0), (2, 2), TerrainKind::is_walkable, config).expect("reachable");
        let coords: Vec<(i32, i32)> = longer.coords().iter().map(|c| (c.x(), c.y())).collect();
        assert_eq!(coords, vec![(0, 0), (1, 0), (2, 0), (2, 1), (2, 2)]);
    }

    #[test]
    fn repeated_calls_are_bit_identical() {
        let grid = open_grid();
        let config = PathConfig::default();
        let reference = find_path(&grid, (0, 0), (127, 127), TerrainKind::is_walkable, config);
        for _ in 0..8 {
            let again = find_path(&grid, (0, 0), (127, 127), TerrainKind::is_walkable, config);
            assert_eq!(again, reference, "equal inputs must give equal results");
        }

        let map = WorldMap::generate(WorldSeed::new(42), WorldGenConfig::default());
        let generated = map.local();
        let reference = find_path(
            generated,
            (0, 0),
            (127, 127),
            TerrainKind::is_walkable,
            config,
        );
        for _ in 0..4 {
            let again = find_path(
                generated,
                (0, 0),
                (127, 127),
                TerrainKind::is_walkable,
                config,
            );
            assert_eq!(again, reference, "generated-map query must be stable");
        }
    }

    #[test]
    fn start_equals_goal_returns_the_trivial_path() {
        let grid = open_grid();
        let path = find_path(
            &grid,
            (7, 7),
            (7, 7),
            TerrainKind::is_walkable,
            PathConfig::default(),
        )
        .expect("trivial path exists");
        assert_eq!(path.len(), 1);
        assert_eq!(path.cost(), 0);
        assert_eq!(
            path.coords().first().copied(),
            LocalCoord::new(7, 7).as_ref().copied()
        );
        // The trivial path consumes no expansion budget at all.
        let free = find_path(
            &grid,
            (7, 7),
            (7, 7),
            TerrainKind::is_walkable,
            PathConfig::new(0, 1),
        )
        .expect("trivial path ignores the node budget");
        assert_eq!(free, path);
    }

    #[test]
    fn zero_path_cap_admits_no_path() {
        let grid = open_grid();
        let config = PathConfig::new(usize::MAX, 0);
        assert_eq!(
            find_path(&grid, (0, 0), (1, 1), TerrainKind::is_walkable, config),
            Err(PathError::Unreachable)
        );
        assert_eq!(
            find_path(&grid, (7, 7), (7, 7), TerrainKind::is_walkable, config),
            Err(PathError::Unreachable)
        );
    }

    #[test]
    fn walled_goal_is_unreachable_even_with_unlimited_budgets() {
        let mut grid = open_grid();
        for (x, y) in [
            (9, 9),
            (10, 9),
            (11, 9),
            (9, 10),
            (11, 10),
            (9, 11),
            (10, 11),
            (11, 11),
        ] {
            grid.set(x, y, TerrainKind::Rock).expect("in bounds");
        }
        // Unlimited budgets still terminate: expansions never exceed the
        // walkable cell count, so the search cannot run unbounded.
        let config = PathConfig::new(usize::MAX, usize::MAX);
        assert_eq!(
            find_path(&grid, (0, 0), (10, 10), TerrainKind::is_walkable, config),
            Err(PathError::Unreachable)
        );
    }

    #[test]
    fn fully_impassable_grid_terminates_without_panic() {
        for kind in [TerrainKind::Rock, TerrainKind::Water] {
            let grid = LocalGrid::filled_with(kind);
            assert_eq!(
                find_path(
                    &grid,
                    (0, 0),
                    (127, 127),
                    TerrainKind::is_walkable,
                    PathConfig::default()
                ),
                Err(PathError::NonWalkable { coord: (0, 0) })
            );
        }
    }

    #[test]
    fn node_budget_limits_expansions_exactly() {
        let grid = open_grid();
        // The straight-line query expands exactly five cells before the
        // goal pops, so a budget of five succeeds and four fails.
        let enough = find_path(
            &grid,
            (0, 0),
            (5, 0),
            TerrainKind::is_walkable,
            PathConfig::new(5, LOCAL_GRID_CELL_COUNT),
        )
        .expect("budget of five suffices");
        assert_eq!(enough.cost(), 5);
        assert_eq!(
            find_path(
                &grid,
                (0, 0),
                (5, 0),
                TerrainKind::is_walkable,
                PathConfig::new(4, LOCAL_GRID_CELL_COUNT),
            ),
            Err(PathError::LimitExceeded {
                nodes: 4,
                budget: 4
            })
        );
        assert_eq!(
            find_path(
                &grid,
                (0, 0),
                (5, 0),
                TerrainKind::is_walkable,
                PathConfig::new(0, LOCAL_GRID_CELL_COUNT),
            ),
            Err(PathError::LimitExceeded {
                nodes: 0,
                budget: 0
            })
        );
    }

    #[test]
    fn path_length_cap_bounds_the_result() {
        let grid = open_grid();
        // (0, 0) -> (4, 0) needs exactly five cells.
        let exact = find_path(
            &grid,
            (0, 0),
            (4, 0),
            TerrainKind::is_walkable,
            PathConfig::new(LOCAL_GRID_CELL_COUNT, 5),
        )
        .expect("a five-cell path fits the cap");
        assert_eq!(exact.len(), 5);
        assert_valid_path(&grid, (0, 0), (4, 0), &exact, 5);
        // A shorter cap makes the goal unreachable within the cap; this is
        // the documented terminal outcome for a length-limited query.
        for cap in [1_usize, 4] {
            assert_eq!(
                find_path(
                    &grid,
                    (0, 0),
                    (4, 0),
                    TerrainKind::is_walkable,
                    PathConfig::new(LOCAL_GRID_CELL_COUNT, cap),
                ),
                Err(PathError::Unreachable),
                "cap {cap} must reject the five-cell path"
            );
        }
    }

    #[test]
    fn out_of_bounds_endpoints_are_documented_errors() {
        let grid = open_grid();
        let config = PathConfig::default();
        for raw in [(-1, 0), (0, -1), (128, 0), (0, 128), (i32::MAX, i32::MIN)] {
            assert_eq!(
                find_path(&grid, raw, (3, 3), TerrainKind::is_walkable, config),
                Err(PathError::OutOfBounds { coord: raw }),
                "out-of-bounds start {raw:?}"
            );
            assert_eq!(
                find_path(&grid, (3, 3), raw, TerrainKind::is_walkable, config),
                Err(PathError::OutOfBounds { coord: raw }),
                "out-of-bounds goal {raw:?}"
            );
        }
        // The start is validated before the goal.
        assert_eq!(
            find_path(
                &grid,
                (-1, -1),
                (200, 200),
                TerrainKind::is_walkable,
                config
            ),
            Err(PathError::OutOfBounds { coord: (-1, -1) })
        );
    }

    #[test]
    fn non_walkable_endpoints_are_documented_errors() {
        let mut grid = open_grid();
        grid.set(3, 3, TerrainKind::Rock).expect("in bounds");
        grid.set(4, 4, TerrainKind::Water).expect("in bounds");
        let config = PathConfig::default();
        assert_eq!(
            find_path(&grid, (3, 3), (0, 0), TerrainKind::is_walkable, config),
            Err(PathError::NonWalkable { coord: (3, 3) })
        );
        assert_eq!(
            find_path(&grid, (0, 0), (4, 4), TerrainKind::is_walkable, config),
            Err(PathError::NonWalkable { coord: (4, 4) })
        );
        // start == goal on a non-walkable cell is a terminal error, not a
        // panic and not a trivial path.
        assert_eq!(
            find_path(&grid, (3, 3), (3, 3), TerrainKind::is_walkable, config),
            Err(PathError::NonWalkable { coord: (3, 3) })
        );
    }

    #[test]
    fn default_config_is_complete_on_the_open_grid() {
        let grid = open_grid();
        // The longest Manhattan pair on the grid: 255 cells, far below the
        // default caps, so the default config never limits a real query.
        let path = find_path(
            &grid,
            (0, 0),
            (127, 127),
            TerrainKind::is_walkable,
            PathConfig::default(),
        )
        .expect("default config admits the full grid");
        assert_eq!(path.len(), 255);
        assert_eq!(path.cost(), 254);
    }

    #[test]
    fn generated_map_query_is_deterministic_and_valid() {
        let map = WorldMap::generate(WorldSeed::new(42), WorldGenConfig::default());
        let grid = map.local();
        let walkable_cells: Vec<LocalCoord> = grid
            .coords()
            .filter(|coord| {
                grid.get_index(coord.index())
                    .expect("coords() yields in-range cells")
                    .is_walkable()
            })
            .collect();
        let first = walkable_cells.first().expect("generated map is walkable");
        let last = walkable_cells.last().expect("generated map is walkable");
        let start = (first.x(), first.y());
        let goal = (last.x(), last.y());
        let config = PathConfig::default();
        let reference = find_path(grid, start, goal, TerrainKind::is_walkable, config);
        for _ in 0..4 {
            assert_eq!(
                find_path(grid, start, goal, TerrainKind::is_walkable, config),
                reference,
                "generated-map query must be deterministic"
            );
        }
        // Whatever the terminal outcome, it is a documented variant; a
        // found path satisfies every validity invariant.
        match reference {
            Ok(path) => assert_valid_path(grid, start, goal, &path, config.max_path_len()),
            Err(
                PathError::Unreachable
                | PathError::LimitExceeded { .. }
                | PathError::OutOfBounds { .. }
                | PathError::NonWalkable { .. },
            ) => {}
        }
    }
}
