// Authored by Kimi Code (AI coding agent) — task CHRON-019.
//! Container baseline for CHRON-019: build cost of `LocalGrid::from_cells`,
//! full 16,384-cell sequential `get` scan cost, and incremental RSS of one
//! retained grid. This is a container baseline for CHRON-020/CHRON-024, not a
//! Phase 1 performance gate. RSS is sampled best-effort via `ps` (KiB units
//! on macOS/Linux).

use std::hint::black_box;
use std::process::Command;
use std::time::Instant;

use palimpsest_sim_world::{LOCAL_GRID_CELL_COUNT, LOCAL_GRID_HEIGHT, LOCAL_GRID_WIDTH, LocalGrid};

fn main() {
    let mut arguments = std::env::args().skip(1);
    let sample_count = arguments.next().map_or(10, |value| {
        value.parse::<usize>().expect("sample count must be usize")
    });
    let warmup_count = arguments.next().map_or(2, |value| {
        value.parse::<usize>().expect("warmup count must be usize")
    });
    assert!(sample_count > 0, "sample count must be positive");

    let expected_checksum = expected_checksum();

    for _ in 0..warmup_count {
        let grid = build_grid();
        assert_eq!(scan(&grid), expected_checksum);
        black_box(grid);
    }

    let mut build_samples = Vec::with_capacity(sample_count);
    let mut scan_samples = Vec::with_capacity(sample_count);
    for _ in 0..sample_count {
        let cells = cells_pattern();
        let build_started = Instant::now();
        let grid = LocalGrid::from_cells(black_box(cells)).expect("exact cell count");
        build_samples.push(build_started.elapsed());

        let scan_started = Instant::now();
        let checksum = scan(&grid);
        scan_samples.push(scan_started.elapsed());
        assert_eq!(checksum, expected_checksum);
        black_box(grid);
    }

    let rss_before = current_rss_bytes();
    let retained = build_grid();
    black_box(&retained);
    let rss_after = current_rss_bytes();
    assert_eq!(scan(&retained), expected_checksum);

    build_samples.sort_unstable();
    scan_samples.sort_unstable();
    let rss_delta = rss_after
        .zip(rss_before)
        .map(|(after, before)| after.saturating_sub(before));

    println!(
        "{{\"cells\":{LOCAL_GRID_CELL_COUNT},\"samples\":{sample_count},\
         \"warmups\":{warmup_count},\
         \"build_min_ns\":{},\"build_median_ns\":{},\"build_max_ns\":{},\
         \"scan_min_ns\":{},\"scan_median_ns\":{},\"scan_max_ns\":{},\
         \"rss_before_bytes\":{},\"rss_after_bytes\":{},\"rss_delta_bytes\":{},\
         \"checksum\":{expected_checksum}}}",
        build_samples.first().expect("samples exist").as_nanos(),
        build_samples[sample_count / 2].as_nanos(),
        build_samples[sample_count - 1].as_nanos(),
        scan_samples.first().expect("samples exist").as_nanos(),
        scan_samples[sample_count / 2].as_nanos(),
        scan_samples[sample_count - 1].as_nanos(),
        json_u64(rss_before),
        json_u64(rss_after),
        json_u64(rss_delta),
    );
}

fn cells_pattern() -> Vec<u64> {
    (0..LOCAL_GRID_CELL_COUNT)
        .map(|index| u64::try_from(index % 251).expect("pattern fits u64"))
        .collect()
}

fn build_grid() -> LocalGrid<u64> {
    LocalGrid::from_cells(cells_pattern()).expect("exact cell count")
}

fn expected_checksum() -> u64 {
    (0..LOCAL_GRID_CELL_COUNT).fold(0_u64, |acc, index| {
        acc.wrapping_add(u64::try_from(index % 251).expect("pattern fits u64"))
    })
}

fn scan(grid: &LocalGrid<u64>) -> u64 {
    let mut checksum = 0_u64;
    for y in 0..LOCAL_GRID_HEIGHT {
        for x in 0..LOCAL_GRID_WIDTH {
            let xi = i32::try_from(x).expect("grid axis fits i32");
            let yi = i32::try_from(y).expect("grid axis fits i32");
            let cell = grid.get(xi, yi).expect("in-bounds coordinate");
            checksum = checksum.wrapping_add(*black_box(cell));
        }
    }
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

fn json_u64(value: Option<u64>) -> String {
    value.map_or_else(|| "null".to_owned(), |number| number.to_string())
}
