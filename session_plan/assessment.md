# Assessment — 2026-04-21

## Build Status
- `cargo build --target wasm32-wasip1` — **pass** (clean)
- `cargo test --lib` — **pass** (75 tests, 0 failures)
- `cargo clippy --target wasm32-wasip1` — **pass** (no warnings)

## Project State

zellai is a Zellij WASM plugin (cdylib targeting `wasm32-wasip1`) at **steps 1–3 complete** of the 9-step roadmap, with partial step 4 (hooks written, but no `zellai init` CLI command yet).

### What exists:
- **Plugin scaffold** (`lib.rs`) — implements `ZellijPlugin` trait with `load()`, `update()`, `render()`. Subscribes to Timer, FileSystem events, RunCommandResult. Handles async `ls` → `cat` pipeline for reading session status files via `run_command` with context-map continuations.
- **Status model** (`status.rs`) — `AgentStatus` struct matching SCHEMA.md JSON schema. Supports 5 agent types + Unknown via `#[serde(other)]`. Validation enforces `needs_attention ↔ status == Waiting` invariant. Staleness detection.
- **Config** (`config.rs`) — `ZellaiConfig` with `[sidebar]`, `[teams]`, `[bridge]` sections. Full serde roundtrip. All defaults match YOYO.md mandates (left, adaptive, orchestrator-top, true).
- **Status bridge** (`status_bridge.rs`) — Pure data manager for tracking agent sessions. Add/remove/stale-mark/sort/retain operations. No Zellij API dependency.
- **Sidebar renderer** (`sidebar.rs`) — Renders agent cards with compact (1-line) and detailed (3-line) modes. Adaptive density selection. Box-drawing chrome (╭╮╰╯│). Attention indicators (⚠ icon, `[!]` badge). Empty state rendering.
- **Attention tracker** (`attention.rs`) — Tracks sessions needing attention. Cursor-based cycling with `next_attention()`. Dismiss/clear mechanics. Cursor preservation across updates.
- **Claude Code hooks** (`hooks/`) — Three shell scripts: `on-stop.sh` (idle → delete), `on-notification.sh` (waiting + message), `on-post-tool-use.sh` (thinking + tool name). All use atomic write (`tmp` → `mv`), respect `ZELLAI_SESSION_ID` env var, degrade gracefully.

### What does NOT exist yet:
- No `zellai init` CLI command (hook auto-installation)
- No `zellai run` generic wrapper or named wrappers (`zellai-codex`, etc.)
- No `workspace.rs` (workspace save/restore)
- No `teams.rs` (teams layout launcher)
- No status bar plugin
- No `zellai doctor` or shell completions
- No CLI binary at all — only the WASM plugin library

## Recent Changes (last 3 sessions)

All development happened in a single squashed commit on 2026-04-21:

1. **Session 1 (15:36)** — Foundation trilogy: config parsing (`ZellaiConfig`), status model (`AgentStatus`), minimal `ZellijPlugin` scaffold that compiles to WASM.
2. **Session 2 (18:45)** — Status bridge (`StatusBridge`) for managing agent sessions, wired into plugin event loop with Timer + run_command. Sidebar renderer with compact/detailed/adaptive card modes.
3. **Session 3 (19:34)** — Attention tracker with priority cycling and dismissal. Stale detection + session cleanup in plugin loop. Three Claude Code hook scripts.

Git log shows 1 commit total: `d4f8470 yoyo: growth session wrap-up`

## Source Architecture

```
src/
  lib.rs              210 lines  — Plugin entry point, event dispatch, run_command handler
  sidebar.rs          731 lines  — Pure rendering (cards, borders, density), 12 tests
  status_bridge.rs    378 lines  — Session data management, 14 tests
  attention.rs        357 lines  — Attention cycling/dismiss state, 13 tests
  config.rs           264 lines  — TOML config with defaults, 8 tests
  status.rs           258 lines  — AgentStatus model + JSON parsing, 8 tests
hooks/
  on-stop.sh           61 lines  — Claude Code Stop hook
  on-notification.sh   79 lines  — Claude Code Notification hook
  on-post-tool-use.sh  80 lines  — Claude Code PostToolUse hook

Total: 2,418 lines (src + hooks)
Tests: 75 unit tests, all passing
```

## Open Issues Summary

No open GitHub issues. The project has zero community-filed issues at this time.

## Gaps & Opportunities

Ordered by the YOYO.md roadmap, with current completion status:

| Step | Feature | Status |
|------|---------|--------|
| 1 | Plugin scaffold | ✅ Complete |
| 2 | Status bridge (reader) | ✅ Complete |
| 3 | Sidebar plugin | ✅ Complete |
| 4 | Claude Code hooks | ⚠️ Hooks written, but `zellai init` (auto-detect + install hooks) not implemented |
| 5 | Generic wrapper | ❌ Not started — `zellai run`, named wrappers |
| 6 | Workspace management | ❌ Not started — `workspace.rs`, CLI commands |
| 7 | Teams command | ❌ Not started — `teams.rs`, `zellai.toml` layout |
| 8 | Status bar plugin | ❌ Not started |
| 9 | DX commands | ❌ Not started — `zellai doctor`, shell completions |

**The biggest gap is the lack of any CLI binary.** The project currently only has the WASM plugin library. Steps 4–9 all require a native CLI tool (`zellai` binary) that can:
- Install hooks (`zellai init`)
- Wrap agent processes (`zellai run`)
- Manage workspaces (`zellai new/attach/list/kill`)
- Launch team layouts (`zellai teams`)
- Run diagnostics (`zellai doctor`)

This means the **next natural step** is either:
1. **Complete step 4** — Build a `zellai init` CLI command that auto-detects `.claude/` directories and installs the hook scripts. This requires adding a `[[bin]]` target to `Cargo.toml` (native binary, not WASM).
2. **Start step 5** — Build `zellai run <command>` as a native wrapper that writes status files. This also requires a native binary.

Both converge on: **we need a CLI binary scaffold** (separate from the WASM plugin) before any of the remaining roadmap steps can proceed.

## Bugs / Friction Found

1. **No bugs found** — build, tests, and clippy all pass cleanly.

2. **`watch_filesystem()` called without a path argument** — In `lib.rs:99`, `watch_filesystem()` is called on permission grant. The zellij-tile API's `watch_filesystem()` watches the plugin's data directory by default, but the sessions directory may be elsewhere. This could be a no-op or watch the wrong directory. The Timer-based polling compensates, but this should be verified when testing against a real Zellij host.

3. **Hook scripts assume `claude` agent** — All three hooks hardcode `"agent": "claude"` in the JSON output. This is correct for Claude Code hooks specifically, but worth noting for when the generic wrapper is built.

4. **No `Cargo.toml` binary target** — The crate is `cdylib` only. Adding a native CLI will require either a `[[bin]]` section in `Cargo.toml` or a workspace split. The WASM plugin can't use `std::fs`/`std::process`, but the CLI must — this architectural split needs careful planning to share types (config, status model) without pulling WASM-incompatible code into the plugin.

5. **sessions_dir uses `~` tilde** — The default `sessions_dir` is `~/.local/share/zellai/sessions`. The tilde isn't expanded by Rust's std or by the WASM host. The `ls` and `cat` commands passed to `run_command` will use shell expansion, so it works today, but a native CLI binary would need explicit tilde expansion.

6. **Single commit history** — The entire project was squashed into one commit. This makes it harder to bisect or understand incremental progress. Future sessions should commit incrementally.
