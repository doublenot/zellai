# Assessment — 2026-04-25

## Build Status

All green:
- `cargo build --target wasm32-wasip1` — **pass** (clean, no warnings)
- `cargo test --lib` — **pass** (162 tests, 0 failures)
- `cargo clippy --target wasm32-wasip1` — **pass** (no warnings)

## Project State

zellai is a mature implementation covering steps 1–9 of the Current Direction roadmap. The WASM plugin compiles and implements the full sidebar + status bar rendering pipeline. A native CLI binary provides workspace management, teams orchestration, hook installation, agent wrapping, diagnostics, and shell completions.

**What exists:**

| Roadmap Step | Status | Key Files |
|---|---|---|
| 1. Plugin scaffold | ✅ Done | `src/lib.rs` (ZellijPlugin trait impl) |
| 2. Status bridge | ✅ Done | `src/status.rs`, `src/status_bridge.rs` |
| 3. Sidebar plugin | ✅ Done | `src/sidebar.rs` (1460 lines — compact/detailed/adaptive cards) |
| 4. Claude Code hooks | ✅ Done | `hooks/*.sh` (258 lines), `src/bin/zellai/init.rs` |
| 5. Generic wrapper | ✅ Done | `src/bin/zellai/run.rs`, `status_writer.rs`, named wrappers |
| 6. Workspace management | ✅ Done | `src/workspace.rs`, `src/bin/zellai/workspace_cmd.rs` |
| 7. Teams command | ✅ Done | `src/teams.rs`, `src/bin/zellai/teams_cmd.rs` |
| 8. Status bar plugin | ✅ Done | `src/status_bar.rs` |
| 9. DX commands | ✅ Done | `src/bin/zellai/doctor.rs`, shell completions via clap |

**Plugin event loop:** `lib.rs` subscribes to `Timer` and `RunCommandResult` events. The timer fires `ls` on the sessions directory, then reads each JSON file via chained `run_command` calls. Keyboard events (`Key`) drive attention cycling and dismiss. Two rendering modes: sidebar (default) and status-bar (via `mode=status-bar` config key).

## Recent Changes (last 3 sessions)

From journal entries (git log shows a single squashed commit):

1. **2026-04-24 13:44** — PR/CI status and ports in sidebar cards. Extended `StatusWriter` to shell out to `gh pr view` / `gh pr checks`. Detailed card renderer now shows ports and CI status with colored icons.

2. **2026-04-24 12:55** — Named wrapper binaries (`zellai-codex`, `zellai-gemini`, `zellai-aider`) via argv[0] detection. Clippy cleanup. Screenshot SVG regenerated.

3. **2026-04-24 04:48** — Pluralization fix ("1 agents" → "1 agent"), security patch for shell injection in hook scripts, unit tests for `zellai init`, SVG screenshot embedded in README.

## Source Architecture

```
src/                          Total: ~6,296 lines of Rust
├── lib.rs              342   Plugin entry point, ZellijPlugin trait impl, event dispatch
├── sidebar.rs         1460   Sidebar renderer (compact/detailed/adaptive cards, ANSI styling)
├── status.rs           258   AgentStatus model, parsing, staleness
├── status_bridge.rs    378   StatusBridge — session state manager
├── status_bar.rs       265   Minimal status bar segment renderer
├── config.rs           417   ZellaiConfig parsing (TOML), defaults, keybindings
├── attention.rs        357   AttentionTracker — priority rotation, dismissal
├── workspace.rs        680   Workspace model, templates, save/load/list/delete
├── teams.rs            360   KDL layout generation for teams
├── bin/
│   ├── zellai/
│   │   ├── main.rs          227   CLI entry point (clap), 11 subcommands
│   │   ├── init.rs          199   Hook installation for Claude Code
│   │   ├── run.rs           290   Generic agent wrapper
│   │   ├── status_writer.rs 816   Status file writer for wrappers
│   │   ├── workspace_cmd.rs 366   new/list/kill/attach commands
│   │   ├── teams_cmd.rs     227   teams subcommand
│   │   └── doctor.rs        332   Environment diagnostics
│   └── screenshot.rs        122   SVG screenshot generator

hooks/                       258 lines total
├── on-stop.sh           75   Writes idle status, deletes file on clean exit
├── on-notification.sh   91   Writes needs_attention + last_message
└── on-post-tool-use.sh  92   Updates status to "thinking" with tool name

docs/
├── screenshot.py       187   Python SVG screenshot generator
└── screenshot.svg            Current plugin screenshot
```

