# Palimpsest Tooling

Audit date: 2026-08-29 (Asia/Shanghai)

This document governs the Codex Plugin and Skill development toolchain only. It does not authorize Phase 0 or product implementation. `MASTER_SPEC.md` remains read-only and authoritative.

## Governance files

| File | Status | Action |
| --- | --- | --- |
| `MASTER_SPEC.md` | Present; read in full | Read-only; unchanged |
| `AGENTS.md` | Present; read in full | Active repository instructions |
| `docs/ARCHITECTURE.md` | Present | Phase 0 architecture baseline |
| `docs/PERFORMANCE.md` | Present | Phase 0 measurement rules and results index |

The project directory was initialized as a Git repository by CHRON-015. No
remote or commit was created automatically.

## Development environment audit

| Tool | Status / version | Purpose | Source / installation |
| --- | --- | --- | --- |
| Codex CLI | 0.147.0 | Codex CLI and Plugin management | `/Users/gabrielmu/.local/bin/codex`; pre-existing, origin not identified |
| GitHub CLI | 2.96.0 | GitHub Issues, PRs, Actions, authentication | `/Users/gabrielmu/.local/bin/gh`; pre-existing |
| Git | 2.50.1 (Apple Git-155) | Version control | `/usr/bin/git`, Xcode toolchain |
| Rust | 1.98.0 (`aarch64-apple-darwin`, stable) | Simulation Core toolchain | Installed for CHRON-001 with the official rustup installer, minimal profile |
| Cargo | 1.98.0 | Rust build/test/benchmark | Installed with the stable Rust toolchain under `/Users/gabrielmu/.cargo`; current app shells may need to source `/Users/gabrielmu/.cargo/env` |
| Godot | 4.7.2 stable official, Universal arm64/x86_64 | Godot 4 macOS client and runtime | Official Standard macOS build installed for CHRON-002 at `/Users/gabrielmu/Applications/Godot.app`; SHA-512, Apple code signature, and notarization verified |
| uv | 0.11.29 | Isolated Python CLI installation | `/Users/gabrielmu/.local/bin/uv`; pre-existing |
| Python | 3.9.6 | System Python; not used for gda runtime | `/usr/bin/python3`; no upgrade attempted |
| Node.js | 24.19.0 | Plugin Eval runtime | NVM path `/Users/gabrielmu/.nvm/versions/node/v24.19.0/bin/node`; pre-existing |
| pipx | NOT FOUND | Fallback isolated Python tool installer | Not needed because uv is available |
| gda | 0.12.0 | Godot CLI automation and project Skill | PyPI wheel installed with `uv tool install gda==0.12.0`; uv provisioned isolated CPython 3.13.14 |

## Installed Plugins

Required Palimpsest plugins:

| Plugin | Version | Source | Authentication | Enabled / validation |
| --- | --- | --- | --- | --- |
| GitHub | active-session cache `0.1.10-5f7cd798dc99`; local curated manifest `0.1.6` | Official OpenAI curated marketplaces | `gh auth status`: authenticated to github.com via keyring; no PAT was created | Enabled. `github`, `gh-fix-ci`, `gh-address-comments`, and `yeet` discovered. Write confirmation policy was not broadened. |
| Codex Security | active remote release 0.1.22; local curated snapshot manifest 0.1.11 | Official `codex-security@openai-curated-remote` through the Codex app, plus official `codex-security@openai-curated` CLI registration | No separate OAuth observed | Installed and enabled; current session reload required for honest automatic routing validation. Cached 0.1.22 workflows include repository/deep scan, diff scan, threat model, finding discovery, validation, and attack-path analysis. No scan was run. |
| Plugin Eval | Plugin manifest 0.1.2; bundled local CLI reports 0.1.0 | Official `plugin-eval@openai-curated` | None required | Installed and enabled. Local script successfully analyzed all six custom Skills and initialized benchmarks; automatic Skill routing requires restart validation. |
| Build macOS Apps | 0.1.4 | Official `build-macos-apps@openai-curated` | None required | Installed and enabled; signing, entitlements, packaging, notarization, telemetry, test-triage, and bundle diagnostics discovered. |

Other pre-existing enabled plugins reported by `codex plugin list` were not installed or changed by this task: `documents` 26.826.12353, `pdf` 26.826.12353, `spreadsheets` 26.826.12353, `presentations` 26.826.12353, `template-creator` 26.826.12353, `codex-app-tools` 0.1.3, `sites` 0.1.46, `browser` 26.825.31414, `chrome` 26.825.31414, `computer-use` 1.0.1000901, `latex` 0.2.6, `visualize` 1.0.23, plus curated `canva`, `figma`, and `game-studio` snapshot `bd2122cb`. Their presence predates this task; the instruction not to install Figma or Game Studio was honored, and they were not removed because removal was not requested.

