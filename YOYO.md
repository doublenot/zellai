# zellai

## What This Is

An implementation of the [zellai vision](zellai-vision.md) — a Zellij plugin (Rust/WASM) that gives Linux developers a native, agent-aware terminal workspace for running multiple AI coding agents simultaneously.

This project is being grown by [yoyo](https://github.com/yologdev/yoyo-evolve), a self-evolving coding agent. Every commit after the initial setup was made by yoyo, triggered by GitHub issues or on a scheduled cadence.

## Founding Vision

Read `zellai-vision.md` for the complete product vision. The core: a Zellij plugin that surfaces per-pane AI agent metadata (status, git branch, working dir, notifications, ports, PR/CI) in a configurable sidebar, with multi-agent team orchestration via `zellai teams`.

## Current Direction

Build zellai in this order:

1. **Plugin scaffold** — empty Zellij plugin that compiles to WASM, loads in Zellij, renders a placeholder pane
2. **Status bridge (reader)** — parse session status files from `<user-data-dir>/zellai/sessions/`; model the `AgentStatus` struct
3. **Sidebar plugin** — render agent cards in a Zellij pane; implement adaptive card density (compact / detailed); attention indicators (badge + glow + idle dimming)
4. **Claude Code hooks** — auto-detect and configure Claude Code hooks (Stop, Notification, PostToolUse) to write status files; implement `zellai init`
5. **Generic wrapper** — `zellai run <command>` wrapper that writes status files for any agent; named wrappers (`zellai-codex`, `zellai-gemini`, `zellai-aider`)
6. **Workspace management** — named workspaces, save/restore layouts; `zellai new`, `zellai attach`, `zellai list`, `zellai kill`
7. **Teams command** — `zellai teams` launches orchestrator-top layout; `zellai.toml` project config
8. **Status bar plugin** — minimal Zellij status bar segment showing workspace name + agent count
9. **DX commands** — `zellai doctor` for diagnostics; shell completions (bash, zsh, fish)

### Open Questions (community decides via issues)

- PR/CI status integration depth (polling interval, gh CLI dependency)?
- Pipe bridge upgrade path timeline?

## Tech Stack

- **Language**: Rust (stable, minimum version 1.84)
- **Compile target**: `wasm32-wasip1` (requires Rust ≥1.84; the older `wasm32-wasi` target was removed in 1.84)
- **Plugin framework**: [zellij-tile](https://crates.io/crates/zellij-tile) — the public Zellij plugin SDK (not `zellij-utils`, which is Zellij's internal daemon library and not for plugins)
- **Serialization**: `serde` + `serde_json` (status files), `toml` (config)
- **IPC**: file-based; status files at `<user-data-dir>/zellai/sessions/<session-id>.json`
- **Git integration**: shell out to `git` (branch, dirty state) and `gh` CLI (PR number, CI status); both are optional runtime deps — degrade gracefully if `gh` is absent
- **Testing**: `cargo test --lib` (unit tests for pure logic only); plugin-API-touching code is tested via `zellij plugin --` dev-load (cannot be unit-tested against the host target)

## Build & Test

```sh
cargo build --target wasm32-wasip1           # debug build
cargo build --target wasm32-wasip1 --release # release build
cargo test --lib                              # unit tests (pure logic only — no Zellij API types)
cargo clippy --target wasm32-wasip1          # lint
cargo fmt                                     # format

# Load plugin in a running Zellij session
zellij plugin -- target/wasm32-wasip1/debug/zellai.wasm

# Run with plugin host (standalone, no Zellij session required)
zellij plugin --configuration key=value -- file:target/wasm32-wasip1/debug/zellai.wasm
```

## Directory Structure

```
zellai-vision.md        # product vision
YOYO.md                 # this file (project context)
SCHEMA.md               # plugin architecture, status schema, config schema
src/
  main.rs               # plugin entry point; ZellijPlugin trait impl
  sidebar.rs            # sidebar rendering logic
  status_bridge.rs      # reads and parses session status files
  workspace.rs          # workspace save/restore logic
  config.rs             # zellai.toml parsing
  attention.rs          # attention indicator state
  teams.rs              # teams layout launcher
  wrappers/             # per-agent wrapper scripts
hooks/                  # Claude Code hook scripts
  on-stop.sh
  on-notification.sh
  on-post-tool-use.sh
docs/
  brainstorms/          # requirements docs from ce:brainstorm sessions
  plans/                # implementation plans from ce:plan sessions
.yoyo/
  journal.md            # running session notes
  learnings.md          # project-specific learnings
```

## How yoyo Works Here

- Each session: read `zellai-vision.md` for product direction, assess the codebase, identify the biggest gap
- Decide what to build next based on the Current Direction order above
- Factor in GitHub issues labeled `agent-input` if they align with the vision — but the vision drives
- Run `cargo build --target wasm32-wasip1 && cargo test --lib && cargo clippy --target wasm32-wasip1` after every change
- If builds break and can't be fixed in 3 attempts, revert with `git checkout -- .`
- Write session notes to `.yoyo/journal.md`
- Record project-specific learnings to `.yoyo/learnings.md`
- The git history IS the story — write clear commit messages

## Plugin-Specific Rules

These rules are mandatory. The yoyo evaluator agent should reject any diff that violates them.

**Always compile to `wasm32-wasip1`.** Never use `wasm32-unknown-unknown` — Zellij requires the WASI target. The correct toolchain command is `rustup target add wasm32-wasip1`.

**No std I/O in the plugin binary.** WASM plugins cannot use `std::fs`, `std::net`, or `std::process` directly. Use Zellij's plugin API for all host interactions: `run_command` for subprocesses, file events via subscription. For the status bridge: in `load()`, call `subscribe(&[EventType::FileSystemUpdate, EventType::Timer])`. The Zellij host fires `FileSystemUpdate` events when watched files change. There is no `watch_filesystem()` function — subscription is the mechanism.

**Status files are the only IPC.** The plugin communicates with the outside world exclusively through `<user-data-dir>/zellai/sessions/`. No sockets, no pipes, no shared memory in the initial implementation. The pipe bridge upgrade (Option C) is a future milestone — do not implement it prematurely.

**Never block the render loop.** All I/O (reading status files, shelling out to `git`/`gh`) must be non-blocking. Use `run_command` (async) for any subprocess call. The `render()` function must return immediately.

**`zellai.toml` is always optional.** Every config value must have a sensible default. The plugin must load and function correctly with no `zellai.toml` present.

**Sidebar position default is `left`.** Card density default is `adaptive`. Teams layout default is `orchestrator-top`. Attention animation default is `true`. These defaults must be preserved even if the config parsing changes.
