# zellai — Founding Vision

<!-- This file is the founding vision. Do not modify. -->

## What zellai Is

zellai is the agentplex for Linux — a Zellij plugin that gives developers a native, agent-aware terminal workspace for running multiple AI coding agents simultaneously. It brings the orchestration capabilities of cmux to every Linux terminal, without requiring a specific terminal application or operating system.

Where cmux is a macOS-only terminal app, zellai is a plugin: it runs inside the terminal you already use, on any Linux machine, over any SSH session, in any environment where Zellij runs.

## The Problem

AI coding agents have changed how developers work. A developer running Claude Code, Codex, or Gemini CLI is no longer just editing files — they are collaborating with a system that thinks, runs commands, and periodically needs human direction.

The problem: nothing was built for this on Linux. Developers managing multiple agent sessions do so with raw tmux or Zellij splits and zero visibility into what any agent is doing. There is no way to know at a glance which agent has finished, which one is stuck waiting, or which one just sent a notification. Every context switch requires reading the pane to understand agent state.

On macOS, cmux solved this. On Linux, there is nothing.

## The Core Experience

zellai wraps Zellij's existing pane system with an intelligence layer. Every pane running an AI agent gets metadata that zellai tracks and surfaces in a persistent sidebar:

- What agent is running and what it is currently doing
- Which git branch it is working on
- What directory it is in
- Whether it is actively thinking, waiting for input, or idle
- The most recent notification or status message it sent
- What ports its dev server is listening on
- The linked pull request and its CI status

When an agent needs attention — a question to answer, a tool confirmation, an error to review — zellai signals it visually. The developer sees which pane needs them without reading every terminal.

## The Agent Workspace

zellai introduces the concept of an **agent workspace**: a named, persistent layout of panes, each running one or more AI coding agents, with a shared sidebar showing the status of the whole team.

A workspace has:
- A name and a working directory
- A set of agent panes, each with its own task or context
- A sidebar showing all pane metadata at a glance
- An optional status bar segment showing a workspace-level summary

Workspaces can be saved and restored. Starting a new session with the same workspace name resumes the same layout with agents ready to launch.

## Multi-Agent Orchestration

`zellai teams` is the entry point for multi-agent work. It spawns a pre-configured workspace of agent panes — one command to go from zero to a full team of AI agents working in parallel.

The default team layout mirrors Claude Code's teammate mode: one orchestrator pane and multiple worker panes, each with its own task. But zellai is agent-agnostic: a team can mix Claude Code, Codex, Gemini CLI, Aider, or any CLI agent.

Custom team layouts are defined in a `zellai.toml` file, checked into the project, and shared with teammates.

## Agent Agnosticism

zellai supports any CLI AI coding agent. The foundation is a file-based status bridge: agents write status updates to a shared directory via hooks or a thin wrapper script. zellai reads these files and keeps the sidebar current.

Supported out of the box:
- **Claude Code** — via native hooks (Stop, Notification, PostToolUse); zero configuration
- **Codex** — via `zellai-codex` wrapper
- **Gemini CLI** — via `zellai-gemini` wrapper
- **Aider** — via `zellai-aider` wrapper
- **OpenCode** — via `zellai-opencode` wrapper
- **Any agent** — via `zellai run <command>`, the generic wrapper

Future: an in-band Zellij pipe bridge — wrapper scripts that route structured events through Zellij's native pipe system — for richer, lower-latency status without polling.

## What Makes zellai Different from cmux

| | cmux | zellai |
|---|---|---|
| Platform | macOS only | Linux, any terminal |
| Distribution | Native app (Homebrew) | Zellij plugin |
| Agent support | Claude Code primary | All CLI agents |
| Terminal required | Ghostty (bundled) | Any terminal running Zellij |
| Browser integration | Built-in | External (future) |
| SSH / remote | Via cmux app | Native (Zellij handles it) |
| Headless / CI | No | Yes |
| Extensible | Custom plugin API | Zellij plugin ecosystem |

zellai runs anywhere Zellij runs: local machines, remote servers over SSH, Docker containers, CI environments. There is no GUI requirement.

## The Full Feature Set

### Sidebar
- Per-pane agent status: thinking, waiting for input, idle, error
- Git branch and dirty state per pane
- Working directory per pane
- Latest agent notification or status message
- Listening ports per pane
- Linked PR number and CI status via `gh` CLI
- Visual attention ring when agent needs human input
- Collapsible to a minimal icon strip for small screens

### Workspace Management
- Named workspaces with saved pane layouts
- Workspace templates: single agent, team, review, research
- `zellai new <name>` — create and launch a workspace
- `zellai attach <name>` — attach to a running workspace
- `zellai list` — list all active workspaces and agent states
- `zellai kill <name>` — terminate a workspace and all its agents

### Multi-Agent Teams
- `zellai teams` — launch the default team layout for the current project
- `zellai.toml` — project-level team and workspace configuration
- Custom layouts: define pane count, agent type, and initial prompt per pane
- Orchestrator + worker topology
- Orchestrator Task Board (optional): dedicated pane view for task-level state across the active team
- Task Board views: Kanban (`todo | in-progress | review | done | blocked`) and dependency-aware DAG tree (ASCII, level-grouped)
- Task metadata: assigned pane identifier (index or name), git branch, last activity timestamp
- Aggregate Task Board stats: total tasks, success rate, optional cost/token consumption
- Task Board is configurable: dedicated pane, or disabled entirely
- DAG dependencies are surfaced as task relationships for visibility in the Task Board
- Dependency enforcement is delegated to the orchestrator agent, so tasks may execute before dependencies are complete
- Running tasks before dependencies complete may cause execution failures unless the orchestrator agent applies its own enforcement logic
- DAG view should explicitly indicate informational mode so users do not assume execution order is guaranteed
- Broadcast mode: send the same prompt to all agent panes at once via Zellij pipes
- Targeted message send: send a structured message to a specific agent pane by index or name without switching focus
- Future: session Messages view in orchestrator pane with send/receive history

