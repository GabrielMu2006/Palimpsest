use std::time::{Duration, Instant};

use palimpsest_sim_entity::EntityIdAllocator;
use palimpsest_sim_storage::{EntitySnapshot, PendingWorkSnapshot, Snapshot, SnapshotCodec};
use palimpsest_sim_time::{SimClock, SimInstant};
use serde::Serialize;

#[derive(Serialize)]
struct Metrics {
    entities: usize,
    pending_work: usize,
    samples: usize,
    raw_bytes: usize,
    compressed_bytes: usize,
    compression_ratio: f64,
    encode_min_ns: u128,
    encode_median_ns: u128,
    encode_max_ns: u128,
    decode_min_ns: u128,
    decode_median_ns: u128,
    decode_max_ns: u128,
}

fn main() {
    let count = std::env::args()
        .nth(1)
        .map_or(10_000, |value| value.parse().expect("count"));
    let samples = std::env::args()
        .nth(2)
        .map_or(10, |value| value.parse().expect("samples"));
    let mut allocator = EntityIdAllocator::default();
    let mut entities = Vec::with_capacity(count);
    let mut pending = Vec::with_capacity(count);
    for index in 0..count {
        let id = allocator.allocate().expect("ID capacity");
        entities.push(EntitySnapshot {
            entity_id: id,
            state: u64::try_from(index).expect("fits"),
        });
        pending.push(PendingWorkSnapshot {
            entity_id: id,
            due: SimInstant::from_seconds(i64::try_from(index).expect("fits")),
            work_type: "dummy_update".to_owned(),
        });
    }
    let snapshot = Snapshot::new(
        SimClock::at(SimInstant::from_seconds(10_000)),
        allocator,
        entities,
        pending,
    )
    .expect("valid snapshot");
    let raw =
        bincode::serde::encode_to_vec(&snapshot, bincode::config::standard()).expect("raw encode");
    let mut encodes = Vec::with_capacity(samples);
    let mut decodes = Vec::with_capacity(samples);
    let mut compressed = Vec::new();
    for _ in 0..samples {
        let started = Instant::now();
        compressed = SnapshotCodec::encode(&snapshot).expect("encode");
        encodes.push(started.elapsed());
        let started = Instant::now();
        let restored = SnapshotCodec::decode(&compressed).expect("decode");
        decodes.push(started.elapsed());
        assert_eq!(restored, snapshot);
    }
    encodes.sort_unstable();
    decodes.sort_unstable();
    let metrics = Metrics {
        entities: count,
        pending_work: count,
        samples,
        raw_bytes: raw.len(),
        compressed_bytes: compressed.len(),
        compression_ratio: ratio(compressed.len(), raw.len()),
        encode_min_ns: encodes[0].as_nanos(),
        encode_median_ns: median(&encodes).as_nanos(),
        encode_max_ns: encodes[samples - 1].as_nanos(),
        decode_min_ns: decodes[0].as_nanos(),
        decode_median_ns: median(&decodes).as_nanos(),
        decode_max_ns: decodes[samples - 1].as_nanos(),
    };
    println!("{}", serde_json::to_string(&metrics).expect("metrics JSON"));
}

fn median(values: &[Duration]) -> Duration {
    values[values.len() / 2]
}
fn ratio(compressed: usize, raw: usize) -> f64 {
    f64::from(u32::try_from(compressed).expect("fits"))
        / f64::from(u32::try_from(raw).expect("fits"))
}
