# Plugin and Skill post-restart validation

Status: `POST_RESTART_VALIDATION_REQUIRED`

After restarting Codex in the Palimpsest root:

1. Confirm the required Plugins are enabled and their Skills are discoverable:
   - GitHub: `github`, `gh-fix-ci`, `gh-address-comments`, `yeet`.
   - Codex Security: `security-scan`, `security-diff-scan`, `threat-model`, `finding-discovery`, `validation`, `attack-path-analysis`.
   - Plugin Eval: `plugin-eval`, `evaluate-skill`, `evaluate-plugin` and benchmark/token-budget routing.
   - Build macOS Apps: `signing-entitlements` and `packaging-notarization` at minimum.
2. Confirm project Skills are discovered: `gda` plus all six `palimpsest-*` Skills.
3. Confirm routing without implementing product code:
   - A Godot runtime-debug request prefers gda, not Swift/Xcode `build-run-debug`.
   - A performance request invokes measurement-first guidance.
   - A Master Spec conflict creates a Change Proposal rather than editing `MASTER_SPEC.md`.
4. Run the two Plugin Eval benchmark configurations in their isolated-copy mode:
   - `.agents/skills/palimpsest-task-executor/.plugin-eval/benchmark.json`
   - `.agents/skills/palimpsest-architecture-guard/.plugin-eval/benchmark.json`
5. Review isolated benchmark outputs. Do not copy product-code changes back into Palimpsest. Record scores or failures here only after genuine runs.

Do not run a Codex Security repository scan in this validation. Do not install or enable `gda-mcp`. Do not enter Phase 0. Do not modify `MASTER_SPEC.md`.

Shortest follow-up prompt:

> 在 Palimpsest 根目录读取 `docs/PLUGIN_SKILL_POST_RESTART.md`，执行全部 `POST_RESTART_VALIDATION_REQUIRED`；只验证 Plugins、Skills 和 gda，不扫描仓库、不进入 Phase 0、不修改 `MASTER_SPEC.md`。
