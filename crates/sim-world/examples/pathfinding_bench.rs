// Authored by Kimi Code (AI coding agent) — task CHRON-024.
//! `bench_pathfinding` baseline for CHRON-024 (Master Spec §75): A* queries
//! over the fixed CHRON-020 default-config map (seed 42), release build,
//! 10 post-warm-up samples per query with the median reported. The query
//! set is derived deterministically from BFS component labelling and covers
//! reachable pairs of growing distance, a cross-component (provably
//! unreachable) pair when the map has more than one walkable component, a
//! node-budget-limited query, and a path-length-limited query. Correctness
//! assertions (outcome checks, path validity, per-sample determinism, the
//! seed-42 golden map hash) remain enabled. RSS is sampled best-effort via
//! `ps` (KiB units on macOS/Linux), mirroring `worldgen_bench`.
//!
//! `find_path` deliberately exposes no expansion counter (the CHRON-024
//! API contract fixes `Path` to coords + cost), so exact per-query
//! expansion counts are derived black-box through the node budget: the
//! search is deterministic, therefore with budget `B < E` it returns
//! `LimitExceeded` and with `B >= E` the terminal outcome; bisection finds
//! the threshold `E`. Budget-limited queries report `error.nodes`
//! directly.

use std::collections::VecDeque;
use std::hint::black_box;
use std::process::Command;
use std::time::Instant;

use palimpsest_sim_world::{
    LOCAL_GRID_CELL_COUNT, LocalCoord, LocalGrid, Path, PathConfig, PathError, TerrainKind,
    WorldGenConfig, WorldMap, WorldSeed, find_path,
};

/// Fixed map seed for this baseline (CHRON-020 default generator config).
const SEED: u64 = 42;
/// Golden FNV-1a content hash locked by the CHRON-020 worldgen tests.
const SEED_42_MAP_HASH: u64 = 8_056_959_030_977_719_378;
/// Timed samples per query; the median (sorted index `SAMPLES / 2`) is
/// reported, matching `worldgen_bench`.
const SAMPLES: usize = 10;
/// Warm-up runs per query before sampling.
const WARMUPS: usize = 2;

/// BFS distance tiers (in steps) for the short/medium reachable queries;
/// the long query uses the farthest reachable cell.
const SHORT_STEPS: u32 = 8;
const MEDIUM_STEPS: u32 = 40;

/// One benchmark query with the terminal outcome it must produce.
struct Query {
    name: &'static str,
    start: (i32, i32),
    goal: (i32, i32),
    config: PathConfig,
    expected: Expected,
}

/// The terminal outcome a query must produce; asserted before timing.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Expected {
    Found,
    Unreachable,
    LimitExceeded,
}

impl Expected {
    fn as_str(self) -> &'static str {
        match self {
            Self::Found => "found",
            Self::Unreachable => "unreachable",
            Self::LimitExceeded => "limit_exceeded",
        }
    }
}

fn main() {
    let map = WorldMap::generate(WorldSeed::new(SEED), WorldGenConfig::default());
    assert_eq!(
        content_hash(map.local()),
        SEED_42_MAP_HASH,
        "seed 42 map must match the CHRON-020 golden hash"
    );
    let grid = map.local();
    let queries = build_queries(grid);
    let has_unreachable_pair = queries.iter().any(|query| query.name == "unreachable");

    let rss_before = current_rss_bytes();
    let mut retained: Vec<Result<Path, PathError>> = Vec::new();
    let mut max_nodes_expanded = 0_usize;
    let mut peak_path_len = 0_usize;

    for query in &queries {
        let reference = run_query(grid, query);
        check_outcome(grid, query, &reference);

        for _ in 0..WARMUPS {
            let _ = black_box(run_query(grid, query));
        }
        let mut samples = Vec::with_capacity(SAMPLES);
        for _ in 0..SAMPLES {
            let started = Instant::now();
            let result = black_box(run_query(grid, query));
            samples.push(started.elapsed().as_nanos());
            assert_eq!(
                result, reference,
                "determinism: repeated samples diverged for query {}",
                query.name
            );
        }
        samples.sort_unstable();

        let nodes = match &reference {
            Err(PathError::LimitExceeded { nodes, .. }) => *nodes,
            _ => measure_expansions(grid, query),
        };
        max_nodes_expanded = max_nodes_expanded.max(nodes);
        let (path_len, cost) = match &reference {
            Ok(path) => {
                peak_path_len = peak_path_len.max(path.len());
                (Some(path.len()), Some(u64::from(path.cost())))
            }
            Err(_) => (None, None),
        };
        println!(
            "{{\"query\":\"{}\",\"seed\":{SEED},\"start\":[{},{}],\"goal\":[{},{}],\
             \"outcome\":\"{}\",\"max_nodes\":{},\"max_path_len\":{},\
             \"samples\":{SAMPLES},\"min_ns\":{},\"median_ns\":{},\"max_ns\":{},\
             \"nodes_expanded\":{},\"path_len\":{},\"cost\":{}}}",
            query.name,
            query.start.0,
            query.start.1,
            query.goal.0,
            query.goal.1,
            query.expected.as_str(),
            query.config.max_nodes(),
            query.config.max_path_len(),
            samples.first().expect("samples exist"),
            samples[SAMPLES / 2],
            samples[SAMPLES - 1],
            nodes,
            json_usize(path_len),
            json_u64(cost),
        );
        retained.push(reference);
    }
    black_box(&retained);
    let rss_after = current_rss_bytes();
    let rss_delta = rss_after
        .zip(rss_before)
        .map(|(after, before)| after.saturating_sub(before));

    println!(
        "{{\"summary\":\"bench_pathfinding\",\"seed\":{SEED},\
         \"generator_version\":{},\"queries\":{},\"unreachable_pair\":{has_unreachable_pair},\
         \"max_nodes_expanded\":{max_nodes_expanded},\"peak_path_len\":{peak_path_len},\
         \"rss_delta_bytes\":{},\"map_fnv1a\":{}}}",
        WorldGenConfig::GENERATOR_VERSION,
        queries.len(),
        json_u64(rss_delta),
        content_hash(grid),
    );
}

