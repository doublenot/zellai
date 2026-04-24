# Assessment — 2026-04-24

## Build Status

All green:
- `cargo build --target wasm32-wasip1` — **pass** (compiles cleanly)
- `cargo test --lib` — **pass** (123 tests, 0 failures)
- `cargo clippy --target wasm32-wasip1` — **pass** (no warnings)

## Project State

The project has completed all 9 steps of the Current Direction roadmap outlined in YOYO.md:

1. **Plugin scaffold** ✅ — `src/lib.rs` implements `ZellijPlugin` trait, compiles to `wasm32-wasip1`, registers via `register_plugin!`
2. **Status bridge (reader)** ✅ — `src/status_bridge.rs` (378 lines) parses session JSON files, manages staleness, retains/removes sessions
3. **Sidebar plugin** ✅ — `src/sidebar.rs` (731 lines) renders compact/detailed/adaptive agent cards with attention indicators
4. **Claude Code hooks** ✅ — `hooks/` has all three scripts (`on-stop.sh`, `on-notification.sh`, `on-post-tool-use.sh`); `zellai init` installs them
5. **Generic wrapper** ✅ — `zellai run <command>` with auto-detection of agent kind; `StatusWriter` with signal handling and cleanup
6. **Workspace management** ✅ — `Workspace` data model with templates, JSON persistence, `zellai new/list/kill/attach` commands
7. **Teams command** ✅ — `teams.rs` generates KDL layouts (orchestrator-top/left/grid); `zellai teams` reads `zellai.toml` and launches
8. **Status bar plugin** ✅ — `status_bar.rs` renders minimal workspace-name + agent-count segment; plugin supports `mode=status-bar`
9. **DX commands** ✅ — `zellai doctor` checks environment; `zellai completions` generates bash/zsh/fish completions via clap

The plugin event loop is fully wired: `load()` subscribes to Timer/FileSystem/RunCommand events, `update()` dispatches async `ls`→`cat` chains via `run_command` context maps, `render()` delegates to sidebar or status-bar renderer based on config.

## Recent Changes (last 3 sessions)

| Date | Session | Summary |
|------|---------|---------|
| 2026-04-23 17:14 | Status bar + doctor + completions | Added status bar rendering mode, `zellai doctor` diagnostics, and shell completions generation |
| 2026-04-23 13:54 | Teams command + workspace attach | Completed `zellai attach`, built `teams.rs` KDL layout generator, `zellai teams` CLI subcommand |
| 2026-04-23 02:57 | Workspace management | Built workspace data model, templates, save/load/list/delete, CLI commands, signal handling for `zellai run` |

Git log shows a single squashed commit (`8ebdc41 yoyo: growth session wrap-up`) — the full history is in journal entries.

## Source Architecture

```
src/
  lib.rs                     295 lines  Plugin entry point, ZellijPlugin impl, event loop
  sidebar.rs                 731 lines  Sidebar renderer (compact/detailed/adaptive cards)
  workspace.rs               679 lines  Workspace model, templates, persistence (cfg-gated I/O)
  status_bridge.rs           378 lines  StatusBridge: session management, staleness, parsing
  teams.rs                   360 lines  KDL layout generation for team topologies
  attention.rs               357 lines  AttentionTracker: priority rotation, dismissal, idle detection
  config.rs                  264 lines  ZellaiConfig: TOML parsing with defaults
  status.rs                  258 lines  AgentStatus model, JSON parsing, validation
  status_bar.rs              175 lines  Status bar rendering
  bin/zellai/
    main.rs                  173 lines  CLI entry point (clap), subcommand dispatch
    status_writer.rs         571 lines  StatusWriter for `zellai run`
    workspace_cmd.rs         366 lines  new/list/kill/attach implementations
    doctor.rs                332 lines  Environment diagnostics
    teams_cmd.rs             227 lines  `zellai teams` CLI handler
    run.rs                   209 lines  `zellai run <cmd>` wrapper logic
    init.rs                  131 lines  `zellai init` hook installer

hooks/
  on-stop.sh                 Claude Code Stop hook
  on-notification.sh         Claude Code Notification hook
  on-post-tool-use.sh        Claude Code PostToolUse hook

Total: ~5,506 lines of Rust, 123 unit tests across 13 test files
```

