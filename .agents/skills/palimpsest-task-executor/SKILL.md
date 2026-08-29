---
name: palimpsest-task-executor
description: Execute one explicitly identified Palimpsest task, GitHub issue, or milestone subtask within its stated scope. Use when the request is a bounded implementation task; do not use for open-ended architecture exploration or unrelated coding.
---

# Palimpsest Task Executor

Before changing files, read `MASTER_SPEC.md`, `AGENTS.md` when present, the relevant ADRs, and the task specification. Treat `MASTER_SPEC.md` as read-only and authoritative.

Extract and state the task's scope, out-of-scope items, dependencies, allowed files, tests, benchmark expectations, and definition of done. If any field is absent, infer only what is low-risk and necessary; stop for direction when the missing choice would materially change the result.

Implement only the current task. Do not automatically begin the next task, broaden scope, opportunistically refactor other modules, delete failing tests, or remove long-term architectural interfaces for short-term simplicity.

Run the specified checks in proportion to the change. Finish with a change summary, tests and benchmarks run, known limitations, and any remaining blockers.