/// Runs one query with black-boxed inputs so the timer sees real work.
fn run_query(grid: &LocalGrid<TerrainKind>, query: &Query) -> Result<Path, PathError> {
    find_path(
        black_box(grid),
        black_box(query.start),
        black_box(query.goal),
        TerrainKind::is_walkable,
        black_box(query.config),
    )
}

/// Asserts the query's terminal outcome and, for found paths, every path
/// validity invariant (endpoints, 4-adjacency, walkability, length cap,
/// `cost == len - 1`).
fn check_outcome(grid: &LocalGrid<TerrainKind>, query: &Query, result: &Result<Path, PathError>) {
    match (result, query.expected) {
        (Ok(path), Expected::Found) => {
            let coords = path.coords();
            assert!(!coords.is_empty());
            assert!(path.len() <= query.config.max_path_len());
            assert_eq!(
                path.cost(),
                u32::try_from(path.len() - 1).expect("length fits u32")
            );
            assert_eq!(coords.first().map(|c| (c.x(), c.y())), Some(query.start));
            assert_eq!(coords.last().map(|c| (c.x(), c.y())), Some(query.goal));
            for pair in coords.windows(2) {
                let step = pair[0].x().abs_diff(pair[1].x()) + pair[0].y().abs_diff(pair[1].y());
                assert_eq!(step, 1, "consecutive path cells must be 4-adjacent");
                assert!(
                    grid.get_index(pair[1].index())
                        .expect("path cells are in bounds")
                        .is_walkable(),
                    "path cells must be walkable"
                );
            }
        }
        (Err(PathError::Unreachable), Expected::Unreachable)
        | (Err(PathError::LimitExceeded { .. }), Expected::LimitExceeded) => {}
        (other, wanted) => panic!(
            "query {} produced {other:?}, expected {}",
            query.name,
            wanted.as_str()
        ),
    }
}

/// Exact expansion count of a query, derived black-box through the node
/// budget (see the module docs). `LOCAL_GRID_CELL_COUNT` always suffices:
/// a search closes each cell at most once, so expansions never exceed the
/// walkable cell count.
fn measure_expansions(grid: &LocalGrid<TerrainKind>, query: &Query) -> usize {
    let mut low = 0_usize;
    let mut high = LOCAL_GRID_CELL_COUNT;
    while low < high {
        let mid = low + (high - low) / 2;
        let probe = PathConfig::new(mid, query.config.max_path_len());
        let result = find_path(
            grid,
            query.start,
            query.goal,
            TerrainKind::is_walkable,
            probe,
        );
        if matches!(result, Err(PathError::LimitExceeded { .. })) {
            low = mid + 1;
        } else {
            high = mid;
        }
    }
    low
}

