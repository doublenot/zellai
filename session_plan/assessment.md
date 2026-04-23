# Assessment — 2026-04-23

## Build Status
- `cargo build --target wasm32-wasip1` — **pass** (clean, no warnings)
- `cargo test --lib` — **pass** (75 tests, 0 failures)
- `cargo clippy --target wasm32-wasip1` — **pass** (clean, no warnings)

## Project State
The project has completed steps 1–5 of the Current Direction roadmap:

1. **Plugin scaffold** ✅ — `ZellijPlugin` trait impl in `lib.rs`, compiles to WASM, handles events
2. **Status bridge (reader)** ✅ — `StatusBridge` manages agent sessions, parses JSON, marks stale, retains/removes sessions
3. **Sidebar plugin** ✅ — full sidebar rendering with compact/detailed/adaptive card density, attention indicators, Unicode box-drawing borders
4. **Claude Code hooks** ✅ — three hook scripts (`on-stop.sh`, `on-notification.sh`, `on-post-tool-use.sh`) in `hooks/`; `zellai init` auto-detects `.claude/` and installs them
5. **Generic wrapper** ✅ — `zellai run <command>` spawns child process, writes status files atomically, auto-detects agent kind from command name, periodic background updates, clean exit/signal handling

Supporting infrastructure:
- `ZellaiConfig` with full serde defaults and TOML parsing
- `AgentStatus` model with JSON parsing and validation (invariant enforcement)
- `AttentionTracker` with priority cycling, dismissal, cursor preservation
- Dual-target crate: `cdylib` for WASM plugin + `rlib`/`[[bin]]` for native CLI (`zellai-cli`)
- Plugin event loop: `Timer` → `ls` → `cat` → parse → render; `run_command` continuation pattern; home dir resolution via shell-out

## Recent Changes (last 3 sessions)

1. **2026-04-22** — Implemented `zellai run <command>` with `StatusWriter`, agent auto-detection, session ID generation, and thorough unit tests. Fixed output filename collision bug and tilde expansion issue.
2. **2026-04-21 22:07** — Added CLI binary scaffold (`src/bin/zellai/`) with `zellai init` to auto-detect `.claude/` and install hook scripts. Dual-target approach (WASM + native).
3. **2026-04-21 19:34** — Added `AttentionTracker` module, wired `mark_stale` and session cleanup into the plugin event loop, wrote the three Claude Code hook scripts.

Only 1 commit in `git log` (squashed or single-commit main):
- `1ac606e yoyo: growth session wrap-up`

## Source Architecture

```
src/
  lib.rs               250 lines  — plugin entry point, ZellijPlugin impl, event dispatcher
  sidebar.rs           731 lines  — sidebar rendering (compact/detailed/adaptive cards)
  status_bridge.rs     378 lines  — agent session management (read-side)
  attention.rs         357 lines  — attention state tracking + keyboard cycling
  config.rs            264 lines  — zellai.toml parsing with serde defaults
  status.rs            258 lines  — AgentStatus model, JSON parsing, validation
  bin/zellai/
    main.rs             55 lines  — CLI entry point (clap), dispatches to subcommands
    status_writer.rs   571 lines  — status file writer (write-side, atomic)
    run.rs             196 lines  — `zellai run` wrapper implementation
    init.rs            131 lines  — `zellai init` hook installer
hooks/
  on-stop.sh          1676 bytes  — writes idle status, deletes file
  on-notification.sh  2235 bytes  — writes attention + last_message
  on-post-tool-use.sh 2148 bytes  — updates thinking status + tool name

Total: 3,191 lines of Rust, 75 unit tests
```

## Open Issues Summary
No open issues on GitHub (`gh issue list` returns empty).

## Gaps & Opportunities

The roadmap shows 9 steps. Steps 1–5 are complete. The next items in order:

### Step 6: Workspace management (next up)
- `src/workspace.rs` — does not exist yet
- CLI commands not implemented: `zellai new <name>`, `zellai attach <name>`, `zellai list`, `zellai kill <name>`
- Named workspaces with saved pane layouts
- Workspace templates: single agent, team, review, research
- Save/restore layouts to `<user-data-dir>/zellai/workspaces/<name>.json`

### Step 7: Teams command
- `src/teams.rs` — does not exist yet
- `zellai teams` CLI command to launch orchestrator-top layout
- `zellai.toml` `[[teams.layout]]` custom layout blocks
- Pane spawning via Zellij API with agent commands

### Step 8: Status bar plugin
- Minimal Zellij status bar segment (workspace name + agent count + attention count)
- Separate plugin binary or mode flag

### Step 9: DX commands
- `zellai doctor` — diagnostics (check Zellij version, hook scripts, sessions dir, `gh` CLI)
- Shell completions (bash, zsh, fish) — likely via `clap_complete`

### Named wrappers (mentioned in step 5 journal)
- `zellai-codex`, `zellai-gemini`, `zellai-aider` — convenience symlinks/scripts
- Not yet implemented, though `zellai run --agent codex -- codex` works equivalently

### Missing from current implementation
- No keybinding handling in the plugin (SCHEMA.md defines `[keybindings]` section but the plugin doesn't process key events)
- No `watch_filesystem()` path filtering — the plugin watches the entire filesystem, not just the sessions directory
- No signal handling in `zellai run` (SIGINT/SIGTERM aren't caught to write final status before exit)
- `ports` field is always empty `[]` in `StatusWriter` — no port detection implemented
- PR/CI status fields (`pr_number`, `pr_ci_status`) are never populated by any writer

## Bugs / Friction Found

1. **No signal handling in `zellai run`**: If the user hits Ctrl-C, the wrapper process is killed without writing a final status or cleaning up the status file. The stale detection will eventually catch it, but there's a 60-second window where the sidebar shows a "thinking" agent that's actually dead.

2. **Background thread agent detection is redundant**: In `run.rs` lines 72-76, `detect_agent` is called again on the command name to pass to the background thread's `StatusWriter`, even though `agent` was already resolved. The `agent` variable should just be cloned.

3. **`watch_filesystem()` is called without a path**: In `lib.rs` line 115, `watch_filesystem()` is called with no arguments, which watches the entire filesystem. This could generate excessive `FileSystemUpdate` events on systems with active file I/O. The sessions directory should ideally be the only watched path.

4. **No `docs/` directory**: YOYO.md mentions `docs/brainstorms/` and `docs/plans/` but neither exists.

5. **`edition = "2024"` in Cargo.toml**: Uses Rust 2024 edition, which requires Rust 1.85+. The YOYO.md says minimum version is 1.84. Minor inconsistency — should either bump the minimum version docs or use edition 2021.
