// Authored by Kimi Code (AI coding agent) — task CHRON-023.
//! Activity-site baseline for CHRON-023: `find_nearest` query cost and
//! `record_work` throughput over a deterministic 20-site micro-settlement
//! fixture (the upper end of the spec's ~6–20 site range), plus the
//! incremental RSS of the retained collection. Median of ten post-warm-up
//! samples, integer nanoseconds per op. This is a small static-data baseline,
//! not a Phase 1 hard gate; the per-Person work-loop cost is realized in
//! CHRON-027/CHRON-028. RSS is sampled best-effort via `ps` (KiB units on
//! macOS/Linux).

use std::hint::black_box;
use std::process::Command;
use std::time::Instant;

use palimpsest_sim_world::{
    ActivitySite, ActivitySites, LocalCoord, SiteKind, WorkCounter, WorldGenConfig, WorldMap,
    WorldSeed,
};

/// Site count of the benchmark fixture (micro-settlement upper bound).
const SITE_COUNT: usize = 20;
/// `find_nearest` queries per timed sample.
const QUERY_OPS: usize = 10_000;
/// `record_work` advances per timed sample.
const ADVANCE_OPS: usize = 10_000;
/// Seed for both the fixture map and the deterministic query stream.
const FIXTURE_SEED: u64 = 0x5EED_5EED_5EED_5EED;

/// Affordance cycle for the fixture, matching `place_defaults`.
const KIND_CYCLE: [SiteKind; 3] = [SiteKind::Meal, SiteKind::Rest, SiteKind::Work];

fn main() {
    let mut arguments = std::env::args().skip(1);
    let sample_count = arguments.next().map_or(10, |value| {
        value.parse::<usize>().expect("sample count must be usize")
    });
    let warmup_count = arguments.next().map_or(2, |value| {
        value.parse::<usize>().expect("warmup count must be usize")
    });
    assert!(sample_count > 0, "sample count must be positive");

    let map = WorldMap::generate(WorldSeed::new(FIXTURE_SEED), WorldGenConfig::default());
    let sites = fixture_sites(&map);
    let work_coords: Vec<LocalCoord> = sites
        .sites_of(SiteKind::Work)
        .map(ActivitySite::coord)
        .collect();
    assert!(!work_coords.is_empty(), "fixture must contain work sites");

    for _ in 0..warmup_count {
        black_box(run_queries(&sites));
        black_box(run_advances(&sites, &work_coords));
    }

    let mut query_samples = Vec::with_capacity(sample_count);
    let mut advance_samples = Vec::with_capacity(sample_count);
    let mut query_checksum = None;
    for _ in 0..sample_count {
        let started = Instant::now();
        let checksum = run_queries(&sites);
        query_samples.push(
            started.elapsed().as_nanos() / u128::try_from(QUERY_OPS).expect("op count fits u128"),
        );
        match query_checksum {
            None => query_checksum = Some(checksum),
            Some(reference) => assert_eq!(
                checksum, reference,
                "find_nearest must be deterministic across samples"
            ),
        }

        let started = Instant::now();
        let advanced = run_advances(&sites, &work_coords);
        advance_samples.push(
            started.elapsed().as_nanos() / u128::try_from(ADVANCE_OPS).expect("op count fits u128"),
        );
        assert_eq!(
            advanced,
            u64::try_from(ADVANCE_OPS).expect("op count fits u64"),
            "every record_work call must advance a counter"
        );
    }

    let rss_before = current_rss_bytes();
    let retained = fixture_sites(&map);
    black_box(&retained);
    let rss_after = current_rss_bytes();
    let rss_delta = rss_after
        .zip(rss_before)
        .map(|(after, before)| after.saturating_sub(before));

    query_samples.sort_unstable();
    advance_samples.sort_unstable();
    println!(
        "{{\"sites\":{SITE_COUNT},\"samples\":{sample_count},\
         \"query_ops_per_sample\":{QUERY_OPS},\"advance_ops_per_sample\":{ADVANCE_OPS},\
         \"find_nearest_min_ns_per_op\":{},\"find_nearest_median_ns_per_op\":{},\
         \"find_nearest_max_ns_per_op\":{},\"record_work_min_ns_per_op\":{},\
         \"record_work_median_ns_per_op\":{},\"record_work_max_ns_per_op\":{},\
         \"rss_delta_bytes\":{},\"find_nearest_checksum\":{}}}",
        query_samples.first().expect("samples exist"),
        query_samples[sample_count / 2],
        query_samples[sample_count - 1],
        advance_samples.first().expect("samples exist"),
        advance_samples[sample_count / 2],
        advance_samples[sample_count - 1],
        json_u64(rss_delta),
        query_checksum.expect("at least one sample ran"),
    );
}