/// Builds the deterministic query set from BFS component labelling.
fn build_queries(grid: &LocalGrid<TerrainKind>) -> Vec<Query> {
    let full = PathConfig::default();
    let (component_of, components) = label_components(grid);
    let largest = components
        .iter()
        .enumerate()
        .max_by_key(|(index, cells)| (cells.len(), usize::MAX - index))
        .expect("the generated map has a walkable component")
        .0;
    let largest_id = u32::try_from(largest).expect("component count fits u32");
    let anchor = *components[largest]
        .first()
        .expect("the largest component is non-empty");
    let distances = bfs_distances(grid, anchor);
    let pick_at_or_beyond = |tier: u32| -> LocalCoord {
        grid.coords()
            .find(|coord| {
                let distance = distances[coord.index()];
                distance != u32::MAX && distance >= tier
            })
            .unwrap_or_else(|| farthest(&distances))
    };
    let short = pick_at_or_beyond(SHORT_STEPS);
    let medium = pick_at_or_beyond(MEDIUM_STEPS);
    let long = farthest(&distances);

    let mut queries = vec![
        Query {
            name: "trivial",
            start: raw(anchor),
            goal: raw(anchor),
            config: full,
            expected: Expected::Found,
        },
        Query {
            name: "short",
            start: raw(anchor),
            goal: raw(short),
            config: full,
            expected: Expected::Found,
        },
        Query {
            name: "medium",
            start: raw(anchor),
            goal: raw(medium),
            config: full,
            expected: Expected::Found,
        },
        Query {
            name: "long",
            start: raw(anchor),
            goal: raw(long),
            config: full,
            expected: Expected::Found,
        },
    ];

    // Provably unreachable pair: endpoints in different walkable
    // components. Omitted on a hypothetical single-component map.
    let other_component_cell = grid.coords().find(|coord| {
        let component = component_of[coord.index()];
        component != u32::MAX && component != largest_id
    });
    if let Some(goal) = other_component_cell {
        queries.push(Query {
            name: "unreachable",
            start: raw(anchor),
            goal: raw(goal),
            config: full,
            expected: Expected::Unreachable,
        });
    }

    // Budget-limited queries, derived from the long query's exact cost so
    // the limited outcomes are guaranteed rather than map-luck.
    let long_query = Query {
        name: "long",
        start: raw(anchor),
        goal: raw(long),
        config: full,
        expected: Expected::Found,
    };
    let long_expansions = measure_expansions(grid, &long_query);
    let long_path_len = match run_query(grid, &long_query) {
        Ok(path) => path.len(),
        Err(error) => panic!("the long query must be reachable: {error}"),
    };
    // The spawn guarantee makes the largest component >= 64 cells, so the
    // farthest cell is never the anchor itself.
    assert!(long_expansions >= 1, "long query must expand the anchor");
    assert!(long_path_len >= 2, "long query must have a real path");
    queries.push(Query {
        name: "node_budget",
        start: raw(anchor),
        goal: raw(long),
        config: PathConfig::new(long_expansions / 2, full.max_path_len()),
        expected: Expected::LimitExceeded,
    });
    queries.push(Query {
        name: "path_budget",
        start: raw(anchor),
        goal: raw(long),
        config: PathConfig::new(full.max_nodes(), long_path_len / 2),
        expected: Expected::Unreachable,
    });
    queries
}

/// The reachable cell farthest from the BFS root (row-major first on ties).
fn farthest(distances: &[u32]) -> LocalCoord {
    let best = distances
        .iter()
        .enumerate()
        .filter(|(_, distance)| **distance != u32::MAX)
        .max_by_key(|(index, distance)| (**distance, usize::MAX - index))
        .expect("at least the root is reachable")
        .0;
    LocalCoord::from_index(best).expect("a distance index is a valid cell")
}

/// Row-major 4-connected component labelling over walkable cells: per-cell
/// component id (`u32::MAX` for impassable cells) plus each component's
/// cells in row-major order.
fn label_components(grid: &LocalGrid<TerrainKind>) -> (Vec<u32>, Vec<Vec<LocalCoord>>) {
    let mut component_of = vec![u32::MAX; LOCAL_GRID_CELL_COUNT];
    let mut components: Vec<Vec<LocalCoord>> = Vec::new();
    for start in grid.coords() {
        if component_of[start.index()] != u32::MAX || !is_walkable(grid, start) {
            continue;
        }
        let id = u32::try_from(components.len()).expect("component count fits u32");
        let mut cells = Vec::new();
        let mut queue = VecDeque::from([start]);
        component_of[start.index()] = id;
        while let Some(cell) = queue.pop_front() {
            cells.push(cell);
            for next in bench_neighbours(cell) {
                if component_of[next.index()] == u32::MAX && is_walkable(grid, next) {
                    component_of[next.index()] = id;
                    queue.push_back(next);
                }
            }
        }
        components.push(cells);
    }
    (component_of, components)
}

