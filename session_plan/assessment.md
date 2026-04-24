# Assessment — 2026-04-24

## Build Status
**All green.** WASM build, unit tests (132 passing), and clippy all pass cleanly with zero warnings.

```
cargo build --target wasm32-wasip1  ✓
cargo test --lib                    ✓ (132 tests, 0 failures)
cargo clippy --target wasm32-wasip1 ✓ (0 warnings)
```

## Project State
The project has completed all 9 milestones from the YOYO.md Current Direction roadmap at a foundational level:

1. **Plugin scaffold** ✓ — `lib.rs` implements `ZellijPlugin` trait, compiles to WASM, handles events
2. **Status bridge** ✓ — `status_bridge.rs` + `status.rs` parse session status files, model `AgentStatus`
3. **Sidebar plugin** ✓ — `sidebar.rs` renders compact/detailed/adaptive agent cards with attention indicators
4. **Claude Code hooks** ✓ — 3 shell hook scripts + `zellai init` installer
5. **Generic wrapper** ✓ — `zellai run <command>` with agent auto-detection, signal handling, status lifecycle
6. **Workspace management** ✓ — `Workspace` model, templates, save/load/list/delete + `zellai new/list/kill/attach`
7. **Teams command** ✓ — `teams.rs` generates layouts, `zellai teams` launches multi-agent sessions
8. **Status bar plugin** ✓ — `status_bar.rs` renders workspace summary segment
9. **DX commands** ✓ — `zellai doctor` diagnostics, shell completions (bash/zsh/fish), keyboard navigation

The entire codebase lives in a single initial commit (70504b3). ~5,900 lines of Rust + ~230 lines of shell hooks.

## Recent Changes (last 3 sessions)
From journal.md (all within the last ~36 hours):

1. **2026-04-24 03:18** — Agent-aware hooks (ZELLAI_AGENT env var for multi-agent sessions), `[keybindings]` config section with `parse_key`, keyboard event handling in plugin event loop (arrow navigation, dismiss)
2. **2026-04-23 17:14** — Status bar rendering mode, `zellai doctor` diagnostics, shell completions via clap_complete
3. **2026-04-23 13:54** — `zellai attach`, teams module with KDL layout generation, `zellai teams` CLI subcommand

All sessions produced working code. No reverts recorded.

## Source Architecture

```
src/
  lib.rs              (342 lines)  — WASM plugin: ZellijPlugin trait, event loop, command dispatch
  sidebar.rs          (731 lines)  — Pure rendering: agent cards, density selection, box drawing
  status.rs           (258 lines)  — Data model: AgentKind, AgentStatusValue, CiStatus, AgentStatus
  status_bridge.rs    (378 lines)  — In-memory session store: add/remove/stale/sort/GC
  status_bar.rs       (175 lines)  — Status bar single-line rendering
  attention.rs        (357 lines)  — Attention tracking: priority queue, cursor, dismiss, cycling
  config.rs           (417 lines)  — TOML config parsing with all defaults
  workspace.rs        (679 lines)  — Workspace model, templates, file persistence
  teams.rs            (360 lines)  — Team layout generation (orchestrator-top/left/grid)

  bin/zellai/
    main.rs           (173 lines)  — CLI dispatch: 9 subcommands via clap
    init.rs           (131 lines)  — Hook installer for Claude Code
    run.rs            (209 lines)  — Agent wrapper with signal handling
    status_writer.rs  (571 lines)  — Atomic JSON status file I/O
    doctor.rs         (332 lines)  — Environment diagnostics
    teams_cmd.rs      (227 lines)  — Teams CLI handler
    workspace_cmd.rs  (366 lines)  — Workspace CLI handlers

hooks/
  on-stop.sh          (65 lines)   — Writes idle, deletes status file
  on-notification.sh  (83 lines)   — Writes waiting + needs_attention
  on-post-tool-use.sh (84 lines)   — Writes thinking + tool name

Total: ~5,938 lines of Rust + 232 lines of shell
```

## Open Issues Summary
**No open GitHub issues.** The issue tracker is empty.

## Gaps & Opportunities

### Gaps relative to YOYO.md / zellai-vision.md

**High priority — functional gaps in shipped features:**

1. **`jump_to` keybinding is configured but never handled** — `lib.rs` has a TODO at line ~183 noting that session IDs can't yet be mapped to Zellij pane IDs. The attention cycling (`next_attention`) works but can't actually focus the target pane.

