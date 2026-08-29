use std::hint::black_box;
use std::time::{Duration, Instant};

use palimpsest_sim_core::{EntityId, EventId, EventRecord, SignificanceScore, SimInstant};
use serde::Serialize;

#[derive(Serialize)]
struct Metrics {
    events: usize,
    samples: usize,
    generation_min_ns: u128,
    generation_median_ns: u128,
    generation_max_ns: u128,
    generation_events_per_second: f64,
    serialization_min_ns: u128,
    serialization_median_ns: u128,
    serialization_max_ns: u128,
    serialization_events_per_second: f64,
    serialized_bytes: usize,
    bytes_per_event: f64,
}

fn main() {
    let mut args = std::env::args().skip(1);
    let count = args
        .next()
        .map_or(100_000, |value| value.parse().expect("event count"));
    let samples = args
        .next()
        .map_or(10, |value| value.parse().expect("sample count"));
    assert!(count > 0 && samples > 0);
    let mut generation = Vec::with_capacity(samples);
    let mut serialization = Vec::with_capacity(samples);
    let mut serialized_bytes = 0_usize;

    for _ in 0..samples {
        let started = Instant::now();
        let mut events = Vec::with_capacity(count);
        for index in 0..count {
            let event_id =
                EventId::new(u64::try_from(index).expect("index fits u64") + 1).expect("non-zero");
            let actor_raw = u64::try_from(index % 10_000).expect("index fits u64") + 1;
            let mut event = EventRecord::new(
                event_id,
                SimInstant::from_seconds(i64::try_from(index).expect("index fits i64")),
                "dummy_update",
            )
            .expect("valid event");
            event
                .add_actor(EntityId::new(actor_raw).expect("non-zero"))
                .expect("unique actor");
            event.set_significance(SignificanceScore::try_from(100).expect("bounded"));
            events.push(event);
        }
        generation.push(started.elapsed());

        let started = Instant::now();
        let mut bytes = 0_usize;
        for event in &events {
            bytes = bytes
                .checked_add(serde_json::to_vec(event).expect("serialize").len())
                .expect("byte count capacity");
        }
        serialization.push(started.elapsed());
        serialized_bytes = bytes;
        black_box(events);
    }

    generation.sort_unstable();
    serialization.sort_unstable();
    let gm = generation[samples / 2];
    let sm = serialization[samples / 2];
    let count_u32 = u32::try_from(count).expect("count fits u32");
    let metrics = Metrics {
        events: count,
        samples,
        generation_min_ns: generation[0].as_nanos(),
        generation_median_ns: gm.as_nanos(),
        generation_max_ns: generation[samples - 1].as_nanos(),
        generation_events_per_second: rate(count_u32, gm),
        serialization_min_ns: serialization[0].as_nanos(),
        serialization_median_ns: sm.as_nanos(),
        serialization_max_ns: serialization[samples - 1].as_nanos(),
        serialization_events_per_second: rate(count_u32, sm),
        serialized_bytes,
        bytes_per_event: f64::from(u32::try_from(serialized_bytes).expect("bytes fit u32"))
            / f64::from(count_u32),
    };
    println!("{}", serde_json::to_string(&metrics).expect("metrics JSON"));
}

fn rate(count: u32, elapsed: Duration) -> f64 {
    f64::from(count) / elapsed.as_secs_f64()
}
