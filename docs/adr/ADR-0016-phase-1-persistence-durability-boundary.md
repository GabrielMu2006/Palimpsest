# ADR-0016: Phase 1 Persistence Durability Boundary

- Status: Accepted by product-owner Phase 0 decision (2026-08-29, decisions 4 and 5)
- Date: 2026-08-29
- Decision owners: Product owner confirmation required for persistence/durability-boundary changes

## Context

The product owner accepted SQLite WAL + `NORMAL` for routine batched events
(decision 4) and decided to intentionally invalidate Phase 0 snapshot artifacts
in favor of a hardened, versioned Snapshot V1 (decision 5). Phase 1 is a kernel
validation (World Grid, Terrain, Person, Needs, Utility AI, 100 NPCs, 10 years)
and must not slip into the phase-6 archive/production-save system. This ADR
records the durability boundary without pretending production persistence exists.

## Decision

- Routine batched structured events use SQLite WAL with `synchronous=NORMAL`.
  Stronger durability (checkpointing, or `synchronous=FULL`/explicit
  checkpoint boundaries) is reserved for explicit durability boundaries, not the
  default event path.
- Phase 0 snapshot artifacts are explicitly **not compatible** with any future
  format; they are not loadable and are treated as invalidated prototypes.
- A future Snapshot V1 must, before being treated as a supported save, add:
  schema `version`, a content-type tag, a `checksum`, a decoded/expanded-size cap,
  and atomic replace semantics. None of these is implemented or guaranteed in
  Phase 1.
- Phase 1 does **not** implement production persistence, full archive, `.world`
  packaging, replay, or the Phase 6 archive/history systems. Those remain out of
  Phase 1 scope.

## Public Contract

- The simulation kernel runs and validates over 10 years without depending on a
  round-trip persistence save. Persistence is optional to Phase 1 DoD.
- Any event/appender exposed in Phase 1 uses WAL `NORMAL` for routine batched
  work and documents the durability setting used.
- Snapshot artifacts produced by Phase 0 are not readable as saves; no API
  accepts them.
- No public Phase 1 API claims atomic replace, checksum validation, size caps, or
  content-version compatibility for snapshots; those are explicitly future
  (Snapshot V1) contracts.

## Consequences

- Phase 1 stays focused on kernel correctness and the 10-year DoD without
  front-running archive/save infrastructure.
- Routine event throughput keeps the measured fast path; crash-loss policy for
  the NORMAL setting is explicitly a documented product limitation, not a runtime
  guarantee (final-report risk 3).
- Not implementing Snapshot V1 now avoids pretending prototype artifacts are
  durable saves and avoids building archive machinery before its phase.
- Any future real persistence change is governed by new ADR / proposal work and
  would widen this contract.

## Rejected / Deferred Alternatives

- Rely on Phase 0 snapshot artifacts as saves: rejected; product-owner decision
  5 invalidates them for compatibility.
- Implement full production persistence / `.world` in Phase 1: rejected; it is
  out of Phase 1 scope and belongs to the later archive/history phase.
- Use `synchronous=FULL` for all writes: rejected for Phase 1 routine events; it
  trades the measured throughput for durability the kernel validation does not
  need, and stronger durability stays at explicit boundaries.
- Treat replay as re-simulation from seed: rejected per Master Spec §38; replay
  uses snapshot + delta, which is out of Phase 1.

## Supersedes / Extends

Resolves ADR-0009's "Phase 0 snapshot format" compatibility doubt and extends
ADR-0008 (SQLite event store) with the Phase 1 durability setting. Supersedes
any implication that Phase 0 snapshot artifacts are durable saves, without
claiming support for a Snapshot V1 yet.
