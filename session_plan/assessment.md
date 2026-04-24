# Assessment — 2026-04-24

## Build Status

**WASM build:** ✅ pass (`cargo build --target wasm32-wasip1`)
**Unit tests:** ✅ 133 passed, 0 failed (`cargo test --lib`)
**Clippy (WASM):** ✅ clean — zero warnings
**Clippy (host):** ⚠ 3 warnings — `&PathBuf` should be `&Path` in `workspace.rs` (lines 237, 282, and one more)

## Project State

zellai is a mature implementation covering steps 1–9 of the Current Direction roadmap. All core features are built:

- **Plugin scaffold** (step 1) — `lib.rs` implements `ZellijPlugin` trait, compiles to WASM, registers plugin
- **Status bridge** (step 2) — `status.rs` (AgentStatus model), `status_bridge.rs` (StatusBridge reader with stale detection)
- **Sidebar plugin** (step 3) — `sidebar.rs` (compact/detailed/adaptive card rendering, attention indicators)
- **Claude Code hooks** (step 4) — `hooks/*.sh` (3 hook scripts), `init.rs` (`zellai init` installs hooks)
- **Generic wrapper** (step 5) — `run.rs` (`zellai run <command>`), `status_writer.rs` (atomic status file writes)
- **Workspace management** (step 6) — `workspace.rs` (templates, save/restore), `workspace_cmd.rs` (new/list/kill/attach)
- **Teams command** (step 7) — `teams.rs` (layout generation), `teams_cmd.rs` (`zellai teams`)
- **Status bar plugin** (step 8) — `status_bar.rs` (workspace summary segment), `lib.rs` has PluginMode::StatusBar
- **DX commands** (step 9) — `doctor.rs` (`zellai doctor`), shell completions via clap_complete

The plugin event loop handles Timer and RunCommandResult events for non-blocking status file reads. The codebase uses a clean dual-target architecture: WASM plugin (`cdylib`) + native CLI (`zellai-cli`), with pure-logic modules shared between both.

## Recent Changes (last 3 sessions)

1. **2026-04-24 04:48** — Pluralization fix ("1 agents" → "1 agent"), security patch for shell injection in hooks via `json_escape()`, SVG screenshot generation, README update
2. **2026-04-24 03:18** — Agent-aware hooks (`ZELLAI_AGENT` env var), `[keybindings]` config with `parse_key`, keyboard event handling in plugin event loop
3. **2026-04-23 17:14** — Status bar rendering mode, `zellai doctor` diagnostics, shell completions (bash/zsh/fish) via clap

Git log shows only 1 commit (shallow clone): `1ffcf1d Update YOYO.md`

## Source Architecture

```
src/
  lib.rs              342 lines  — ZellijPlugin impl (WASM-gated), module re-exports
  attention.rs        357 lines  — AttentionTracker (priority cycling, dismiss, idle)
  config.rs           417 lines  — ZellaiConfig + all sub-configs, TOML parsing
  sidebar.rs          731 lines  — Agent card rendering (compact/detailed/adaptive)
  status.rs           258 lines  — AgentStatus model, JSON parsing, validation
  status_bar.rs       189 lines  — Single-line status bar segment
  status_bridge.rs    378 lines  — In-memory agent session store, stale detection
  teams.rs            360 lines  — Team layout generation (orch-top/left/grid)
  workspace.rs        679 lines  — Workspace data model + file persistence
  bin/
    zellai/
      main.rs         173 lines  — CLI dispatch (clap, 9 subcommands)
      doctor.rs       332 lines  — Runtime diagnostics
      init.rs         199 lines  — Hook installation
      run.rs          209 lines  — Generic agent wrapper
      status_writer.rs 571 lines — Atomic status file I/O
      teams_cmd.rs    227 lines  — `zellai teams` CLI
      workspace_cmd.rs 366 lines — new/list/kill/attach CLI
    screenshot.rs     122 lines  — Sample data renderer
hooks/
  on-stop.sh           75 lines
  on-notification.sh   91 lines
  on-post-tool-use.sh  92 lines

Total: ~6,170 lines across 20 files
Tests: 133 unit tests across all modules
```

