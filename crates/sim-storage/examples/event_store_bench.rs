use std::time::Instant;

use palimpsest_sim_events::{EventId, EventRecord};
use palimpsest_sim_storage::EventStore;
use palimpsest_sim_time::SimInstant;
use serde::Serialize;

#[derive(Serialize)]
struct Metrics {
    events: usize,
    batch_size: usize,
    append_ns: u128,
    append_events_per_second: f64,
    checkpoint_ns: u128,
    database_bytes: u64,
    bytes_per_event: f64,
}

fn main() {
    let mut args = std::env::args().skip(1);
    let count = args
        .next()
        .map_or(100_000, |value| value.parse().expect("event count"));
    let batch_size = args
        .next()
        .map_or(1_000, |value| value.parse().expect("batch size"));
    assert!(count > 0 && batch_size > 0);
    let events: Vec<_> = (0..count).map(make_event).collect();
    let directory = tempfile::tempdir().expect("temporary directory");
    let path = directory.path().join("world.db");
    let mut store = EventStore::open(&path).expect("open store");

    let started = Instant::now();
    for batch in events.chunks(batch_size) {
        store.append_batch(batch).expect("append batch");
    }
    let append = started.elapsed();
    assert_eq!(
        store.event_count().expect("count"),
        u64::try_from(count).expect("fits u64")
    );

    let started = Instant::now();
    store.checkpoint().expect("checkpoint");
    let checkpoint = started.elapsed();
    store.integrity_check().expect("integrity");
    let database_bytes = std::fs::metadata(&path).expect("database metadata").len();
    let count_u32 = u32::try_from(count).expect("count fits u32");
    let database_bytes_u32 = u32::try_from(database_bytes).expect("database size fits u32");
    let metrics = Metrics {
        events: count,
        batch_size,
        append_ns: append.as_nanos(),
        append_events_per_second: f64::from(count_u32) / append.as_secs_f64(),
        checkpoint_ns: checkpoint.as_nanos(),
        database_bytes,
        bytes_per_event: f64::from(database_bytes_u32) / f64::from(count_u32),
    };
    println!("{}", serde_json::to_string(&metrics).expect("metrics JSON"));
}

fn make_event(index: usize) -> EventRecord {
    let raw = u64::try_from(index).expect("index fits u64") + 1;
    EventRecord::new(
        EventId::new(raw).expect("non-zero"),
        SimInstant::from_seconds(i64::try_from(index).expect("fits i64")),
        "dummy_update",
    )
    .expect("valid event")
}
