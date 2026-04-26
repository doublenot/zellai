# Assessment — 2026-04-26

## Build Status
**All green.** `cargo build --target wasm32-wasip1` passes cleanly. `cargo test --lib` passes all 204 tests. `cargo clippy --target wasm32-wasip1` passes with zero warnings.

## Project State
The project is well into the mid-game. All 9 milestones from the YOYO.md Current Direction are at least partially implemented:

1. **Plugin scaffold** ✅ — WASM plugin compiles, loads in Zellij, renders three modes (Sidebar, StatusBar, TaskBoard)
2. **Status bridge (reader)** ✅ — `StatusBridge` parses session JSON, `mark_stale`, attention-first sorting
3. **Sidebar plugin** ✅ — Compact/detailed/adaptive card density, ANSI-aware rendering, attention badges, idle dimming
4. **Claude Code hooks** ✅ — Three hook scripts (`on-stop.sh`, `on-notification.sh`, `on-post-tool-use.sh`), `zellai init` installs them
5. **Generic wrapper** ✅ — `zellai run <command>`, auto-detects agent kind, argv0-based named wrappers (`zellai-codex` etc.)
6. **Workspace management** ✅ — `zellai new`, `zellai attach`, `zellai list`, `zellai kill`, 4 workspace templates
7. **Teams command** ✅ — `zellai teams`, `zellai.toml` config, layout generation (orchestrator-top/left/grid)
8. **Status bar plugin** ✅ — Single-line segment: workspace name, agent count, attention count
9. **DX commands** ✅ — `zellai doctor` diagnostics, shell completions (bash/zsh/fish)

Beyond the 9 milestones, the project also includes:
- **Task board** (Kanban + DAG views) — wired into plugin as third render mode
- **Per-pane logging** — `zellai log <pane>` with `StatusWriter` log output
- **PR/CI status** — `gh pr view`/`gh pr checks` integration with 30s caching
- **Port detection** — `/proc/net/tcp` parsing for child process ports
- **Keyboard navigation** — Arrow keys cycle agent cards, dismiss attention indicators
- **SVG screenshot** — Auto-generated plugin interface rendering

**Total codebase: 9,260 lines of Rust** across 18 source files + 258 lines of shell hooks.

## Recent Changes (last 3 sessions)
From journal (history squashed into single commit `7c100d3`):

1. **2026-04-25 13:13** — Task board plugin integration and port detection. Wired Kanban/DAG into WASM plugin as third render mode. Added port detection via `/proc/net/tcp` parsing.
2. **2026-04-25 02:48** — Task board data model and per-pane logging. `TaskBoard` parser with dependency levels, `zellai log` CLI with `--follow` stub.
3. **2026-04-24 13:44** — PR/CI status and ports in sidebar cards. `gh pr view`/`gh pr checks` integration in `StatusWriter`, sidebar renders port/CI info.

## Source Architecture

```
src/
├── lib.rs              (438 lines)  — ZellijPlugin trait impl, event loop, 3 render modes
├── sidebar.rs          (1460 lines) — Agent card rendering (compact/detailed/adaptive)
├── status.rs           (258 lines)  — AgentStatus model, parsing, validation
├── status_bridge.rs    (378 lines)  — In-memory session store, stale detection
├── status_bar.rs       (265 lines)  — Single-line status bar segment
├── task_board.rs       (1294 lines) — Kanban + DAG views, dependency levels
├── attention.rs        (357 lines)  — Attention tracking with cursor cycling
├── config.rs           (540 lines)  — zellai.toml parsing with serde defaults
├── teams.rs            (369 lines)  — Layout generation (3 topologies)
├── workspace.rs        (680 lines)  — Workspace model, templates, file persistence
└── bin/zellai/
    ├── main.rs          (257 lines) — CLI entry point (clap), 11 subcommands
    ├── run.rs           (295 lines) — `zellai run <command>` wrapper
    ├── status_writer.rs (1194 lines)— Status file writer, git/PR/port collection
    ├── init.rs          (199 lines) — `zellai init` hook installation
    ├── doctor.rs        (332 lines) — `zellai doctor` diagnostics
    ├── log.rs           (351 lines) — `zellai log` per-pane log viewer
    ├── teams_cmd.rs     (227 lines) — `zellai teams` CLI
    └── workspace_cmd.rs (366 lines) — `zellai new/list/kill/attach` CLI

hooks/
├── on-stop.sh          (75 lines)   — Writes idle status, deletes file
├── on-notification.sh  (91 lines)   — Writes waiting status with attention
└── on-post-tool-use.sh (92 lines)   — Writes thinking status with tool name

docs/screenshot.svg     (159 lines)  — SVG rendering of plugin interface
```

