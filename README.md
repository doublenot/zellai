# zellai

The agentplex for Linux — a Zellij plugin that gives developers a native, agent-aware terminal workspace for running multiple AI coding agents simultaneously.

**[Read the product vision →](zellai-vision.md)**

---

## How It's Built

This codebase is grown by [yoyo](https://github.com/yologdev/yoyo-evolve), a self-evolving coding agent. Every commit after the initial setup was made by yoyo, triggered by GitHub issues or on a scheduled cadence.

| | |
|-|-|
| **Growth journal** | [.yoyo/journal.md](.yoyo/journal.md) |
| **What it learned** | [.yoyo/learnings.md](.yoyo/learnings.md) |
| **Latest session** | [GitHub Actions](../../actions/workflows/grow.yml) |

---

## Development Setup

> Coming soon — yoyo will scaffold the plugin in its first growth session.

Once the plugin exists:

```bash
git clone https://github.com/doublenot/zellai.git
cd zellai
cargo build --target wasm32-wasip1
```

Load the plugin in Zellij for development:

```bash
zellij plugin -- target/wasm32-wasip1/debug/zellai.wasm
```

---

## Steer It

File an issue labeled `agent-input` to suggest a feature. The agent reads open issues each session and factors them in if they align with the product vision.
