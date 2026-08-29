# ADR-0013: Person Needs / Action Boundaries

- Status: Proposed — awaiting product-owner approval with the first implementing Task
- Date: 2026-08-29
- Decision owners: Product owner confirmation required for Person core-domain changes

## Context

Phase 1 must let 100 NPCs move, eat, sleep, and work (Master Spec §84) without
bearing the full Person model identity, body, personality, values, skills,
profession, family, relations, memory, knowledge, goals, beliefs, or economy.
Phase 0 built none of these systems, so Phase 1 must define a deliberately
minimal Person domain and ActivitySite model that is honest about what it does
and does not simulate.

## Decision

Define a minimal Person domain for Phase 1 and explicitly exclude every system
that belongs to a later phase.

Person core state is limited to:

- `hunger` / `fatigue`: fixed-point, bounded pressure scalars in a closed range
  that never overflow; pressure rises with elapsed simulation time and is
  reduced by the corresponding activity.
- `CurrentAction`: the currently selected action kind plus where it is executed.
- `Location`: the occupied `LocalCoord` on the single Phase 1 local tile map.
- An `EntityId` identity and a runtime ECS handle as required by ADR-0002/0011.

ActivitySite is only an affordance point, not an economy or building system. A
site advertises at most one of `Meal`, `Rest`, or `Work` affordance.

- A site has no inventory, no production chain, and no market.
- `Work` may produce only a bounded observation count (a scalar that counts work
  performed at a site) for validation metrics. It does not create resources,
  goods, tools, weapons, or any item.

Explicitly excluded from Phase 1 Person and ActivitySite scope: age, family and
kinship, personality, values, preferences, skills, profession, relations,
memory, knowledge, beliefs, goals, body/health, disease, injuries, inventory,
economy, production chains, markets, prices, trade, and organizations.

## Public Contract

- `Person` state fields for hunger/fatigue are bounded fixed-point integers; all
  update functions are checked and saturate/expose an explicit error rather than
  silently wrapping.
- `CurrentAction` and `Location` are readable and mutated through the
  simulation kernel, not by Godot (see ADR-0007 / ADR-0015).
- `ActivitySite` exposes exactly a `kind` affordance label (`Meal|Rest|Work`) and
  a `LocalCoord`; no inventory or production interface is present.
- Work-related output is a saturating `u64` observation counter exposed only to
  Developer Metrics and the DoD validation harness; it is not a game resource.

## Consequences

- Phase 1 validates moving/eating/sleeping/working mechanics and
  utility-selection depth without dragging in unbuilt systems.
- Keeping hunger/fatigue bounded avoids the population/resource explosion class
  of bugs the Chaos Simulation Test targets (Master Spec §76).
- Activities are testable in isolation, which keeps the 100-NPC/10-year
  validation deterministic and auditable.
- The minimal Person model must be extended by a later phase; Phase 2 Life
  Simulation (subject to a future ADR) will widen it. Phase 1 does
  not reserve placeholder fields for those systems.

## Rejected / Deferred Alternatives

- Include age/family/personality/skills in Phase 1 Persons: rejected; those are
  Phase 2 scope and would double the Model without Phase 1 DoD benefit.
- Give ActivitySite inventory/production semantics: rejected; it pre-builds the
  Phase 3 economy/production chain prematurely.
- Model hunger/fatigue as unbounded or floating-point: rejected; floating-point
  drift and unbounded growth undermine determinism and the chaos test.
- Implement a full body/health/disease model: rejected; Phase 5 scope and not
  required for Phase 1 DoD.
- NPC inventories and markets: deferred; Master Spec explicitly defers open
  market economy off the Phase 1 kernel.

## Supersedes / Extends

New decision; extends the Phase 1 scope defined in `AGENTS.md` and adheres to
`MASTER_SPEC.md` §84. Consistent with ADR-0002 and ADR-0011. Does not supersede
any prior ADR.