## Open Issues Summary
No open GitHub issues at this time (empty response from `gh issue list`).

## Gaps & Opportunities

### High-Value Gaps (from zellai-vision.md not yet implemented)

1. **Broadcast mode** — Vision specifies "send the same prompt to all agent panes at once via Zellij pipes." Not implemented at all. No code references broadcast or targeted messaging.

2. **Targeted message send** — Vision: "send a structured message to a specific agent pane by index or name without switching focus." Not implemented.

3. **Session Messages view** — Vision: "Future: session Messages view in orchestrator pane with send/receive history." Not started.

4. **`jump_to` keybinding** — Config defines it (`Ctrl g`) but `lib.rs` event handler doesn't handle it. Users can't jump to panes by index/name.

5. **`focus_terminal_pane` for attention cycling** — TODO in `lib.rs:241`: the `next_attention` keybinding updates the internal cursor but doesn't actually focus the Zellij pane. The core UX promise (cycle to the pane that needs you) is incomplete.

6. **Attention animation** — `attention_animation` config flag exists but sidebar rendering doesn't use it. No pulsing glow effect — just static badge dots and idle dimming.

7. **Custom team layouts** — `teams.rs` returns empty vec for `Custom` layout type. Vision describes `[[teams.layout]]` blocks with per-pane prompts.

8. **`--follow` mode for `zellai log`** — Stub that prints warning. Real tail -f behavior not implemented.

9. **Pipe bridge upgrade** — Vision mentions "in-band Zellij pipe bridge for event-driven, zero-polling updates" as future work. Not started (intentionally deferred per YOYO.md).

### Medium-Value Polish

10. **Code duplication** between `workspace_cmd::cmd_attach` and `teams_cmd::cmd_teams` for Zellij pane creation sequences.

11. **`parse_key` only handles `Ctrl+char`** — No Alt, Shift, or special key support for keybindings.

12. **Hook scripts omit `pr_number`/`pr_ci_status` fields** — Works due to `#[serde(default)]` but inconsistent with the schema.

13. **AttentionTracker dismissed set** never pruned for removed sessions — unbounded memory growth.

14. **WASM plugin check in doctor** only works from project root (hardcoded `target/` path).

15. **Background thread in `run.rs` creates second `StatusWriter`** — opens duplicate log file handle.

## Bugs / Friction Found

1. **Attention cycling doesn't actually focus panes** (lib.rs:241 TODO) — This is the most user-facing gap. The keybinding selects the next needing-attention agent in the sidebar display but never calls `focus_terminal_pane`, so the user doesn't switch to that pane. Core workflow broken.

2. **No real bugs found** — Build is clean, clippy is clean, all 204 tests pass. The codebase is solid.

3. **`write-chars` approach for pane creation** (teams_cmd.rs, workspace_cmd.rs) — Simulates keyboard input to type commands into panes. Fragile if shell isn't ready or has custom prompts. Works in practice but is a known brittleness.

4. **No integration tests** — Only unit tests exist. No end-to-end testing of the plugin loading in Zellij, hooks writing status files, or CLI commands interacting with Zellij sessions.