### Status Bridge
- Claude Code native hook integration (Stop, Notification, PostToolUse)
- Generic wrapper: `zellai run <agent-command>`
- Named wrappers: `zellai-claude`, `zellai-codex`, `zellai-gemini`, `zellai-aider`
- Status schema: agent name, status, git branch, working directory, last message, ports, timestamp
- Status files written under `~/.local/share/zellai/sessions/` using the zellai workspace name as directory when known, otherwise a `session-<id>` directory
- Optional per-pane structured execution log (status events, developer interactions, and tool calls when surfaced by hooks)
- Execution logs written alongside status files at `~/.local/share/zellai/sessions/<workspace>/<pane>.log`
- `zellai log <pane>` for per-pane session log retrieval
- Future: in-band Zellij pipe bridge for event-driven, zero-polling updates

### Keyboard Navigation
- Jump to any agent pane by index or name
- Cycle through panes needing attention with a single key
- Dismiss notifications without switching panes
- Toggle focus between orchestrator Task Board pane and agent panes
- Workspace-level keybindings configurable in `zellai.toml`

### Status Bar Integration
- Zellij status bar plugin showing workspace summary
- Shows: workspace name, agent count, how many agents need attention
- Minimal footprint — a single status bar segment

### Developer Experience
- `zellai init` — configure hooks and generate `zellai.toml` for the current project
- Zero configuration for Claude Code users: hooks are detected and wired automatically
- Shell completions for bash, zsh, and fish
- `zellai doctor` — diagnose missing hooks, unsupported agents, and config issues

## UI Design

### Sidebar Layout

The sidebar is configurable: **left**, **right**, or **bottom strip**. The default is left.

Left and right modes render full agent cards with all metadata visible. Bottom strip mode renders a compact single-line entry per agent — name, status, branch — to minimise vertical footprint.

Users set their preferred position in `zellai.toml`:

```toml
[sidebar]
position = "left"   # left | right | bottom
```

### Agent Cards

Card density is configurable: **compact** (two lines), **detailed** (all fields), or **adaptive**. The default is adaptive.

In adaptive mode, agents needing attention automatically expand to show full detail — status message, PR/CI status, last notification, listening ports. Agents that are thinking or idle collapse to compact view. This keeps the sidebar focused on what matters without requiring manual configuration.

```toml
[sidebar]
card_density = "adaptive"   # compact | detailed | adaptive
```

### Attention Indicators

When an agent needs human input, zellai signals it with two simultaneous cues:

- **Badge dot** — a red dot appears on the card corner, like a mobile notification
- **Ambient glow** — the card emits a soft pulsing red glow
- **Idle dimming** — agents that are not waiting dim slightly, drawing the eye toward the active card

Both indicators are on by default. Animation can be disabled for users who prefer a static display:

```toml
[sidebar]
attention_animation = true   # set false to disable pulsing
```

### Teams Layout

`zellai teams` launches a pre-configured multi-agent workspace. The default layout is **orchestrator top**: the orchestrator pane spans the top portion of the screen, with worker panes split equally beneath it.

Alternative layouts — orchestrator left, equal grid — are available via `zellai.toml`. Custom layouts with any pane count and topology can be defined per project.

```toml
[teams]
default_layout = "orchestrator-top"   # orchestrator-top | orchestrator-left | equal-grid | custom
```

The orchestrator pane can optionally host a dedicated Task Board panel. The Task Board can be disabled entirely, or enabled with configurable columns and optional cost tracking:

```toml
[teams.orchestrator]
task_board = true
task_board_columns = ["todo", "in-progress", "review", "done", "blocked"]
show_cost_tracking = false
dag_view = true
```

Within this panel, users can switch between Kanban and DAG views via a configurable keybinding. DAG mode is an ASCII dependency tree. It is organized by dependency level, measured as tree depth from root tasks with no dependencies (for example: level 0 has no dependencies, level 1 depends on level 0). Entries are also grouped by status within each level. The view is optimized for terminal constraints rather than full graph rendering.

## What zellai Is Not

zellai is not a terminal emulator. It does not replace Ghostty, Alacritty, kitty, or any other terminal. It is a plugin that runs inside Zellij.

zellai is not a browser. It does not embed a web view or provide browser automation. That capability belongs to external tools such as agent-browser, which agents can launch independently.

zellai is not an AI agent. It does not run models, send prompts, or make decisions. It orchestrates and surfaces the state of agents that do.

zellai is not opinionated about which agent you use. No agent receives privileged treatment beyond what its native integration supports.

## Open Source and Community

zellai is open source. The founding commit contains this vision, a YOYO.md for agent-driven development, and an empty Rust/WASM plugin scaffold. From that base, yoyo grows the implementation session by session, guided by this vision and GitHub issues labeled `agent-input`.

The community steers what comes next. File an issue. The vision drives; issues refine.
