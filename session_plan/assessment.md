# Assessment — 2026-04-23

## Build Status
**All green.** `cargo build --target wasm32-wasip1` passes, `cargo test --lib` passes (113 tests, 0 failures), `cargo clippy --target wasm32-wasip1` clean (no warnings).

## Project State
The project is well into its build-out. Steps 1–7 from the YOYO.md roadmap are complete:

1. ✅ **Plugin scaffold** — `lib.rs` implements `ZellijPlugin` trait, compiles to `wasm32-wasip1`, renders sidebar
2. ✅ **Status bridge (reader)** — `status_bridge.rs` parses session JSON files, tracks agents, marks stale
3. ✅ **Sidebar plugin** — `sidebar.rs` renders agent cards with compact/detailed/adaptive density, attention indicators
4. ✅ **Claude Code hooks** — Three hook scripts (`on-stop.sh`, `on-notification.sh`, `on-post-tool-use.sh`) + `zellai init` to install them
5. ✅ **Generic wrapper** — `zellai run <command>` with auto-detection for claude/codex/gemini/aider/opencode, periodic status updates, signal handling
6. ✅ **Workspace management** — Data model with templates, JSON persistence, `zellai new/list/kill/attach` commands
7. ✅ **Teams command** — `zellai teams` with layout generation (orchestrator-top/left/equal-grid), `zellai.toml` config loading

**Not yet started:**
- Step 8: Status bar plugin
- Step 9: DX commands (`zellai doctor`, shell completions)

## Recent Changes (last 3 sessions)
From journal (most recent first):

1. **2026-04-23 13:54** — Teams command and workspace attach. Completed `zellai attach` for reconnecting to existing workspaces, then built `teams.rs` (layout generation) and `zellai teams` CLI subcommand.
2. **2026-04-23 02:57** — Workspace management end-to-end: `Workspace` data model, JSON persistence (save/load/list/delete), CLI commands (`zellai new/list/kill`), signal handling hardening for `zellai run`.
3. **2026-04-22 13:52** — Generic wrapper `zellai run` + `StatusWriter` with agent auto-detection, session ID generation, and unit tests.

Git log shows only a single merge commit visible on main (`0fd0401 Merge pull request #2`), suggesting prior work was squash-merged.

## Source Architecture

```
src/                          (library — WASM plugin + shared types)
  lib.rs              249 lines   ZellijPlugin impl, event loop, run_command dispatch
  status.rs           258 lines   AgentStatus, AgentKind, AgentStatusValue, parse_status
  config.rs           264 lines   ZellaiConfig, parse_config (TOML), all defaults
  sidebar.rs          731 lines   render_sidebar, compact/detailed cards, density selection
  status_bridge.rs    378 lines   StatusBridge (agent map, stale marking, retention)
  attention.rs        357 lines   AttentionTracker (cycling, dismissal, cursor mgmt)
  workspace.rs        679 lines   Workspace data model + templates + JSON persistence
  teams.rs            360 lines   generate_team_layout (orchestrator-top/left/grid)

src/bin/zellai/               (native CLI binary — host target only)
  main.rs             152 lines   clap CLI definition, subcommand dispatch
  init.rs             131 lines   `zellai init` — detect .claude/, install hooks
  run.rs              209 lines   `zellai run` — process wrapper with status tracking
  status_writer.rs    571 lines   StatusWriter (atomic JSON writes, git info, agent detection)
  workspace_cmd.rs    366 lines   new/list/kill/attach subcommands
  teams_cmd.rs        227 lines   `zellai teams` subcommand

hooks/                        (shell scripts — Claude Code integration)
  on-stop.sh                  Write idle status, then delete file (clean exit)
  on-notification.sh          Write waiting status + notification text
  on-post-tool-use.sh         Write thinking status + tool name

Total: ~4,932 lines of Rust, 113 unit tests
```

Key architectural decisions:
- **Dual-target crate**: `cdylib` for WASM plugin + `rlib` for native CLI binary
- **`#[cfg(not(target_arch = "wasm32"))]`** gates all `std::fs`/`std::process` code
- **Pure-logic modules** (status, config, sidebar, attention, teams) have zero `zellij_tile` imports → fully unit-testable
- **`run_command` context map** as continuation state for async multi-step operations in the plugin

## Open Issues Summary
**No open issues.** `gh issue list` returned an empty list.

## Gaps & Opportunities

### Next roadmap items (steps 8–9):
1. **Status bar plugin (step 8)** — A separate Zellij status bar segment showing workspace name + agent count + attention count. This is a second WASM binary (or a mode of the existing plugin). Requires understanding Zellij's status bar plugin API.
2. **DX commands (step 9)** — `zellai doctor` for diagnostics (check hooks installed, zellij version, sessions dir writable, etc.) and shell completions (bash, zsh, fish via clap's `generate` feature).

### Vision features not yet addressed:
- **Named wrappers** (`zellai-codex`, `zellai-gemini`, `zellai-aider`) — mentioned in step 5 but not implemented. Currently only `zellai run --agent <name>` exists. These could be symlinks or thin wrapper scripts.
- **Keyboard navigation in plugin** — `AttentionTracker` has the state logic but the plugin's `update()` doesn't handle key events yet. No subscription to `KeyEvent` or binding dispatch.
- **Port detection** — Status files always write `"ports": []`. No listening port detection logic exists.
- **PR/CI integration** — No `gh` CLI integration. `pr_number` and `pr_ci_status` are always null.
- **Task Board** — The vision describes an orchestrator Task Board (Kanban + DAG views). Not started.
- **Broadcast mode / targeted messages** — Sending prompts to agent panes. Not started.
- **Execution logging** — Per-pane structured execution logs. Not started.

### Code quality opportunities:
- **Duplicated Zellij CLI orchestration** — `workspace_cmd::cmd_attach` and `teams_cmd::cmd_teams` share nearly identical logic for creating tabs, splitting panes, and writing commands. This should be factored into a shared helper.
- **No integration test for the plugin** — All tests are unit tests. The plugin event loop (`lib.rs`) is untested except by manual WASM loading.

## Bugs / Friction Found
- **No bugs found.** Build, tests, and clippy are all clean.
- **Minor friction**: The `[[bin]]` is named `zellai-cli` (to avoid WASM output collision) but the vision and docs refer to the command as `zellai`. Users would need an alias or rename. This is a known constraint documented in learnings.
- **`zellai attach` uses `write-chars` to type commands** — This writes the command text into the terminal rather than running it directly. If the shell prompt isn't ready when `write-chars` fires, the command could be garbled. Same pattern in `teams_cmd`. A more robust approach would be `zellij run --` or a startup script, but Zellij CLI options are limited.
