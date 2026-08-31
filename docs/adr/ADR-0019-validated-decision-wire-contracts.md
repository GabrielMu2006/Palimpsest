# ADR-0019: Validated Decision Wire Contracts

- Status: Accepted — product owner confirmed the four follow-up items on 2026-08-30
- Date: 2026-08-30
- Decision owners: Product owner
- Task: REM-006 in `docs/reports/PHASE_1_REVIEW_REMEDIATION_PLAN_V1.md`
- Extends: ADR-0013 and ADR-0014

## Context

`ActionCandidate::new` currently accepts any kind/target combination, while
derived deserialization bypasses `PerturbationSpec::new` range checks. Selection
accepts duplicate enumeration keys and duplicate candidates. The perturbation
implementation silently clamps malformed wire input, so represented settings
can differ from executed settings and a selected trace key can be ambiguous.

These are public construction/serialization boundaries. This ADR proposes their
exact correction before implementation, not a general-purpose validation
framework or a durable save schema.

## Scope / Out of Scope

Validate candidate shape, candidate-set identity, perturbation settings, and
selected-key correspondence within `sim-ai`. Preserve valid diagnostic JSON
keys and deterministic scoring. Do not change default weights, action kinds,
factor kinds, selection precedence, Needs, Event Store, snapshots, Godot, or
action execution. Do not deserialize diagnostics back into simulation truth.

World-dependent feasibility remains a separate contract: a structurally valid
target does not prove a matching reachable site still exists. Normal callers
must use `candidate_actions` with the same context; a future executor must
recheck preconditions. This task must not add an implicit pathfinding/gating
policy to the selector under the guise of wire validation.

## Decision

### 1. Individual candidates

- `Idle` has `target: None` exactly; Move/Eat/Sleep/Work require `Some(LocalCoord)`.
- `LocalCoord` keeps its existing validated 128-by-128 bounds and wire shape.
- An individual `order: u64` may be any value, including `u64::MAX`: it is a
  label whose membership validity depends on the collection, not a persistent
  entity identity. No global candidate-order allocator is introduced.
- Change public `ActionCandidate::new(kind, target, order)` to return
  `Result<ActionCandidate, CandidateError>`. `CandidateError` distinguishes
  `IdleHasTarget` from `MissingTarget { kind }` and implements Display/Error.
- Deserialize through the same validation using a private wire helper. Keep
  serialization fields `kind`, `target`, `order` and existing enum names.
- No public unchecked constructor, mutable fields, silent target repair,
  skipped candidate, or fallback Idle. The deterministic provider may assemble
  its own proven-valid fields internally; it retains its current return type.

### 2. Complete candidate sets at selection

Keep `select_action`'s `Result<Selection, DecisionError>` signature and add a
typed invalid-set error variant. Before scoring a selection input, require:

1. Non-empty; preserve `DecisionError::EmptyCandidates` for the empty input.
2. Unique `order` keys whose **set** is exactly `0..n-1`.
3. No duplicate `(kind, target)` pair, even with different order keys.

Do **not** require vector position to equal `order`. A permutation of a valid
set remains valid and must preserve the winner and tie reason. `all_scores`
and trace candidates retain the input order; byte equality of the entire
trace is promised for identical ordered inputs, not different permutations.
Do not sort, renumber, deduplicate, or synthesize a candidate on the caller's
behalf. Selected keys refer to exactly one candidate.

Use a shared internal validator with a typed `CandidateSetError` distinguishing
duplicate order, order outside `0..n-1`, and duplicate kind/target. On an input
with multiple defects, report the first duplicate key in input order, then the
first out-of-range key, then the first duplicate kind/target. Checking unique
keys plus the range proves contiguous coverage without an enormous scan up to
an attacker-supplied `u64::MAX`. Do not iterate to the largest supplied key.
The existing small provider bound remains unchanged.