## Open Issues Summary

No open issues. The issue tracker at `doublenot/zellai` is empty.

## Gaps & Opportunities

All 9 roadmap steps are implemented. The remaining gaps are relative to the **full feature set** described in `zellai-vision.md`:

### Not Yet Implemented (from vision)

1. **Orchestrator Task Board** — The vision describes a dedicated pane view for task-level state (Kanban columns: todo/in-progress/review/done/blocked, DAG dependency tree, aggregate stats, cost tracking). Not implemented at all. This is a significant feature in the vision under "Multi-Agent Teams."

2. **Broadcast mode** — "Send the same prompt to all agent panes at once via Zellij pipes." Not implemented.

3. **Targeted message send** — "Send a structured message to a specific agent pane by index or name without switching focus." Not implemented.

4. **Per-pane execution logs** — The vision specifies `~/.local/share/zellai/sessions/<workspace>/<pane>.log` and `zellai log <pane>` CLI command. Not implemented.

5. **`zellai log <pane>`** — CLI command for per-pane session log retrieval. Not implemented.

6. **Workspace templates beyond basic** — Vision lists "single agent, team, review, research" templates. `workspace.rs` has template support but the actual template definitions may need enrichment.

7. **Bottom strip sidebar mode** — The vision describes a "bottom strip mode" that "renders a compact single-line entry per agent." The sidebar supports left/right/bottom position config, but bottom strip rendering may not have the single-line-per-agent layout described.

8. **Jump-to-pane by index/name** — Vision mentions "jump to any agent pane by index or name." The `jump_to` keybinding is in the config schema but may not be wired up in the plugin event loop.

9. **Pipe bridge upgrade** — Listed as a future milestone; intentionally deferred per project rules.

### Quality & Polish Opportunities

10. **End-to-end integration testing** — No integration tests exist. The plugin can only be tested by loading in Zellij. A smoke test script that writes status files and verifies sidebar output would add confidence.

11. **README completeness** — README could document all CLI commands with examples, installation instructions, and the screenshot.

12. **Release packaging** — No release workflow, no WASM binary distribution, no `cargo install` readiness.

## Bugs / Friction Found

1. **No bugs in build/test** — All 162 tests pass, clippy is clean, WASM build succeeds.

2. **Single squashed commit** — Git history shows only one commit (`8c240e8 yoyo: growth session wrap-up`). The journal documents 12+ sessions of work, but the commit history doesn't reflect the incremental story. This makes bisecting or understanding evolution impossible.

3. **`sidebar.rs` is large (1460 lines)** — Contains rendering logic, ANSI helpers, density selection, card formatting, and metadata line rendering. Could benefit from splitting into sub-modules (e.g., `sidebar/cards.rs`, `sidebar/ansi.rs`).

4. **`status_writer.rs` is large (816 lines)** — Contains writer logic, agent detection, session ID generation, path resolution, and many tests. Could be split.

5. **No `zellai.toml` example file** — The config schema is documented in SCHEMA.md but there's no example file users can copy and customize.

6. **`edition = "2024"` in Cargo.toml** — Uses the 2024 edition which requires Rust 1.85+. The YOYO.md says "minimum version 1.84." This is a minor documentation inconsistency (the code works with current Rust but the stated minimum is wrong).
