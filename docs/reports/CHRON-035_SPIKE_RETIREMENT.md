# CHRON-035 — Spike workload retirement

Status: planned removal map; executable removal waits for CHRON-034 hosted checks.

| Current path | Planned replacement / preserved coverage |
|---|---|
| Core spike_workload.rs and public run/type reexports | Remove production dummy; WorldKernel is the authoritative real path |
| headless library run alias and default CLI | Real reachable100-person fixture; actual advance/metrics/snapshot; reject negative time |
| bench_mode_workload binary | Remove; CHRON033 direct/worker/rendered same-work comparison |
| Godot benchmark_spike_workload | Remove only this method/import; keep init/render proof and PalimpsestMicroWorld |
| CI dummy default/mode smokes | Real runner and representative100-person benchmark/corpus smoke; primitive storage/event/scheduler diagnostics remain |
| Two old spike tests | Preserve finite scheduler/event validation/count/drain assertions in a private test, plus real runner final-time/population/future-queue and negative-time tests |

The unavailable retired Core exports will be covered by compile-fail doctests.
No intentionally uncompilable normal workspace test is added. No production
spike implementation is retained under a new name. ADR0010's historical rationale
stays intact with a retired status and explicit replacement reference.

Historical reports are retained against the pre-turn worktree baseline:

- ARCHITECTURE_SPIKE_V1.md: fba1a570501cf094fa8efe2458c01c8e6bfeffbb4dc3735fe2d941d059fce414
- CHRON-008_10K_DUMMY_BENCHMARK.md: fbf45ad40d648e0e91864e2486fd2660e55f0394be09fbf78abbf887197543fe
- CHRON-017_HEADLESS_RENDERED.md: 9106d310ca7a2ad0b9a89219ac73a1b4b9e6feb3aed7252b5e4a805b757ffeea

The baseline already included owner-approved public-repository/protection text
changes in the spike report; retirement must not undo them or confuse old HEAD
with this task's baseline. Master Spec hash remains its immutable recorded value.

No new performance workload is needed. Phase0's roughly2.09 headless/rendered
ratio is historical and not comparable to the asynchronous representative kernel.
Use CHRON033's measured same-work ratio instead. The3/5/7GB caps remain unchanged.