`score_candidates` remains an independent diagnostic scoring operation returning
`Vec<CandidateScore>`: it can score a subset or a single candidate with an
arbitrary order. It does not select, produce a populated selected key, or
certify that its input is a complete decision set. Do not change its API merely
to impose complete-set validation on this distinct operation.

### 3. Perturbation

- Permitted effective range: `Zero`, or `Bounded(epsilon)` with `0..=100`.
  Keep `MAX_EPSILON = 100` and the current deterministic mixer.
- Change `PerturbationSpec::new(seed, range)` to return
  `Result<PerturbationSpec, PerturbationError>`, with typed
  `EpsilonOutOfRange { epsilon }`. Defaults and ZERO remain valid/infallible.
- The public `PerturbationRange` enum is a raw request value, not independently
  a validated execution configuration: native code can spell `Bounded(-1)`,
  but cannot create a `PerturbationSpec` from it. Document this distinction.
  Deserialization of the range itself must also reject invalid epsilon.
- `PerturbationSpec` deserialization uses the same check, including when nested
  in other input types. Its private fields have no bypassing public setter.
- Reject negative epsilon, 101, `i64::MIN`, and `i64::MAX`. Serde must also reject
  numeric overflow and non-integer values; never round or saturate input.
- Remove execution-time `.clamp(0, MAX_EPSILON)`: validated settings are used
  verbatim. Keep both `Zero` and `Bounded(0)` as valid, wire-distinct forms with
  exactly zero numerical perturbation; do not silently canonicalize either.
- Keep the exact wire keys `seed`, `range`, `Zero`, and `Bounded`.

### 4. Trace identity versus diagnostic fragments

The existing `trace_for` can return one unselected candidate whose order is,
for example, 6. Preserve this partial-inspection use case; it is not an invalid
one-element complete selection set.

- `DecisionTrace::new` becomes fallible with a typed `TraceValidationError`;
  unselected fragments may be empty or have non-contiguous keys, but may not
  duplicate order or kind/target. They keep `selected` and `tie_break` unset.
- `trace_for` retains its return type by constructing the known-valid single
  fragment internally. No public invalid input path may panic.
- A trace with `selected: Some(key)` must contain a complete, non-empty valid
  selection set, that key exactly once, and a non-null `tie_break`. With
  `selected: None`, `tie_break` must also be None. Enforce these rules on
  deserialization as well as native construction.
- `DecisionTrace::decided` remains crate-internal and is fed only validated
  selector results. Validate identity before emitting a Selection; private
  construction must not become a second public unchecked path.
- `Selection` deserialization must also validate its `all_scores` set and
  match its chosen candidate/key to `trace.selected` and its trace candidates.
  The per-key candidate, total score, and nested candidate trace must agree
  between the duplicated output fields. Unknown selected keys, duplicate keys,
  or conflicting copies are errors, not repaired output.

This is structural/correspondence validation, not proof that imported scores
are historically true. No context or weights accompany every diagnostic
fragment, so this task must not claim to recompute world truth from JSON.
Factor input range/schema expansion and cryptographic trace authenticity are
out of scope. Selection/scoring against a live context remains the authority;
imported traces remain read-only diagnostic data, never execution commands.

### 5. Error and compatibility policy

Native paths return typed errors; serde paths translate the same failures to
`serde::de::Error::custom`. Do not couple tests to the full prose of a serde
error, although it must identify the violated rule. No invalid input is
accepted by clamping, reordering, dropping, retrying with defaults, or panicking.

The three public constructor changes (ActionCandidate, PerturbationSpec,
DecisionTrace) are intentional source-level API changes recorded by this ADR.
Update all directly affected callers/tests/examples inside `sim-ai`; the
parent must recheck external callers before dispatch. If additional production
callers outside REM-007's whitelist exist then, stop for an explicit task-scope
extension rather than changing them opportunistically.

Valid provider output and valid existing serialized diagnostics preserve field
names, numeric representations, and enum names, including valid partial traces.
Previously accepted malformed values are intentionally rejected. Phase 1
diagnostics are not durable save records; no database/snapshot migration or
new versioned persistence format is introduced.

