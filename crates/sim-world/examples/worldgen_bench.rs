// Authored by Kimi Code (AI coding agent) — task CHRON-020.
//! Worldgen baseline for CHRON-020: full-map generation wall-time over a
//! fixed seed corpus, serialized map bytes, and incremental RSS. This is a
//! determinism/preview-cost baseline for CHRON-024, not a Phase 1 hard gate.
//! RSS is sampled best-effort via `ps` (KiB units on macOS/Linux).

use std::hint::black_box;
use std::process::Command;
use std::time::Instant;

use palimpsest_sim_world::{TerrainKind, WorldGenConfig, WorldMap, WorldSeed};

/// Fixed, documented seed corpus for this baseline.
const SEED_CORPUS: [u64; 3] = [0, 1, 42];

fn main() {
    let mut arguments = std::env::args().skip(1);
    let sample_count = arguments.next().map_or(10, |value| {
        value.parse::<usize>().expect("sample count must be usize")
    });
    let warmup_count = arguments.next().map_or(2, |value| {
        value.parse::<usize>().expect("warmup count must be usize")
    });
    assert!(sample_count > 0, "sample count must be positive");

    let config = WorldGenConfig::default();
    for seed_value in SEED_CORPUS {
        let seed = WorldSeed::new(seed_value);
        for _ in 0..warmup_count {
            black_box(WorldMap::generate(seed, config));
        }

        let mut gen_samples = Vec::with_capacity(sample_count);
        let mut reference = None;
        for _ in 0..sample_count {
            let started = Instant::now();
            let map = WorldMap::generate(seed, config);
            gen_samples.push(started.elapsed());
            match &reference {
                None => reference = Some(map),
                Some(previous) => assert!(
                    previous.local().iter().eq(map.local().iter()),
                    "generation must be deterministic within a run"
                ),
            }
        }
        let map = reference.expect("at least one sample ran");

        let rss_before = current_rss_bytes();
        let retained = WorldMap::generate(seed, config);
        black_box(&retained);
        let rss_after = current_rss_bytes();
        let rss_delta = rss_after
            .zip(rss_before)
            .map(|(after, before)| after.saturating_sub(before));

        gen_samples.sort_unstable();
        let serialized_bytes = serde_json::to_vec(map.local())
            .expect("serialize map")
            .len();
        println!(
            "{{\"seed\":{seed_value},\"samples\":{sample_count},\
             \"gen_min_ns\":{},\"gen_median_ns\":{},\"gen_max_ns\":{},\
             \"map_json_bytes\":{serialized_bytes},\"rss_delta_bytes\":{},\
             \"fnv1a\":{}}}",
            gen_samples.first().expect("samples exist").as_nanos(),
            gen_samples[sample_count / 2].as_nanos(),
            gen_samples[sample_count - 1].as_nanos(),
            json_u64(rss_delta),
            content_hash(map.local()),
        );
    }
}

/// The same FNV-1a 64 content hash the golden-seed tests lock against.
fn content_hash(map: &palimpsest_sim_world::LocalGrid<TerrainKind>) -> u64 {
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
