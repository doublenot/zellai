# Assessment — 2026-04-25

## Build Status

All green:
- `cargo build --target wasm32-wasip1` — **pass** (WASM plugin compiles cleanly)
- `cargo test --lib` — **pass** (187 tests, 0 failures)
- `cargo clippy --target wasm32-wasip1` — **pass** (0 warnings)
- `cargo clippy` (native target) — **pass** (0 warnings)

## Project State

zellai is a mature dual-target crate (WASM plugin + native CLI) at ~8,400 lines of Rust across 21 source files, with 187 unit tests. All 9 milestones in the Current Direction are substantially implemented:

1. **Plugin scaffold** ✅ — `lib.rs` implements `ZellijPlugin` (load/update/render), compiles to `wasm32-wasip1`
2. **Status bridge** ✅ — `status_bridge.rs` manages in-memory agent sessions; `status.rs` models `AgentStatus` with JSON parsing, staleness detection, validation
3. **Sidebar plugin** ✅ — `sidebar.rs` (1,460 lines) renders adaptive agent cards with compact/detailed/mixed density, attention indicators (badge + glow + idle dimming), PR/CI metadata, ports display
4. **Claude Code hooks** ✅ — 3 hook scripts (`on-stop.sh`, `on-notification.sh`, `on-post-tool-use.sh`) with JSON-safe escaping and atomic writes; `zellai init` auto-detects `.claude/` and installs them
5. **Generic wrapper** ✅ — `zellai run <command>` with status tracking, signal handling, periodic updates; named wrappers (`zellai-codex`, `zellai-gemini`, `zellai-aider`) via argv[0] detection
6. **Workspace management** ✅ — `Workspace` data model with 4 templates, JSON persistence (save/load/list/delete), CLI commands (`new`, `list`, `kill`, `attach`)
7. **Teams command** ✅ — `teams.rs` generates layouts (orchestrator-top/left/grid), `zellai teams` CLI reads `zellai.toml` and launches multi-agent sessions; task board data model with DAG/Kanban/stats
8. **Status bar plugin** ✅ — `status_bar.rs` renders workspace summary segment (`⬡ name | N agents | M⚠`)
9. **DX commands** ✅ — `zellai doctor` checks environment (Zellij, WASM, hooks, gh, sessions dir); shell completions for bash/zsh/fish; `zellai log` for per-pane execution logs

## Recent Changes (last 3 sessions)

Git history is shallow (1 commit visible: `f5973d1 yoyo: growth session wrap-up`), but journal entries show:

1. **2026-04-25 02:48** — Task board data model (`task_board.rs`) with Kanban columns, DAG dependency levels, aggregate stats. Per-pane logging (`zellai log`) with `--follow` mode stub and `--list` discovery.
2. **2026-04-24 13:44** — PR/CI status collection in `StatusWriter` via `gh pr view`/`gh pr checks`. Extended detailed sidebar cards to display ports and PR/CI info with colored status icons.
3. **2026-04-24 12:55** — Named wrapper binaries (`zellai-codex`, `zellai-gemini`, `zellai-aider`). Clippy cleanup (`&PathBuf` → `&Path`). SVG screenshot regeneration.

## Source Architecture