## Open Issues Summary

**No open issues.** (`gh issue list` returned `[]`)

The community hasn't filed any `agent-input` issues yet. Development is entirely vision-driven.

## Gaps & Opportunities

The entire 9-step roadmap is implemented. The remaining gaps are refinements and features from the founding vision that haven't been built yet:

### Not Yet Implemented (from zellai-vision.md)

1. **Named wrappers** — Vision specifies `zellai-codex`, `zellai-gemini`, `zellai-aider`, `zellai-opencode` as standalone wrapper commands. Currently only `zellai run <command>` exists. Agent detection is automatic, but no named binaries/symlinks are provided.

2. **Orchestrator Task Board** — Vision describes a dedicated pane view with Kanban columns (`todo | in-progress | review | done | blocked`) and DAG dependency tree (ASCII). Not implemented. This is a significant feature from the vision.

3. **Broadcast mode** — "Send the same prompt to all agent panes at once via Zellij pipes." Not implemented.

4. **Targeted message send** — "Send a structured message to a specific agent pane by index or name without switching focus." Not implemented.

5. **Per-pane execution log** — Vision specifies `~/.local/share/zellai/sessions/<workspace>/<pane>.log` and `zellai log <pane>` command. Not implemented.

6. **Keyboard navigation: focus toggle** — "Toggle focus between orchestrator Task Board pane and agent panes." Not wired (the TODO in lib.rs:185 confirms: session_id → pane ID mapping is missing).

7. **Workspace templates** — Data model exists (`SingleAgent`, `Team`, `Review`, `Research`), but `from_template` generates basic layouts. Vision mentions richer workspace templates.

8. **Port detection** — Status schema has `ports: Vec<u16>` but no mechanism detects listening ports. Hooks/wrappers write empty arrays.

9. **PR/CI status** — Schema supports `pr_number` and `pr_ci_status` via `gh` CLI, but no code calls `gh pr view` or `gh run list`. Fields are always null.

10. **Pipe bridge** — Listed as a future upgrade in the vision. Not started (correctly deferred per YOYO.md).

### Quality & Polish Opportunities

- **Clippy warnings** — 3 `&PathBuf` → `&Path` warnings in workspace.rs (host target)
- **Integration testing** — No end-to-end tests; only unit tests for pure logic
- **README "Coming soon"** — Development setup section still says "Coming soon — yoyo will scaffold the plugin in its first growth session" despite the plugin being fully built

## Bugs / Friction Found

1. **README stale text** — `README.md` line 23 says "Coming soon — yoyo will scaffold the plugin in its first growth session." This is outdated — the scaffold has existed for many sessions. The development setup instructions are already present below it, making the "coming soon" confusing.

2. **Clippy `&PathBuf` warnings (host)** — `workspace.rs` functions `load_workspace_from` and `delete_workspace_from` take `&PathBuf` instead of `&Path`. Minor but produces 3 warnings on every host-target clippy run.

3. **Port detection gap** — The status schema supports `ports` but nothing populates it. Users expecting port visibility (per the vision) will see empty arrays.

4. **PR/CI fields always null** — Same as ports: schema is ready, no code fills it. Users relying on `gh` integration will find nothing.

5. **Session ID → Pane ID mapping missing** — The TODO at `lib.rs:185` acknowledges this. Keyboard navigation (`next_attention`, `dismiss`) can cycle through attention state but can't actually focus the corresponding Zellij pane, limiting the practical value of keyboard shortcuts.

6. **No `zellai log` subcommand** — Vision specifies per-pane execution logs and a `zellai log <pane>` retrieval command. Neither the log writing infrastructure nor the CLI command exists.