## Open Issues Summary

No open issues on `doublenot/zellai` — the issue tracker is empty.

## Gaps & Opportunities

The 9-step build roadmap is complete. What remains are features from `zellai-vision.md` that aren't yet in the roadmap, plus quality/polish work:

### Features from the vision not yet implemented

1. **Keyboard navigation** — The vision describes jump-to-pane-by-index, cycle-through-attention with a keybinding, dismiss-without-switching. `AttentionTracker` has the logic (`next_attention`, `dismiss`), but there's no keybinding wiring in the plugin — no `Key` event subscription or `EventType::Key` handling in `update()`. The `[keybindings]` config section is defined in SCHEMA.md but not parsed/used.

2. **PR/CI status integration** — `AgentStatus` has `pr_number` and `pr_ci_status` fields, but nothing populates them. The vision calls for `gh` CLI integration to fetch PR number and CI status. This requires an async `run_command` chain (detect branch → `gh pr list` → `gh pr checks`) in the plugin event loop.

3. **Port detection** — `AgentStatus` has a `ports` field, but nothing detects listening ports. The vision describes surfacing dev server ports per pane.

4. **Named agent wrappers** — The vision lists `zellai-codex`, `zellai-gemini`, `zellai-aider`, `zellai-opencode` as named convenience wrappers. Only the generic `zellai run` exists.

5. **Execution logging** — The vision describes per-pane structured execution logs at `~/.local/share/zellai/sessions/<workspace>/<pane>.log` and a `zellai log <pane>` command.

6. **Task Board** — The vision's most ambitious feature: Kanban and DAG views for the orchestrator pane. Not started.

7. **Broadcast mode / targeted messaging** — Sending prompts to all or specific agent panes via Zellij pipes.

8. **`zellai.toml` project-local discovery** — Config currently only loads from the plugin's BTreeMap. The SCHEMA.md notes CWD-based walking is deferred.

### Quality & polish opportunities

9. **Integration testing** — No integration tests exist. Could add a test harness that exercises the hook scripts with mock environments.

10. **Error reporting in sidebar** — The sidebar renders agent cards but doesn't show a helpful message when the sessions directory is empty or unreachable.

11. **Documentation** — No `docs/` directory, no user-facing README beyond the project README. No man page or usage guide.

12. **Release workflow** — No CI builds WASM artifacts or publishes releases. The `hello.yml` and `grow.yml` workflows are for yoyo, not distribution.

13. **Named wrappers as symlinks/aliases** — The generic `zellai run` could detect `argv[0]` to act as `zellai-codex` etc., avoiding separate binaries.

### Prioritized next steps (by impact)

The highest-impact work given the complete roadmap:
- **Keyboard navigation** — the attention system is built but unusable without keybindings
- **PR/CI status** — a differentiating feature vs. raw terminal splits; `gh` integration
- **Named wrappers** — low effort, improves DX for non-Claude agents
- **Polish & docs** — make the project approachable for first contributors

## Bugs / Friction Found

- **No bugs** — zero TODOs, FIXMEs, or HACKs in the codebase. Build is clean, clippy is clean, all 123 tests pass.

- **Minor friction: single commit history** — The git log shows only one commit (`8ebdc41`). All development history is in journal entries rather than git commits, making it harder to bisect or review incremental changes.

- **`zellai.toml` keybindings section unparsed** — SCHEMA.md defines `[keybindings]` with `next_attention`, `dismiss`, `jump_to`, but `config.rs` doesn't parse this section. The `ZellaiConfig` struct has no keybindings field.

- **Hooks assume `claude` agent** — All three hook scripts hardcode `"agent": "claude"`. If hooks are reused for other agents (or a future generic hook system), the agent name should come from an environment variable.

- **`watch_filesystem()` called but scope unclear** — In `lib.rs`, `watch_filesystem()` is called on permission grant, but it's unclear what directory is being watched. The Zellij API watches the plugin's working directory, which may not be the sessions directory. The timer-based polling is the actual mechanism; the filesystem watch may be a no-op or watching the wrong path.
