# Assessment — 2026-04-21

## Build Status
- `cargo build --target wasm32-wasip1` — **pass** (no warnings)
- `cargo test --lib` — **pass** (58 tests, 0 failures)
- `cargo clippy --target wasm32-wasip1` — **pass** (no warnings)

## Project State
The project has completed the first three items on the Current Direction roadmap:

1. **Plugin scaffold** ✅ — `lib.rs` implements `ZellijPlugin` trait with `load()`, `update()`, `render()`. Compiles to `wasm32-wasip1`. Subscribes to Timer, FileSystem*, RunCommandResult, and PermissionRequestResult events. Uses `run_command` for async file I/O.

2. **Status bridge (reader)** ✅ — `status.rs` defines `AgentStatus` with full schema from SCHEMA.md (version, session_id, agent kind, status, git branch/dirty, working_dir, last_message, ports, PR/CI, needs_attention, updated_at). Validation enforces the `needs_attention ↔ status==Waiting` invariant. `status_bridge.rs` provides `StatusBridge` for managing the agent map (update_from_json, remove_session, mark_stale, agents_sorted). The plugin event loop chains `ls` → `cat` via `run_command` with context maps as continuation state.

3. **Sidebar plugin** ✅ — `sidebar.rs` renders agent cards in compact (1-line) or detailed (3-line) modes. Adaptive density algorithm selects AllCompact/AllDetailed/Mixed based on available rows and attention count. Box-drawing borders, status icons (⚙/⚠/○/✗), truncation with ellipsis, center-aligned empty state. 731 lines with 21 tests covering all rendering paths.

4. **Config** ✅ — `config.rs` defines `ZellaiConfig` with sidebar, teams, and bridge sections. All fields have mandated defaults. TOML parsing with serde defaults for missing keys. 9 tests.

## Recent Changes (last 3 sessions)
Only 1 commit in the git log (this is a young project):
- `c18a993` — yoyo: growth session wrap-up

From journal:
- **2026-04-21 18:45** — Built StatusBridge layer, wired it into the plugin event loop with Timer subscriptions and run_command for non-blocking file reads, implemented sidebar renderer with compact/detailed/adaptive density.
- **2026-04-21 15:36** — Built config parsing (ZellaiConfig), status model (AgentStatus), and the minimal ZellijPlugin scaffold. Kept all logic pure and unit-testable.

## Source Architecture
```
src/
  lib.rs             175 lines  Plugin entry point, ZellijPlugin trait, event dispatch
  config.rs          264 lines  ZellaiConfig, SidebarConfig, TeamsConfig, BridgeConfig + parsing
  status.rs          258 lines  AgentStatus, AgentKind, AgentStatusValue, CiStatus + parsing
  status_bridge.rs   308 lines  StatusBridge: agent map management, stale detection, sorting
  sidebar.rs         731 lines  Sidebar rendering: compact/detailed cards, adaptive density
                   ─────────
                   1,736 total

Missing directories:
  hooks/          — not created yet (Claude Code hook scripts)
  docs/           — not created yet
  src/attention.rs — not created yet
  src/workspace.rs — not created yet
  src/teams.rs     — not created yet
  src/wrappers/    — not created yet
```

## Open Issues Summary
No open issues on doublenot/zellai.

## Gaps & Opportunities
Measuring against YOYO.md Current Direction roadmap:

| # | Item | Status |
|---|------|--------|
| 1 | Plugin scaffold | ✅ Done |
| 2 | Status bridge (reader) | ✅ Done |
| 3 | Sidebar plugin | ✅ Done |
| **4** | **Claude Code hooks** | **🔴 Not started** — Next on roadmap. Needs: `hooks/on-stop.sh`, `hooks/on-notification.sh`, `hooks/on-post-tool-use.sh` that write status JSON files. Also needs `zellai init` command to auto-detect `.claude/` and configure hooks. |
| 5 | Generic wrapper | Not started — `zellai run <command>`, named wrappers |
| 6 | Workspace management | Not started — `zellai new/attach/list/kill` |
| 7 | Teams command | Not started — `zellai teams`, `zellai.toml` project config |
| 8 | Status bar plugin | Not started |
| 9 | DX commands | Not started — `zellai doctor`, shell completions |

**The biggest gap is item 4: Claude Code hooks.** This is the write side of the status bridge — without it, there's no way for agents to produce the status files that the plugin reads. The entire read pipeline (status_bridge → sidebar rendering) is built but has nothing to read.

Secondary gaps:
- **No `attention.rs` module** — the SCHEMA.md spec calls for attention state tracking with `next_attention()` cycling and `dismiss()`. The sidebar renders attention indicators, but there's no keyboard navigation or dismiss functionality.
- **No CLI binary** — the project only has a cdylib (WASM plugin). `zellai init`, `zellai run`, `zellai teams` etc. need a native CLI binary. This will likely require a second crate or a binary target alongside the cdylib.
- **No `mark_stale` call in the event loop** — `StatusBridge::mark_stale()` exists and is tested, but `lib.rs` never calls it. The timer handler lists sessions and reads files but doesn't invoke staleness detection. This is a functional gap: stale agents will never be transitioned to Idle.

## Bugs / Friction Found
1. **`mark_stale` is never called** — `StatusBridge::mark_stale(now_epoch)` is implemented and tested but the plugin's timer handler in `lib.rs` doesn't call it. Stale agents will remain in their last known status forever. Needs to be called on each timer tick, but requires a source of `now_epoch` (current time) — WASM plugins can't use `std::time`, so this likely needs a `run_command` call to `date +%s` or similar, or tracking elapsed time from timer ticks.

2. **Session cleanup for disappeared files** — The `list_sessions` handler reads filenames from `ls` output and issues `cat` for each `.json` file. But it never removes sessions that existed previously but are no longer listed by `ls`. If an agent's status file is deleted, the session stays in the bridge map until the next `cat` fails with a non-zero exit code (which removes it). This is a race: the file must be `cat`-ed and fail before the session is cleaned up. A more robust approach would compare the `ls` result to known sessions and remove stale entries.

3. **No error logging** — When `run_command` fails or JSON parsing fails, the plugin silently ignores the error. There's no logging mechanism to help debug issues. Zellij plugins can use `eprintln!` for debug output visible in the Zellij log — this would be useful during development.

4. **`edition = "2024"` in Cargo.toml** — The Rust 2024 edition is used, which is fine for Rust ≥1.85 but worth noting since YOYO.md says minimum 1.84. The 2024 edition was stabilized in 1.85. Not currently blocking (the CI environment has a sufficient toolchain), but could cause issues for contributors on 1.84.
