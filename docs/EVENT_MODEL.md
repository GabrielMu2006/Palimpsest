# Phase 0 Structured Event Model

Event truth is versioned structured data, not generated prose. Version 1 contains stable event/entity IDs, integer simulation time, a type key, actor/target/location references, causal links, visibility, bounded significance, and ordered JSON metadata.

Deserialization validates the envelope. Runtime ECS handles and Godot nodes are forbidden. Claims, beliefs, documents, NLG text, and historiography remain separate future models. See ADR-0006.

## Phase 0 Payload Baseline

The representative migration event used by the round-trip test serializes to 225 bytes of compact JSON on Rust 1.98.0 with `serde_json` 1.0.151. This is a payload-size baseline, not an Event Store throughput result; CHRON-016 will measure event generation and serialization throughput separately.
