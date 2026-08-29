use std::time::{Duration, Instant};

use palimpsest_headless_runner::run;
use serde::Serialize;

#[derive(Serialize)]
struct Metrics {
    mode: &'static str,
    entities: u64,
    final_sim_second: i64,
    samples: usize,
    median_ns: u128,
    min_ns: u128,
    max_ns: u128,
    median_entity_work_per_second: f64,
}

fn main() {
    let mut args = std::env::args().skip(1);
    let entities = args
        .next()
        .map_or(10_000, |value| value.parse().expect("entity count"));
    let seconds = args
        .next()
        .map_or(1_000, |value| value.parse().expect("final sim second"));
    let samples = args
        .next()
        .map_or(10, |value| value.parse().expect("sample count"));
    assert!(entities > 0 && seconds >= 0 && samples > 0);

    let mut elapsed = Vec::with_capacity(samples);
    for _ in 0..samples {
        let started = Instant::now();
        let result = run(entities, seconds).expect("valid comparison workload");
        elapsed.push(started.elapsed());
        assert_eq!(result.processed_work, entities);
        assert_eq!(result.remaining_scheduled, 0);
    }
    elapsed.sort_unstable();
    let median = elapsed[samples / 2];
    let metrics = Metrics {
        mode: "headless",
        entities,
        final_sim_second: seconds,
        samples,
        median_ns: median.as_nanos(),
        min_ns: elapsed[0].as_nanos(),
        max_ns: elapsed[samples - 1].as_nanos(),
        median_entity_work_per_second: rate(entities, median),
    };
    println!("{}", serde_json::to_string(&metrics).expect("metrics JSON"));
}

fn rate(entities: u64, elapsed: Duration) -> f64 {
    let count = u32::try_from(entities).expect("entity count fits u32");
    f64::from(count) / elapsed.as_secs_f64()
}