```
src/                           (8,364 lines total)
├── lib.rs                     (343)  WASM plugin entry: ZellijPlugin impl, event loop
├── sidebar.rs                 (1460) Sidebar rendering: agent cards, density, ANSI
├── status.rs                  (258)  AgentStatus model, JSON parsing, validation
├── status_bridge.rs           (378)  In-memory session store, stale detection
├── config.rs                  (540)  ZellaiConfig TOML parsing with defaults
├── attention.rs               (357)  Attention tracker: cycle, dismiss, cursor
├── teams.rs                   (369)  Team layout generation (pure logic)
├── task_board.rs              (571)  Task board model: Kanban, DAG, stats
├── status_bar.rs              (265)  Status bar segment renderer
├── workspace.rs               (680)  Workspace model + JSON persistence
├── bin/
│   ├── zellai/
│   │   ├── main.rs            (257)  CLI entry: clap subcommands
│   │   ├── run.rs             (290)  zellai run: process wrapper + status
│   │   ├── init.rs            (199)  zellai init: hook installation
│   │   ├── doctor.rs          (332)  zellai doctor: env diagnostics
│   │   ├── log.rs             (351)  zellai log: per-pane log viewer
│   │   ├── status_writer.rs   (999)  Write-side: atomic JSON, git, PR/CI
│   │   ├── teams_cmd.rs       (227)  zellai teams: CLI → layout → Zellij
│   │   └── workspace_cmd.rs   (366)  zellai new/list/kill/attach
│   └── screenshot.rs          (122)  SVG screenshot generator
hooks/
├── on-stop.sh                 (75)   Hook: idle → cleanup
├── on-notification.sh         (91)   Hook: notification → needs_attention
└── on-post-tool-use.sh        (92)   Hook: tool use → thinking

Tests: 187 total (59 sidebar + 30 status_writer + 27 workspace + 22 config +
       20 task_board + 15 status_bridge + 14 workspace_cmd + 13 attention +
       12 status_bar + 12 teams + 12 log + 9 teams_cmd + 8 status +
       6 init + 5 doctor + 5 run)
```

## Open Issues Summary

No open GitHub issues (`gh issue list` returns `[]`).

## Gaps & Opportunities

### Functional gaps vs. vision (ranked by impact):

1. **Task board rendering not wired into plugin** — `task_board.rs` has a complete data model (Kanban columns, DAG with cycle detection, aggregate stats), but there's no rendering in the sidebar or dedicated pane view. The vision specifies a "Task Board views: Kanban and dependency-aware DAG tree (ASCII, level-grouped)" — the data layer exists but the display layer is missing.

2. **Port detection unimplemented** — `StatusWriter.ports` is always `[]`. The vision promises "What ports its dev server is listening on" and the sidebar renderer already displays ports if present — the detection logic is the gap.

3. **Focus-pane on attention cycle** — Keyboard navigation cycles attention (`Ctrl+A`), but can't actually focus the corresponding Zellij pane (TODO in `lib.rs:186` — needs mapping session_id → pane ID for `focus_terminal_pane`).

4. **`--follow` mode for `zellai log`** — Currently prints a "not yet implemented" warning. The vision specifies per-pane session log retrieval; the core `log` command works, but live tailing doesn't.

5. **Custom teams layout** — `generate_team_layout()` returns empty Vec for `Custom` layout type. The vision defines `[[teams.layout]]` TOML blocks with custom pane configs — not parsed yet.

6. **Broadcast mode & targeted message send** — Vision specifies "send the same prompt to all agent panes at once via Zellij pipes" and "send a structured message to a specific agent pane by index or name." Not started.

7. **Pane command injection fragility** — `workspace_cmd.rs` and `teams_cmd.rs` use `write-chars` to inject commands into panes, which is fragile. Should use Zellij's `--command` flag or `zellij run` for reliability.

### Polish & hardening opportunities:

8. **No integration/E2E tests** — 187 unit tests for pure logic, but zero integration tests verifying the actual WASM plugin in a Zellij session.

9. **`zellai-opencode` named wrapper missing** — Vision lists `zellai-opencode` but only codex/gemini/aider have named wrappers.

10. **Pipe bridge upgrade path** — Listed as a future milestone in both YOYO.md and the vision ("in-band Zellij pipe bridge for event-driven, zero-polling updates"). Not started, by design.

## Bugs / Friction Found

- **No bugs found** — build is clean, all 187 tests pass, clippy has zero warnings on both targets.
- **Shallow git history** — only 1 commit visible (`f5973d1`), which makes incremental progress tracking difficult. This is a CI/shallow-clone artifact, not a project issue.
- **1 TODO in production code** — `lib.rs:186`: pane focus mapping. This is the only known incomplete feature in the core plugin code.
- **`--follow` in `zellai log`** silently degrades — it prints a warning but doesn't error, which could confuse users expecting tail behavior.