2. **Shell hook JSON injection vulnerability** — `on-stop.sh`, `on-notification.sh`, `on-post-tool-use.sh` interpolate `$working_dir`, `$ZELLAI_SESSION_ID`, and `$git_branch` into JSON without proper escaping. Paths containing quotes, backslashes, or special characters will produce invalid JSON and silently break the status bridge. Only `$notification`/`$tool_name` are escaped.

3. **No named wrappers** — YOYO.md step 5 mentions `zellai-codex`, `zellai-gemini`, `zellai-aider` as named wrappers. These don't exist — only `zellai run <command>` is available. (Low friction since `run` auto-detects agent kind, but the convenience aliases are part of the spec.)

4. **Custom team layouts return empty** — `teams.rs` `generate_team_layout` returns `vec![]` for `Custom` layout. `zellai.toml` `[[teams.layout]]` blocks from SCHEMA.md aren't parsed or applied.

5. **No ANSI color/styling in rendering** — Sidebar and status bar produce plain text with box-drawing characters. The vision describes "ambient glow", "red dot badge", "idle dimming" — none of which are implemented. Zellij plugins can emit ANSI escapes; this is a significant UX gap.

6. **No per-pane execution logging** — Vision specifies `~/.local/share/zellai/sessions/<workspace>/<pane>.log` and `zellai log <pane>`. Not implemented.

7. **No Task Board** — Vision describes orchestrator Task Board with Kanban and DAG views. Not implemented (appropriate for later phase).

8. **No broadcast/targeted messaging** — Vision specifies broadcast mode and targeted message send via Zellij pipes. Not implemented.

**Medium priority — polish & robustness:**

9. **`init.rs` has no unit tests** — The hook installation logic (detect, install, overwrite/skip) is fully testable but untested.

10. **Code duplication** — `teams_cmd::cmd_teams` and `workspace_cmd::cmd_attach` share nearly identical Zellij pane-creation logic. Should be extracted to a shared helper.

11. **Status bar pluralization** — "1 agents" instead of "1 agent" (minor but visible).

12. **`parse_key` only supports Ctrl modifier** — No Alt/Shift or special keys (arrows, function keys). Vision mentions arrow navigation.

13. **`doctor.rs` check functions are untestable** — They print directly and probe the filesystem. Could benefit from refactoring to return structured results.

**Lower priority — future features explicitly deferred:**

14. PR/CI status rendering (sidebar has the field but nothing fetches it via `gh`)
15. Pipe bridge upgrade (explicitly deferred in YOYO.md)
16. Browser integration (vision says "External (future)")
17. Session Messages view in orchestrator pane

### Opportunities

- **ANSI styling would have the highest visual impact** — adding colors, bold, dim to the sidebar and status bar would transform the UX from prototype to polished
- **Fixing the shell hook JSON safety issue is the most critical bug** — broken JSON = silent failure of the entire status bridge
- **Adding `init.rs` tests would be quick and high-value** — the module is small and its logic is testable with temp directories
- **Named wrappers are trivial** — thin symlink/script aliases that call `zellai run --agent <name>`

## Bugs / Friction Found

1. **BUG: Shell hooks don't JSON-escape `$working_dir`, `$ZELLAI_SESSION_ID`, `$git_branch`** — Any of these containing `"`, `\`, or control chars will produce malformed JSON. The `on-notification.sh` escapes `$notification` properly but not the other interpolated variables. This is a data corruption bug.

2. **BUG: "1 agents" pluralization** — `status_bar.rs` doesn't singularize "agent" when count is 1.

3. **INCOMPLETE: `jump_to` keybinding** — Configured in `config.rs`, referenced in `lib.rs` comments, but the key event handler in `update()` doesn't match on it. Dead config.

4. **INCOMPLETE: Custom team layouts** — `TeamsLayout::Custom` variant exists in the enum but `generate_team_layout` returns empty. CLI rejects "custom" as invalid in `parse_teams_layout`.

5. **NO TESTS: `init.rs`** — Hook installation logic is completely untested. A regression here would silently install broken hooks.

6. **DUPLICATION: Zellij pane creation** — `teams_cmd.rs` and `workspace_cmd.rs` both independently construct and execute Zellij CLI commands for creating tabs and panes with near-identical patterns.
