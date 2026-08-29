# ADR-0008: SQLite Event Store Prototype

- Status: Accepted for Architecture Spike
- Date: 2026-08-29

## Context
Structured events need local durability, indexed history queries, and safe checkpointing on macOS.

## Decision
Use bundled SQLite through rusqlite, WAL journal mode, synchronous NORMAL, foreign keys, atomic batch append, an indexed event envelope table, and a causal-edge table. Store the complete validated versioned JSON payload plus indexed ID/time/type columns. SQLite INTEGER limits persisted EventId to `i64::MAX`; larger IDs fail explicitly.

## Consequences
The prototype is local, transactional, queryable, and checkpointable. JSON duplicates indexed fields but preserves the full versioned envelope. Final normalized archive schemas and retention remain future decisions.

## Alternatives Considered
- Flat append files: rejected for indexed queries and transactional consistency.
- PostgreSQL: rejected by the single-player local architecture.
- Copy a live WAL database: rejected; checkpointing is required.