/// Builds the deterministic fixture: `SITE_COUNT` sites on walkable cells
/// spread evenly over the row-major walkable sequence, kinds cycling
/// `Meal`/`Rest`/`Work`.
fn fixture_sites(map: &WorldMap) -> ActivitySites {
    let walkable: Vec<LocalCoord> = map
        .local()
        .coords()
        .filter(|coord| {
            map.local()
                .get(coord.x(), coord.y())
                .is_some_and(|kind| kind.is_walkable())
        })
        .collect();
    assert!(
        walkable.len() >= SITE_COUNT,
        "generated map must offer enough walkable cells"
    );
    let last = walkable.len() - 1;
    let sites: Vec<ActivitySite> = (0..SITE_COUNT)
        .map(|slot| {
            let ordinal = slot * last / (SITE_COUNT - 1);
            let kind = KIND_CYCLE[slot % KIND_CYCLE.len()];
            ActivitySite::new(map, walkable[ordinal], kind).expect("filtered walkable")
        })
        .collect();
    ActivitySites::new(sites).expect("spread ordinals are distinct")
}

/// Runs `QUERY_OPS` deterministic nearest-site queries; returns a checksum
/// used to assert determinism across samples.
fn run_queries(sites: &ActivitySites) -> u64 {
    let mut stream = Splitmix64(FIXTURE_SEED);
    let mut checksum = 0_u64;
    for op in 0..QUERY_OPS {
        let value = stream.next_value();
        let x = i32::try_from(value & 0x7F).expect("masked to 0..=127");
        let y = i32::try_from((value >> 7) & 0x7F).expect("masked to 0..=127");
        let from = LocalCoord::new(x, y).expect("masked coordinates are in bounds");
        let kind = KIND_CYCLE[op % KIND_CYCLE.len()];
        if let Some(found) = black_box(sites).find_nearest(black_box(from), kind) {
            checksum =
                checksum.wrapping_add(u64::try_from(found.index()).expect("cell index fits u64"));
        }
    }
    checksum
}

/// Clones the fixture and runs `ADVANCE_OPS` work recordings; returns the
/// total observed count so the caller can assert nothing saturated away.
fn run_advances(sites: &ActivitySites, work_coords: &[LocalCoord]) -> u64 {
    let mut sites = sites.clone();
    let mut sink = 0_u64;
    for coord in work_coords.iter().cycle().take(ADVANCE_OPS) {
        let count = black_box(&mut sites)
            .record_work(black_box(*coord))
            .expect("fixture work site");
        sink = sink.wrapping_add(count);
    }
    black_box(sink);
    sites
        .sites_of(SiteKind::Work)
        .filter_map(ActivitySite::work)
        .map(WorkCounter::get)
        .sum()
}

/// Runs the fixed 20-site workload for incremental-memory measurement.
/// Preparation is complete before the first observation. The operation phase
/// performs exactly one query batch and one advance batch, then verifies both
/// fixed checksums before the retained fixture reaches the second observation.
///
/// # Panics
///
/// Panics when `case` is not `"sites"` or when the deterministic fixture or
/// either fixed checksum assertion fails.
pub fn memory_workload(case: &str, observe: &mut dyn FnMut()) -> u64 {
    const EXPECTED_QUERY_CHECKSUM: u64 = 81_748_317;
    assert_eq!(case, "sites", "unknown site memory workload case: {case}");
    let map = WorldMap::generate(WorldSeed::new(FIXTURE_SEED), WorldGenConfig::default());
    let sites = fixture_sites(&map);
    let work_coords: Vec<LocalCoord> = sites
        .sites_of(SiteKind::Work)
        .map(ActivitySite::coord)
        .collect();
    assert!(!work_coords.is_empty(), "fixture must contain work sites");
    observe();
    let query_checksum = run_queries(&sites);
    let advanced = run_advances(&sites, &work_coords);
    assert_eq!(query_checksum, EXPECTED_QUERY_CHECKSUM);
    assert_eq!(
        advanced,
        u64::try_from(ADVANCE_OPS).expect("op count fits u64")
    );
    black_box(&sites);
    observe();
    query_checksum
}

/// The splitmix64 generator (Stafford/Vigna, public domain reference
/// constants) for a deterministic, reproducible query-coordinate stream.
struct Splitmix64(u64);

impl Splitmix64 {
    fn next_value(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut value = self.0;
        value = (value ^ (value >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        value ^ (value >> 31)
    }
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

fn json_u64(value: Option<u64>) -> String {
    value.map_or_else(|| "null".to_owned(), |number| number.to_string())
}