## Discovered Skills

System/user Skills discovered before this task:

`imagegen`, `openai-docs`, `plugin-creator`, `review-agent`, `skill-creator`, `skill-installer`, `final-exam-review`, `headroom-project`, `markitdown-convert`, `pdf`, `playwright`, `read-long-pdfs`, and `taste-skill`.

Relevant official Plugin Skills discovered:

- GitHub: `github`, `gh-fix-ci`, `gh-address-comments`, `yeet`.
- Codex Security cache: `security-scan`, `deep-security-scan`, `security-diff-scan`, `threat-model`, `finding-discovery`, `validation`, `attack-path-analysis`, `define-security-policy`, `fix-finding`, `propose-security-hardening`, `track-findings`, `triage-finding`, `verify-fix`, and `vulnerability-writeup`.
- Plugin Eval: `plugin-eval`, `evaluate-skill`, `evaluate-plugin`, `improve-skill`, and `metric-pack-designer`.
- Build macOS Apps: `signing-entitlements`, `packaging-notarization`, `telemetry`, `test-triage`, `build-run-debug`, `appkit-interop`, `swiftpm-macos`, `swiftui-patterns`, `liquid-glass`, `view-refactor`, and `window-management`.

Project Skills installed or created by this task:

- Third party: `gda` 0.12.0.
- Palimpsest custom: `palimpsest-task-executor`, `palimpsest-architecture-guard`, `palimpsest-rust-sim`, `palimpsest-performance-gate`, `palimpsest-godot-rust`, and `palimpsest-sim-debug`.

`skill-creator` and `skill-installer` were already present; neither was reinstalled. `plugin-creator` was not used because no custom plugin was created.

## gda third-party record