/// BFS distances from `start` over walkable cells (`u32::MAX` = not
/// reached), expanding neighbours in the same fixed order as the search.
fn bfs_distances(grid: &LocalGrid<TerrainKind>, start: LocalCoord) -> Vec<u32> {
    let mut distances = vec![u32::MAX; LOCAL_GRID_CELL_COUNT];
    distances[start.index()] = 0;
    let mut queue = VecDeque::from([start]);
    while let Some(cell) = queue.pop_front() {
        let next_distance = distances[cell.index()] + 1;
        for next in bench_neighbours(cell) {
            if distances[next.index()] == u32::MAX && is_walkable(grid, next) {
                distances[next.index()] = next_distance;
                queue.push_back(next);
            }
        }
    }
    distances
}

/// Whether a cell is walkable terrain.
fn is_walkable(grid: &LocalGrid<TerrainKind>, coord: LocalCoord) -> bool {
    grid.get_index(coord.index())
        .expect("a LocalCoord always indexes in range")
        .is_walkable()
}

/// In-bounds 4-directional neighbours, fixed order east, south, west,
/// north — the same order the search uses.
fn bench_neighbours(coord: LocalCoord) -> impl Iterator<Item = LocalCoord> {
    const DIRECTIONS: [(i32, i32); 4] = [(1, 0), (0, 1), (-1, 0), (0, -1)];
    DIRECTIONS
        .into_iter()
        .filter_map(move |(dx, dy)| LocalCoord::new(coord.x() + dx, coord.y() + dy))
}

/// Raw `(x, y)` pair for a coordinate.
fn raw(coord: LocalCoord) -> (i32, i32) {
    (coord.x(), coord.y())
}

/// The same FNV-1a 64 content hash the CHRON-020 golden-seed tests lock.
fn content_hash(map: &LocalGrid<TerrainKind>) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for cell in map.iter() {
        let byte = match cell {
            TerrainKind::Ground => 0_u64,
            TerrainKind::Water => 1,
            TerrainKind::Rock => 2,
        };
        hash = (hash ^ byte).wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

/// Runs one named query from the fixed seed-42 query fixture for memory
/// measurement. Map generation and query construction are preparation and are
/// observed before the operation; the second observation occurs only after
/// the validated result remains alive. Expansion measurement is deliberately
/// excluded from this operation workload.
///
/// # Panics
///
/// Panics when `case` is not one of the seven documented query selectors, when
/// the seed-42 golden map changes, or when the selected outcome is invalid.
pub fn memory_workload(case: &str, observe: &mut dyn FnMut()) -> u64 {
    const VALID_CASES: [&str; 7] = [
        "trivial",
        "short",
        "medium",
        "long",
        "unreachable",
        "node_budget",
        "path_budget",
    ];
    assert!(
        VALID_CASES.contains(&case),
        "unknown path memory workload case: {case}"
    );
    let map = WorldMap::generate(WorldSeed::new(SEED), WorldGenConfig::default());
    assert_eq!(
        content_hash(map.local()),
        SEED_42_MAP_HASH,
        "seed 42 map must match the CHRON-020 golden hash"
    );
    let grid = map.local();
    let queries = build_queries(grid);
    let query = queries
        .iter()
        .find(|query| query.name == case)
        .expect("validated path query case exists");
    observe();
    let result = run_query(grid, query);
    check_outcome(grid, query, &result);
    let checksum = match &result {
        Ok(path) => u64::try_from(path.len()).expect("path length fits u64"),
        Err(PathError::Unreachable) => u64::MAX,
        Err(PathError::LimitExceeded { nodes, .. }) => {
            u64::try_from(*nodes).expect("node count fits u64")
        }
        Err(error) => panic!("unexpected terminal result for {case}: {error}"),
    };
    black_box(&result);
    observe();
    checksum
}

fn current_rss_bytes() -> Option<u64> {
    let pid = std::process::id().to_string();
    let output = Command::new("ps")
        .args(["-o", "rss=", "-p", &pid])
        .output()
        .ok()?;
    let text = String::from_utf8(output.stdout).ok()?;
    text.trim().parse::<u64>().ok()?.checked_mul(1024)
}

fn json_usize(value: Option<usize>) -> String {
    value.map_or_else(|| "null".to_owned(), |number| number.to_string())
}

fn json_u64(value: Option<u64>) -> String {
    value.map_or_else(|| "null".to_owned(), |number| number.to_string())
}
