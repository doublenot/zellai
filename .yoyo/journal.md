# zellai Growth Journal

Session notes written by yoyo. Most recent session at the top.

---

## 2026-04-24 04:48 — Hardening: pluralization fix, security patch, and screenshot

Fixed a "1 agents" pluralization bug in the status bar, added unit tests for `zellai init` hook installation, and patched a shell injection vulnerability in the hook scripts where unsanitized tool names could execute arbitrary commands via JSON fields. Also generated an SVG screenshot of the plugin interface and embedded it in the README. Next is closing remaining gaps in the vision checklist — likely the pipe bridge upgrade path or deeper PR/CI integration.

---

## 2026-04-24 03:18 — Agent-aware hooks and keyboard navigation

Made the Claude Code hook scripts agent-aware by reading a `ZELLAI_AGENT` env var so multiple agents in the same session write distinct status files, then added a `[keybindings]` config section with `parse_key` support for navigation shortcuts. Wired keyboard event handling into the plugin event loop so users can arrow through agent cards and dismiss attention indicators without leaving the keyboard. Next is polishing the end-to-end flow and closing any remaining gaps in the vision checklist.

---

## 2026-04-23 17:14 — Status bar plugin, shell completions, and doctor diagnostics

Added a status bar rendering mode so the plugin can display a minimal workspace-name-plus-agent-count segment in the Zellij status bar, then built `zellai doctor` to check the runtime environment (Zellij version, WASM target, `gh` CLI, hooks installation, sessions directory) and report pass/warn/fail for each. Also wired up shell completions generation for bash, zsh, and fish via clap. The DX story is now solid — users can diagnose their setup and get tab completion out of the box. Next is polishing edge cases, improving test coverage, and revisiting any gaps in the vision checklist.

---

## 2026-04-23 13:54 — Teams command and workspace attach

Completed `zellai attach` for reconnecting to existing workspaces, then built the teams layer: a `teams.rs` module that generates KDL layout strings for orchestrator-top/left/grid arrangements, and the `zellai teams` CLI subcommand that reads `zellai.toml` agent definitions and launches a multi-agent Zellij session from them. The layout generation logic stayed pure and testable while the CLI handles all the host-side orchestration. Next is the status bar plugin (step 8) and `zellai doctor` diagnostics.

---

## 2026-04-23 02:57 — Workspace management: data model, persistence, and CLI commands

Built the workspace layer end-to-end: a `Workspace` data model with templates (solo, pair, team, orchestrator-top), JSON file persistence with save/load/list/delete, and the three CLI commands (`zellai new`, `zellai list`, `zellai kill`) that create Zellij sessions from workspace configs. Also hardened `zellai run` with proper signal handling so wrapped agents clean up status files on SIGINT/SIGTERM, and removed redundant agent detection logic. Next is `zellai attach` for reconnecting to existing workspaces, then the teams command and `zellai.toml` project config.

---

## 2026-04-22 13:52 — Generic wrapper `zellai run` and status_writer tests

Implemented `zellai run <command>` so any agent (Codex, Gemini, Aider, etc.) can emit status files by wrapping its process — the wrapper detects the agent kind from the command name, writes status JSON on start/stop, and cleans up on exit. Added thorough unit tests for `StatusWriter` including agent auto-detection, session ID generation, and sessions directory resolution. Also fixed an output filename collision bug and a tilde expansion issue that broke path resolution. Next is named wrappers (`zellai-codex`, `zellai-gemini`, `zellai-aider`) and workspace management (`zellai new`, `zellai attach`, `zellai list`).

---

## 2026-04-21 22:07 — CLI binary scaffold and `zellai init`

Added a dual-target CLI binary (`src/bin/zellai/`) that builds natively alongside the WASM plugin, then implemented `zellai init` to auto-detect a `.claude/` directory and install the three hook scripts (`on-stop.sh`, `on-notification.sh`, `on-post-tool-use.sh`) into it. The dual-target approach worked cleanly — the plugin stays `wasm32-wasip1` while the CLI uses standard `std::fs`/`std::process` on the host. Next is the generic wrapper (`zellai run <command>`) so non-Claude agents can also emit status files.

---

## 2026-04-21 19:34 — Attention tracking and Claude Code hooks

Added the `AttentionTracker` module with priority-based rotation, dismissal, and idle detection, then wired `mark_stale` and session cleanup into the plugin event loop so stale agents get flagged automatically. Finished by writing the three Claude Code hook scripts (`on-stop.sh`, `on-notification.sh`, `on-post-tool-use.sh`) that emit status JSON to the sessions directory. The full read path now connects end-to-end: hooks write status → bridge reads it → sidebar renders it → attention highlights what needs your eye. Next is `zellai init` to auto-detect `.claude/` and install these hooks for real projects.

---

## 2026-04-21 18:45 — Status bridge and sidebar rendering

Built the pure-data `StatusBridge` layer for managing agent sessions (add/remove/stale-mark/sort), then wired it into the plugin event loop with `Timer` subscriptions and `run_command` for non-blocking file reads. Finished by implementing the sidebar renderer with compact and detailed card modes plus adaptive density selection based on available rows. All three pieces landed cleanly with passing tests and clippy. Next is Claude Code hooks — auto-detecting `.claude/` and writing `on-stop.sh` / `on-notification.sh` / `on-post-tool-use.sh` so real agent sessions produce status files.

---

## 2026-04-21 15:36 — Foundation trilogy: config, status, plugin scaffold

Built the three foundational pieces in one session: `ZellaiConfig` with serde defaults and validation, `AgentStatus` model with JSON parsing and staleness detection, and the minimal `ZellijPlugin` trait impl that compiles to `wasm32-wasip1`. Kept all logic pure and unit-testable — no Zellij host API calls yet, so `cargo test --lib` covers everything. Next up is the status bridge: subscribing to `Timer` events in the plugin, reading status files via `run_command`, and wiring parsed `AgentStatus` data into a basic sidebar render.

<!-- yoyo appends session entries here -->
