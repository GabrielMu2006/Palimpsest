// Authored by Kimi Code (AI coding agent) — task CHRON-021.
//! Person spawn baseline for CHRON-021: spawn + attach throughput at 100 and
//! 1,000 persons with a retained-runtime RSS delta. This is the Phase 1
//! person baseline for CHRON-028's 100-person kernel, not the 10K scale gate.
//! RSS is sampled best-effort via `ps` (KiB units on macOS/Linux).

use std::hint::black_box;
use std::process::Command;
use std::time::Instant;

use palimpsest_sim_core::{EntityId, EntityIdAllocator, PersonRuntime};
use palimpsest_sim_world::LocalCoord;

/// The two scales this baseline reports.
const SCALES: [usize; 2] = [100, 1_000];

fn main() {
    let mut arguments = std::env::args().skip(1);
    let sample_count = arguments.next().map_or(10, |value| {
        value.parse::<usize>().expect("sample count must be usize")
    });
    let warmup_count = arguments.next().map_or(2, |value| {
        value.parse::<usize>().expect("warmup count must be usize")
    });
    assert!(sample_count > 0, "sample count must be positive");

    for scale in SCALES {
        for _ in 0..warmup_count {
            black_box(spawn_all(scale));
        }

        let mut samples = Vec::with_capacity(sample_count);
        for _ in 0..sample_count {
            let started = Instant::now();
            let runtime = spawn_all(scale);
            samples.push(started.elapsed());
            assert_eq!(runtime.person_count(), scale);
            black_box(&runtime);
        }

        let rss_before = current_rss_bytes();
        let retained = spawn_all(scale);
        black_box(&retained);
        let rss_after = current_rss_bytes();
        let rss_delta = rss_after
            .zip(rss_before)
            .map(|(after, before)| after.saturating_sub(before));
        let checksum = visible_state_checksum(&retained, scale);

        samples.sort_unstable();
        let median = samples[sample_count / 2];
        let persons = u64::try_from(scale).expect("scale fits u64");
        let persons_f64 = f64::from(u32::try_from(scale).expect("scale fits u32"));
        let per_person_bytes = rss_delta.map(|delta| delta / persons);
        println!(
            "{{\"persons\":{scale},\"samples\":{sample_count},\
             \"spawn_min_ns\":{},\"spawn_median_ns\":{},\"spawn_max_ns\":{},\
             \"spawns_per_second\":{:.3},\"rss_delta_bytes\":{},\
             \"per_person_bytes\":{},\"checksum\":{checksum}}}",
            samples.first().expect("samples exist").as_nanos(),
            median.as_nanos(),
            samples[sample_count - 1].as_nanos(),
            persons_f64 / median.as_secs_f64(),
            json_u64(rss_delta),
            json_u64(per_person_bytes),
        );
    }
}

fn spawn_all(count: usize) -> PersonRuntime {
    let mut runtime = PersonRuntime::new();
    let mut allocator = EntityIdAllocator::default();
    for index in 0..count {
        let x = i32::try_from(index % 128).expect("grid axis fits i32");
        let y = i32::try_from((index / 128) % 128).expect("grid axis fits i32");
        let location = LocalCoord::new(x, y).expect("in bounds");
        let id = runtime
            .spawn(&mut allocator, location)
            .expect("identity capacity");
        black_box(id);
    }
    runtime
}

fn visible_state_checksum(runtime: &PersonRuntime, count: usize) -> u64 {
    let mut checksum = 0_u64;
    for raw in 1..=count {
        let id = EntityId::new(u64::try_from(raw).expect("index fits u64")).expect("non-zero");
        let view = runtime.get(id).expect("spawned person exists");
        checksum = checksum
            .wrapping_add(view.id().get())
            .wrapping_add(u64::from(
                u32::try_from(view.location().x()).expect("non-negative"),
            ))
            .wrapping_add(u64::from(
                u32::try_from(view.location().y()).expect("non-negative"),
            ));
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

/// Retains the complete person workload for the memory benchmark adapter.
/// The callback marks the boundary around the measured allocation/operation.
///
/// # Panics
///
/// Panics when `case` is not `"100"` or `"1000"`.
pub fn memory_workload(case: &str, observe: &mut dyn FnMut()) -> u64 {
    let count = match case {
        "100" => 100,
        "1000" => 1_000,
        other => panic!("invalid person memory workload selector: {other}"),
    };
    observe();
    let runtime = spawn_all(count);
    assert_eq!(runtime.person_count(), count);
    let checksum = visible_state_checksum(&runtime, count);
    let expected = match count {
        100 => 10_000,
        1_000 => 566_168,
        _ => unreachable!(),
    };
    assert_eq!(checksum, expected);
    black_box(&runtime);
    observe();
    checksum
}

#[cfg(test)]
mod tests {
    use super::memory_workload;

    #[test]
    fn memory_adapter_observes_twice_and_matches_golden() {
        let mut callbacks = 0;
        let checksum = memory_workload("100", &mut || callbacks += 1);
        assert_eq!(callbacks, 2);
        assert_eq!(checksum, 10_000);
    }

    #[test]
    fn memory_adapter_matches_1000_golden() {
        let mut callbacks = 0;
        let checksum = memory_workload("1000", &mut || callbacks += 1);
        assert_eq!(callbacks, 2);
        assert_eq!(checksum, 566_168);
    }

    #[test]
    #[should_panic(expected = "invalid person memory workload selector")]
    fn memory_adapter_rejects_invalid_selector() {
        let _ = memory_workload("bad", &mut || {});
    }
}
