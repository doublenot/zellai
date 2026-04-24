# Assessment — 2026-04-24

## Build Status

**All green.** WASM plugin build, host-target tests, and clippy all pass cleanly.

- `cargo build --target wasm32-wasip1` — ✅ success
- `cargo test --lib` — ✅ 133 tests pass, 0 failures
- `cargo clippy --target wasm32-wasip1` — ✅ no warnings

## Project State

The project is remarkably mature. All 9 milestones from the Current Direction in YOYO.md have been implemented to some degree:

| # | Milestone | Status |
|---|-----------|--------|
| 1 | Plugin scaffold | ✅ Complete — compiles to WASM, loads in Zellij |
| 2 | Status bridge (reader) | ✅ Complete — `StatusBridge` reads/parses session JSON, stale detection |
| 3 | Sidebar plugin | ✅ Complete — compact/detailed/adaptive cards, attention indicators, box-drawing UI |
| 4 | Claude Code hooks | ✅ Complete — 3 hook scripts, `zellai init` installs them, json_escape security |
| 5 | Generic wrapper | ✅ Complete — `zellai run`, named wrappers via argv[0] symlink detection |
| 6 | Workspace management | ✅ Complete — `new`, `list`, `kill`, `attach` + templates + JSON persistence |
| 7 | Teams command | ✅ Complete — `zellai teams`, KDL layout generation, config-driven |
| 8 | Status bar plugin | ✅ Complete — `PluginMode::StatusBar` renders workspace summary line |
| 9 | DX commands | ✅ Complete — `zellai doctor` (7 checks), shell completions (bash/zsh/fish) |

**Total codebase:** ~6,046 lines of Rust across 16 source files, plus ~230 lines of shell hooks.

## Recent Changes (last 3 sessions)

The repo has a single squashed commit (`bc8c1aa yoyo: growth session wrap-up`), so git log shows no incremental history. The journal records 10 sessions spanning 2026-04-21 through 2026-04-24:

1. **2026-04-24 12:55** — Named wrapper binaries (`zellai-codex`, `zellai-gemini`, `zellai-aider`), clippy fixes (`&PathBuf` → `&Path`), SVG screenshot update
2. **2026-04-24 04:48** — Pluralization fix ("1 agents" → "1 agent"), `zellai init` tests, shell injection security patch in hooks, SVG screenshot + README embed
3. **2026-04-24 03:18** — Agent-aware hooks (multi-agent `ZELLAI_AGENT` env var), `[keybindings]` config section, keyboard event handling in plugin event loop

## Source Architecture

```
src/
├── lib.rs                  (342 lines)  — WASM plugin: ZellijPlugin trait, event loop, async I/O dispatch
├── status.rs               (258 lines)  — AgentStatus data model, JSON parsing, validation
├── status_bridge.rs        (378 lines)  — Session tracking: HashMap<session_id, AgentStatus>, stale detection
├── config.rs               (417 lines)  — ZellaiConfig TOML parsing with defaults
├── sidebar.rs              (731 lines)  — Sidebar rendering: compact/detailed/adaptive cards, box-drawing
├── attention.rs            (357 lines)  — AttentionTracker: priority rotation, dismiss, cursor cycling
├── teams.rs                (360 lines)  — Team layout generation (orchestrator-top/left/grid)
├── workspace.rs            (680 lines)  — Workspace data model + JSON persistence (cfg-gated I/O)
├── status_bar.rs           (189 lines)  — Single-line status bar segment
├── bin/
│   ├── zellai/
│   │   ├── main.rs         (227 lines)  — CLI entry point (clap), argv[0] wrapper detection
│   │   ├── run.rs          (290 lines)  — `zellai run`: spawn child + periodic status writes
│   │   ├── init.rs         (199 lines)  — `zellai init`: install Claude Code hooks
│   │   ├── doctor.rs       (332 lines)  — `zellai doctor`: 7 diagnostic checks
│   │   ├── status_writer.rs(571 lines)  — StatusWriter: atomic JSON file writes, git info collection
│   │   ├── teams_cmd.rs    (227 lines)  — `zellai teams`: launch Zellij multi-agent layout
│   │   └── workspace_cmd.rs(366 lines)  — `zellai new/list/kill/attach` commands
│   └── screenshot.rs       (122 lines)  — SVG screenshot generator (dev tool)
hooks/
├── on-stop.sh              (75 lines)   — Writes idle status then deletes file
├── on-notification.sh      (91 lines)   — Writes waiting status with needs_attention
└── on-post-tool-use.sh     (92 lines)   — Writes thinking status with tool name
```

