use std::hint::black_box;
use std::time::{Duration, Instant};

use palimpsest_sim_scheduler::Scheduler;
use palimpsest_sim_time::SimInstant;

fn main() {
    let mut arguments = std::env::args().skip(1);
    let item_count = arguments.next().map_or(100_000, |value| {
        value.parse::<usize>().expect("item count must be usize")
    });
    let sample_count = arguments.next().map_or(10, |value| {
        value.parse::<usize>().expect("sample count must be usize")
    });
    assert!(item_count > 0, "item count must be positive");
    assert!(sample_count > 0, "sample count must be positive");

    let mut enqueue_samples = Vec::with_capacity(sample_count);
    let mut dequeue_samples = Vec::with_capacity(sample_count);

    for _ in 0..sample_count {
        let mut scheduler = Scheduler::new();
        let enqueue_started = Instant::now();
        for index in 0..item_count {
            let due_seconds = i64::try_from(index % 10_000).expect("modulo result fits i64");
            scheduler
                .schedule_at(SimInstant::from_seconds(due_seconds), black_box(index))
                .expect("scheduler counters have capacity");
        }
        enqueue_samples.push(enqueue_started.elapsed());

        let dequeue_started = Instant::now();
        let mut popped = 0_usize;
        while let Some(item) = scheduler.pop_due(SimInstant::MAX) {
            black_box(item.into_payload());
            popped += 1;
        }
        dequeue_samples.push(dequeue_started.elapsed());
        assert_eq!(popped, item_count, "benchmark must consume every item");
    }

    enqueue_samples.sort_unstable();
    dequeue_samples.sort_unstable();
    let enqueue_median = enqueue_samples[sample_count / 2];
    let dequeue_median = dequeue_samples[sample_count / 2];
    let enqueue_min = enqueue_samples[0];
    let enqueue_max = enqueue_samples[sample_count - 1];
    let dequeue_min = dequeue_samples[0];
    let dequeue_max = dequeue_samples[sample_count - 1];

    println!(
        "{{\"items\":{item_count},\"samples\":{sample_count},\
         \"enqueue_min_ns\":{},\"enqueue_median_ns\":{},\"enqueue_max_ns\":{},\
         \"enqueue_ops_per_second\":{:.3},\
         \"dequeue_min_ns\":{},\"dequeue_median_ns\":{},\"dequeue_max_ns\":{},\
         \"dequeue_ops_per_second\":{:.3}}}",
        enqueue_min.as_nanos(),
        enqueue_median.as_nanos(),
        enqueue_max.as_nanos(),
        operations_per_second(item_count, enqueue_median),
        dequeue_min.as_nanos(),
        dequeue_median.as_nanos(),
        dequeue_max.as_nanos(),
        operations_per_second(item_count, dequeue_median),
    );
}

fn operations_per_second(item_count: usize, elapsed: Duration) -> f64 {
    let count = u32::try_from(item_count).expect("benchmark item count must fit u32");
    f64::from(count) / elapsed.as_secs_f64()
}
