---
name: palimpsest-godot-rust
description: Implement or debug Palimpsest's Godot client, GDExtension, Rust-to-Godot bridge, tile renderer, or runtime UI. Use when work is Godot-facing; do not use for standalone Rust-core tasks or Swift/AppKit UI development.
---

# Palimpsest Godot Rust

Preserve the direction `Rust simulation truth -> render snapshot or view model -> Godot presentation/input/rendering`. Never make a Godot Node, Scene Tree, or UI model authoritative simulation state.

For Godot scene inspection, headless validation, runtime tree, screenshots, input simulation, performance, logs, and errors, prefer the project `gda` Skill and CLI. Always use structured JSON output and inspect command help or schema when needed. Fall back to the raw Godot CLI only when gda cannot perform the operation.

Do not route routine Godot build, run, debug, or UI work through Build macOS Apps' Swift, AppKit, SwiftPM, or Xcode workflows. Use Build macOS Apps only for macOS signing, entitlements, notarization, native bundle issues, or distribution diagnostics.

Read `MASTER_SPEC.md`, `AGENTS.md` when present, and relevant bridge ADRs before changing contracts.
