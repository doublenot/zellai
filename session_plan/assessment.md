# Assessment — 2026-04-22

## Build Status

**All green.**
- `cargo build --target wasm32-wasip1` — ✅ pass (with warnings, see Bugs section)
- `cargo test --lib` — ✅ 75 tests pass, 0 failures
- `cargo clippy --target wasm32-wasip1` — ✅ clean, no warnings

## Project State

The project has completed **milestones 1–4** of the Current Direction in YOYO.md:

1. **Plugin scaffold** ✅ — `ZellijPlugin` trait impl compiles to `wasm32-wasip1`, registers with `register_plugin!`, handles `load/update/render` lifecycle.
2. **Status bridge (reader)** ✅ — `StatusBridge` manages agent sessions in memory; plugin wires `Timer` → `ls` → `cat` async chain via `run_command` context maps.
3. **Sidebar plugin** ✅ — Full sidebar renderer with compact/detailed/adaptive card density, Unicode box-drawing borders, attention icons, truncation, and proper width handling.
4. **Claude Code hooks** ✅ — Three hook scripts (`on-stop.sh`, `on-notification.sh`, `on-post-tool-use.sh`) with atomic writes. `zellai init` CLI command detects `.claude/` and installs hooks.

The dual-target architecture is working: `cdylib` for the WASM plugin, `rlib` + `[[bin]]` for the native CLI.

## Recent Changes (last 3 sessions)

All development happened on 2026-04-21 in a single growth burst (4 journal entries), followed by a wrap-up commit:

| Session | What landed |
|---------|------------|
| 2026-04-21 15:36 | Foundation: `config.rs`, `status.rs`, plugin scaffold in `lib.rs` |
| 2026-04-21 18:45 | `StatusBridge` data layer, Timer/RunCommand event loop, sidebar renderer |
| 2026-04-21 19:34 | `AttentionTracker` module, stale detection wiring, Claude Code hook scripts |
| 2026-04-21 22:07 | CLI binary scaffold (`src/bin/zellai/`), `zellai init` command |

Git history: single squashed commit `3e91bb0 yoyo: growth session wrap-up` on `main`.

## Source Architecture

```
src/
  lib.rs              (217 lines)  — Plugin entry point; ZellijPlugin trait; event dispatch
  sidebar.rs          (731 lines)  — Sidebar renderer; compact/detailed/adaptive cards
  status_bridge.rs    (378 lines)  — In-memory session store; parse/stale/retain logic
  attention.rs        (357 lines)  — Attention cycling, dismissal, cursor tracking
  config.rs           (264 lines)  — ZellaiConfig with serde defaults; TOML parsing
  status.rs           (258 lines)  — AgentStatus model; JSON parsing; validation
  bin/zellai/
    main.rs           ( 36 lines)  — CLI entry point; clap with Init subcommand
    init.rs           (131 lines)  — `zellai init`; hook file installation

hooks/
  on-stop.sh          ( 61 lines)  — Writes idle status, then deletes file
  on-notification.sh  ( 79 lines)  — Writes waiting status with notification text
  on-post-tool-use.sh ( 80 lines)  — Writes thinking status with tool name

Total Rust: 2,372 lines (including ~1,050 lines of tests)
Total Shell: 220 lines
```

## Open Issues Summary

No open issues on doublenot/zellai. Community hasn't filed any requests yet.

## Gaps & Opportunities

Based on the YOYO.md Current Direction roadmap, the **next milestone is #5: Generic wrapper**.

| Milestone | Status | Notes |
|-----------|--------|-------|
| 5. Generic wrapper | **Not started** | `zellai run <command>` + named wrappers (`zellai-codex`, `zellai-gemini`, `zellai-aider`) |
| 6. Workspace management | Not started | `zellai new/attach/list/kill`; workspace save/restore |
| 7. Teams command | Not started | `zellai teams`; `zellai.toml` project config |
| 8. Status bar plugin | Not started | Minimal Zellij status bar segment |
| 9. DX commands | Not started | `zellai doctor`; shell completions |

**Missing modules** referenced in YOYO.md directory structure but not yet created:
- `src/workspace.rs` — workspace save/restore
- `src/teams.rs` — teams layout launcher
- `src/wrappers/` — per-agent wrapper logic
- `docs/brainstorms/` — empty
- `docs/plans/` — empty

**Feature gaps within existing code:**
- The plugin doesn't handle keyboard input events yet (`Key` events not subscribed). The `AttentionTracker` has `next_attention()` and `dismiss()` but they're never called from the event loop.
- No keybinding support (the `[keybindings]` config section from SCHEMA.md isn't modeled in `config.rs`).
- The `sessions_dir` uses `~` which won't be expanded by `ls` in the `run_command` call — may fail at runtime when the plugin actually runs in Zellij.
- No `watch_filesystem()` is called with a specific path (it's called with no args after permission grant) — may not watch the sessions directory specifically.

## Bugs / Friction Found

### 1. Output filename collision warning (medium severity)
```
warning: output filename collision at target/wasm32-wasip1/debug/zellai.wasm
  the bin target `zellai` and the lib target `zellai` have the same output filename
  this may become a hard error in the future
```
Both the `[[bin]]` and `[lib]` produce `zellai.wasm` when building for WASM. This means the bin target overwrites the lib target (or vice versa). This will likely become a hard error in a future Rust release. **Fix:** rename the bin target (e.g., `name = "zellai-cli"`) or add a `#[cfg(not(target_arch = "wasm32"))]` gate to prevent the bin from being compiled for WASM. The intended workflow is `cargo build --target wasm32-wasip1 --lib` for the plugin and `cargo build` (host) for the CLI, but an unqualified `cargo build --target wasm32-wasip1` tries to build both.

### 2. Tilde `~` in default sessions_dir may not expand (low-medium severity)
The default `sessions_dir` is `~/.local/share/zellai/sessions`. When passed to `run_command(&["ls", "-1", &sessions_dir])`, the `~` is a literal character — `ls` won't expand it. This would silently fail (exit code != 0) and the bridge would never see any sessions. The hook scripts handle this correctly (bash expands `~`), but the plugin doesn't.

### 3. No `docs/` directory yet (cosmetic)
YOYO.md references `docs/brainstorms/` and `docs/plans/` but neither directory exists.

### 4. Hook scripts don't handle JSON special chars in `working_dir` (edge case)
The hook scripts embed `$working_dir` directly in JSON without escaping. Paths with quotes or backslashes would produce invalid JSON. Unlikely in practice but violates robustness.