**Architecture pattern:** Clean separation between pure-logic modules (testable on host target) and Zellij/IO-dependent code (WASM-only or native-only via `#[cfg]` gates). The async I/O pattern uses `run_command` with a `zellai_cmd` context key as a manual continuation-passing style.

## Open Issues Summary

**No open issues.** `gh issue list` returns an empty list. The project has no community-driven feature requests or bug reports pending.

## Gaps & Opportunities

### Unimplemented Features (defined in schema/vision but stubbed or missing)

1. **`jump_to` keybinding** — Parsed in config but never handled in `update()`. The `next_attention` keybinding cycles through sessions but never calls `focus_terminal_pane` (marked with a TODO comment at lib.rs:185).

2. **Port detection** — The `ports` field in AgentStatus is always `[]`. No port scanning or detection logic exists anywhere. The vision calls for "What ports its dev server is listening on."

3. **PR/CI status integration** — `pr_number`, `pr_ci_status`, and `CiStatus` are defined in the schema but never populated by StatusWriter or hooks. The vision explicitly includes "The linked pull request and its CI status via `gh` CLI."

4. **Custom team layouts** — `TeamsLayout::Custom` returns an empty Vec. The vision and config schema define `[[teams.layout]]` blocks for custom pane definitions.

5. **Attention animation** — The `attention_animation` config flag is parsed but never used. The sidebar renders static text only — no pulsing glow or visual animation exists.

6. **Error status** — The `AgentStatusValue::Error` variant is defined but never produced by any writer (hooks or StatusWriter). No code path generates an error status.

7. **Task Board** — The vision describes an "Orchestrator Task Board" with Kanban and DAG views. Not implemented at all.

8. **Broadcast mode / targeted messaging** — Vision describes sending prompts to all/specific agent panes. Not implemented.

9. **Per-pane execution logs** — Vision describes structured logs at `sessions/<workspace>/<pane>.log` and a `zellai log <pane>` command. Not implemented.

10. **ANSI colors in sidebar** — The sidebar uses plain text with Unicode icons. No terminal colors, no dimming of idle agents, no visual differentiation beyond status icons.

### Code Quality Gaps

11. **Code duplication** — `workspace_cmd::cmd_attach()` and `teams_cmd::cmd_teams()` share nearly identical Zellij tab/pane creation logic that should be extracted to a shared function.

12. **Unicode width handling** — Both sidebar and status bar use `chars().count()` for width, which is incorrect for CJK or wide Unicode characters. Should use a unicode-width crate.

13. **Dead code** — `AttentionTracker::clear_dismissed()` is defined but never called. `StatusBridge::session_ids()` is only used in tests.

14. **Race condition in pane creation** — `teams_cmd` and `workspace_cmd` use `zellij action write-chars` to type commands into panes after creating them, with no synchronization to ensure the pane is ready.

### Vision Features Not Yet Started

15. **Pipe bridge** — Listed as future in vision; no implementation.
16. **Session Messages view** — Listed as future in vision; orchestrator send/receive history.

## Bugs / Friction Found

1. **`on-stop.sh` writes then immediately deletes** — The hook writes an "idle" status file then immediately `rm -f`s it. The idle write is wasted work since the file is deleted in the same script invocation. The plugin's stale detection handles orphaned files from crashes, so the explicit idle write adds no value.

2. **`json_escape` in hooks is incomplete** — Only handles `\`, `"`, `\t`, and newlines. Does not handle other JSON control characters (e.g., `\b`, `\f`, or characters below U+0020). Unlikely to cause issues in practice but technically produces invalid JSON for edge-case inputs.

3. **`std::process::exit()` in run.rs** — Called after child process exits, which skips destructors. In practice cleanup happens before this call, but it's an antipattern.

4. **No integration tests** — All 133 tests are unit tests for pure logic. There are no tests that exercise the CLI subcommands end-to-end, or that verify the hooks produce valid JSON that the bridge can parse.

5. **`pr_number` and `pr_ci_status` not in StatusWriter JSON template** — The StatusWriter's JSON template omits these fields entirely. While they're optional in the schema, this means the `gh` CLI integration path has no writer-side support.
