// Authored by Kimi Code (AI coding agent) — task CHRON-022.
//! Needs advance baseline for CHRON-022: per-person `advance` throughput at
//! 100 and 1,000 persons over a simulated one-year interval stepped hourly
//! (8,760 advances per person-year). This is a per-person Phase 1 baseline;
//! the 10-year/100-person collective cost is gated at the kernel level
//! (CHRON-028/032). RSS is sampled best-effort via `ps` (KiB units on
//! macOS/Linux).

use std::hint::black_box;
use std::process::Command;
use std::time::Instant;

use palimpsest_sim_ai::{NeedValue, Needs};
use palimpsest_sim_time::SimDuration;

/// The two scales this baseline reports.
const SCALES: [usize; 2] = [100, 1_000];
/// Hourly cadence assumption: 365 days of hourly advances per person-year.
/// The kernel's real cadence is decided by CHRON-028; this measures per-call
/// cost at a documented cadence.
const HOURS_PER_YEAR: u64 = 8_760;

fn main() {
    let mut arguments = std::env::args().skip(1);
    let sample_count = arguments.next().map_or(10, |value| {
        value.parse::<usize>().expect("sample count must be usize")
    });
    let warmup_count = arguments.next().map_or(2, |value| {
        value.parse::<usize>().expect("warmup count must be usize")
    });
    assert!(sample_count > 0, "sample count must be positive");

    let one_hour = SimDuration::from_seconds(3_600).expect("non-negative duration");
    for scale in SCALES {
        for _ in 0..warmup_count {
            black_box(advance_year(scale, one_hour));
        }

        let mut samples = Vec::with_capacity(sample_count);
        for _ in 0..sample_count {
            let started = Instant::now();
            let all_needs = advance_year(scale, one_hour);
            samples.push(started.elapsed());
            black_box(&all_needs);
        }

        let rss_before = current_rss_bytes();
        let retained = advance_year(scale, one_hour);
        black_box(&retained);
        let rss_after = current_rss_bytes();
        let rss_delta = rss_after
            .zip(rss_before)
            .map(|(after, before)| after.saturating_sub(before));

        let advances_total = u64::try_from(scale).expect("scale fits u64") * HOURS_PER_YEAR;
        let advances_f64 = f64::from(u32::try_from(advances_total).expect("advances fit u32"));
        samples.sort_unstable();
        let median = samples[sample_count / 2];
        println!(
            "{{\"persons\":{scale},\"samples\":{sample_count},\
             \"advances_per_person_year\":{HOURS_PER_YEAR},\
             \"advances_total\":{advances_total},\
             \"year_min_ns\":{},\"year_median_ns\":{},\"year_max_ns\":{},\
             \"advances_per_second\":{:.3},\"rss_delta_bytes\":{}}}",
            samples.first().expect("samples exist").as_nanos(),
            median.as_nanos(),
            samples[sample_count - 1].as_nanos(),
            advances_f64 / median.as_secs_f64(),
            json_u64(rss_delta),
        );
    }
}

/// Advances `count` persons through one simulated year at hourly cadence.
fn advance_year(count: usize, one_hour: SimDuration) -> Vec<Needs> {
    let mut all_needs = vec![Needs::default(); count];
    for _ in 0..HOURS_PER_YEAR {
        for needs in &mut all_needs {
            *needs = black_box(needs.advance(one_hour));
        }
    }
    // Correctness: after a year with no eat/rest, every drive saturates.
    assert!(
        all_needs
            .iter()
            .all(|needs| needs.hunger() == NeedValue::MAX && needs.fatigue() == NeedValue::MAX),
        "a year without consumption must saturate both drives"
    );
    all_needs
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