## Test Vectors / REM-007 Acceptance

Valid individual JSON (order is validated relative to a set only at selection):

```json
{"kind":"Idle","target":null,"order":0}
{"kind":"Eat","target":{"x":3,"y":4},"order":2}
{"seed":42,"range":"Zero"}
{"seed":42,"range":{"Bounded":0}}
{"seed":42,"range":{"Bounded":100}}
```

Each line is a separate JSON document. Invalid counterparts include:

```json
{"kind":"Idle","target":{"x":3,"y":4},"order":0}
{"kind":"Work","target":null,"order":0}
{"kind":"Eat","target":{"x":128,"y":4},"order":0}
{"seed":42,"range":{"Bounded":-1}}
{"seed":42,"range":{"Bounded":101}}
```

Required tests:

- All five action kinds × target absent/present, with individual order values
  0, 1, `u64::MAX`, exercised through native and serde paths. Coordinate
  validity continues to be delegated to the already-tested LocalCoord type.
- Complete sets: empty -> EmptyCandidates; unique keys [0,1] and [1,0] ->
  valid with invariant winner; [0,0] -> duplicate; [0,2], [1], and
  [0,u64::MAX] -> non-contiguous/out-of-range without a huge loop.
- Duplicate `(kind,target)` at distinct otherwise-valid keys -> typed error.
- Epsilon -1/0/100/101 and integer extremes; JSON fractional and overflowing
  values rejected. Native and serde rejection must agree.
- Unselected singleton trace with order 6 still round-trips. Duplicate keys
  in a fragment are rejected by both native constructor and serde.
- Build a valid Selection with the real provider/selector, then separately
  corrupt selected key, duplicate a trace key, remove tie reason, or mismatch
  selected/all-scores/candidate-trace copies; each invalid decode must fail.
- Valid provider output and complete Selection round-trip; native versus
  decoded inputs select identically with zero and bounded seeded perturbation.
- Preserve all existing deterministic, trace, tie, and saturating-arithmetic
  tests. Adapt callers to Result explicitly; do not replace a previously
  asserted failure boundary with an unchecked unwrap or delete its assertion.

Run workspace fmt, Clippy, tests and docs, then the existing candidate/selection
smoke benchmark. Measure validation overhead against the pre-change baseline;
do not assume added checks are free. Full M5 evidence is REM-008.

## Alternatives / Consequences

- Keep derived Deserialize and scorer clamping: rejected; represented and
  executed settings can differ, and invalid identities remain ambiguous.
- Validate only the provider: rejected; public constructors and JSON bypass it.
- Require every diagnostic subset to be numbered from zero: rejected; breaks
  `trace_for` and subset scoring without improving complete-selection safety.
- Introduce a public CandidateSet framework, custom schema registry, or
  persistence migration now: deferred; a shared internal validator and typed
  errors are sufficient for the current small API.
- Remove deserialization from every diagnostic type: deferred; changes useful
  round-trip behavior more broadly than these targeted checks require.

## Task Completion / Acceptance Gate

- Dependencies: accepted ADR-0013/0014 and existing CHRON-025/026 contracts.
- Files modified by REM-006: this ADR only; references in CHRON-025/026 wait
  for acceptance and an authorized task whose whitelist permits them.
- Tests for this ADR-only task: source/API/serde call-site review and specified
  vectors; no implementation or successful runtime regression claim.
- Benchmark: N/A — decision record only.
- DoD: native/wire boundaries, partial versus complete sets, errors,
  compatibility, test cases, and non-goals are explicit. Accepted 2026-08-30.

Approval record: the product owner confirmed the four follow-up items,
including explicit rejection, the three fallible constructors, and trace
identity checks, and clarified whole-plan execution approval. REM-007 is
authorized without another dispatch/ADR confirmation. Tests and independent
review remain required; this does not authorize action execution or persistence.