| Field | Value |
| --- | --- |
| Package | `gda==0.12.0` |
| Source | [aigengame/godot-agent](https://github.com/aigengame/godot-agent) and [PyPI gda](https://pypi.org/project/gda/) |
| Release status | Pre-1.0; latest verified release `v0.12.0` published 2026-08-27 |
| Maintenance | Repository public, not archived, pushed 2026-08-28; recent commits observed |
| License | MIT in both GitHub metadata and PyPI license expression |
| Package author / upstream | PyPI author `haihong.qin`; package project URLs point to `aigengame/godot-agent`; source/license alignment accepted |
| Python requirement | Python >=3.13; uv isolated runtime is CPython 3.13.14 |
| Dependencies | `pydantic>=2.13.4`, `typer>=0.26.7`; `mcp>=2,<3` is optional only |
| Godot requirement | Headless >=4.4; live daemon >=4.6 on macOS/Linux |
| Installation | `uv tool install gda==0.12.0` |
| Project Skill | `.agents/skills/gda/SKILL.md`, installed by gda's own provider/scope mechanism |
| Upgrade policy | `MANUAL REVIEW`; no automatic upgrade |
| MCP | Disabled and unregistered. The wheel exposes a `gda-mcp` executable, but no MCP dependency extra, configuration, or server registration was enabled. |

Capability and risk boundary: gda can read, create, edit, or delete Godot project artifacts, run project code through Godot, export builds, and—when explicitly started—install a development harness for runtime tree inspection, input simulation, screenshots, performance, and logs. These are powerful local capabilities. Use the CLI with structured JSON, review mutations, keep its version pinned, and do not start the live daemon or install its harness without a task that requires it. Godot 4.7.2 is installed and engine operations are available.

## Skill evaluation

Plugin Eval static analysis after one trigger-description refinement:

| Skill | Score | Grade | Risk | Warnings | Static token budget |
| --- | ---: | --- | --- | ---: | --- |
| `palimpsest-task-executor` | 100 | A | low | 0 | good |
| `palimpsest-architecture-guard` | 100 | A | low | 0 | good |
| `palimpsest-rust-sim` | 100 | A | low | 0 | good |
| `palimpsest-performance-gate` | 100 | A | low | 0 | good |
| `palimpsest-godot-rust` | 100 | A | low | 0 | good |
| `palimpsest-sim-debug` | 100 | A | low | 0 | good |

The figures are static estimates, not observed production token usage. Benchmark configurations were initialized for `palimpsest-task-executor` and `palimpsest-architecture-guard`. Together they cover bounded task execution, rejected scope expansion, Master Spec conflict and Change Proposal handling, Godot UI routing away from Rust-only/Swift workflows, and measurement-first performance work. Real benchmark runs are deferred until after restart so project Skill discovery is exercised honestly.

## Skill routing

- Start a normal, explicit Palimpsest task with `palimpsest-task-executor`.
- Add `palimpsest-rust-sim` only for Rust Simulation Core work.
- Add `palimpsest-architecture-guard` only for architectural or cross-module contract changes.
- Add `palimpsest-performance-gate` only for performance, scale, memory, LOD, or benchmark claims.
- Add `palimpsest-godot-rust` only for Godot client or Rust/Godot bridge work.
- Use `palimpsest-sim-debug` for long-running simulation anomalies and first-divergence analysis.

Do not load every Palimpsest Skill for ordinary coding.

## Tool Routing Matrix

| Task Type | Preferred Tool / Skill | Fallback | Do Not Use |
| --- | --- | --- | --- |
| GitHub work | GitHub Plugin (`github`, `gh-fix-ci`, `gh-address-comments`, `yeet`) + `gh` | Read-only GitHub web UI | PAT workarounds; unconditional Full Access |
| Security scan | Codex Security workflow matching repository/diff/threat-model scope | Focused manual review | Run an unrequested whole-repository scan |
| Rust simulation | `palimpsest-task-executor` + `palimpsest-rust-sim` | Rust/Cargo CLI after toolchain installation | Generic third-party “Rust expert” prompt packs |
| Godot edit | `palimpsest-godot-rust` + gda CLI/Skill | Raw Godot CLI | SwiftUI/AppKit workflows |
| Godot runtime debug | gda CLI/Skill | Raw Godot CLI/logs | Build macOS Apps `build-run-debug`; Swift/Xcode app workflow |
| macOS signing | Build macOS Apps `signing-entitlements` | Apple `codesign` diagnostics | gda for signing policy |
| macOS notarization | Build macOS Apps `packaging-notarization` | Apple notarization CLI diagnostics | Godot scene/runtime tooling |
| Skill evaluation | Plugin Eval | `skill-creator` quick validation and manual review | Invented evaluation results |
| Performance benchmark | `palimpsest-performance-gate` + actual benchmark tools | Platform tools with documented method | Intuition-only optimization |
| Long simulation debugging | `palimpsest-sim-debug` + headless runner/metrics | Focused logs and state dumps | Guessing from the final-year outcome |

Build macOS Apps is not the daily Palimpsest client workflow. Palimpsest uses Godot 4, not a SwiftUI/AppKit application. Route only signing, entitlements, notarization, native bundle, macOS runtime/log, and distribution diagnostics to that plugin. Routine Godot build/run/debug belongs to gda or the raw Godot CLI.

## Deferred Tools

No deferred tool below was installed by this task.

| Tool | Intended phase / reason |
| --- | --- |
| Superpowers | Deferred: conflicts may arise with Palimpsest's Master Spec, AGENTS, and task governance |
| Game Studio | Deferred: Phase 0 does not need its web/browser-game workflow; consider only for later visual asset evaluation |
| Linear | Deferred indefinitely while GitHub Issues is the sole task system |
| Figma | UI/UX Design phase |
| Sentry | Alpha runtime, after real runtime telemetry exists |
| Hugging Face | Local LLM phase |
| Build Web Apps | Web phase |
| Vercel | Web deployment phase |
| CircleCI | Not planned; GitHub Actions is the only CI |
| CodeRabbit | Deferred; avoid a second review-governance layer at this stage |
| Neon Postgres | Deferred; architecture specifies SQLite + snapshots |
| OpenAI Developers | Not needed for this tooling scope; LLM is optional and no OpenAI API integration is being built |
| Generic Rust/ECS/performance expert Skills | Not installed; project-specific Skills plus real tests and benchmarks are preferred |
| `gda-mcp` | Deferred pending evidence that CLI + Skill is insufficient; reduces MCP attack surface |

The pre-existing global Figma and Game Studio plugins noted above were not installed during this task. Their future use remains deferred for Palimpsest even though they are available globally.

## Blocked and follow-up state

- `POST_RESTART_VALIDATION_REQUIRED`: confirm Codex Security and Plugin Eval Skill routing, confirm all project Skills are discovered, and run the two real benchmark configurations.
- GitHub OAuth is not blocked: `gh` is currently authenticated. If the GitHub Plugin connector later requests separate OAuth, complete the official connection flow; do not create a PAT workaround.
- Godot operations are unblocked: gda resolves the installed Godot 4.7.2 binary at `/Users/gabrielmu/Applications/Godot.app/Contents/MacOS/Godot`.
- Rust development is unblocked for CHRON-001 and later Phase 0 tasks: stable Rust 1.98.0, Cargo, rustfmt, and Clippy are installed.
- Git change reporting is available. Hosted GitHub Actions execution remains
  pending until the product owner selects/pushes a remote repository.

See `docs/PLUGIN_SKILL_POST_RESTART.md` for the exact safe follow-up.
