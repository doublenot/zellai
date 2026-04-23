Title: Add status bar rendering mode to the plugin
Files: src/lib.rs, src/status_bar.rs, src/sidebar.rs (read-only reference)
Issue: none

## Description

Implement step 8 from the roadmap: a status bar rendering mode for the plugin. Zellij status bar plugins use the same `ZellijPlugin` trait — they're just loaded into the bar area and render a single line. Rather than creating a separate WASM binary, add a `mode` configuration key to the existing plugin that switches between "sidebar" (default) and "status-bar" rendering.

### Configuration

The plugin detects its mode from the Zellij plugin configuration BTreeMap:

```
zellij plugin --configuration mode=status-bar -- file:target/wasm32-wasip1/debug/zellai.wasm
```

In `load()`, check `configuration.get("mode")`. Default is `"sidebar"`.

### Status bar rendering

Create `src/status_bar.rs` with a pure-logic function:

```rust
pub fn render_status_bar(agents: &[&AgentStatus], workspace_name: &str, cols: usize) -> String
```

The status bar segment shows: `workspace_name | N agents | M need attention`

Format: `⬡ workspace | 3 agents | 1⚠` (compact when cols is small)

If no agents are loaded yet: `⬡ zellai` (just the name).

The `⚠` count only appears when > 0 agents need attention. Use Unicode symbols for compactness.

### Plugin changes in `lib.rs`

Add a `mode: PluginMode` field to `ZellaiPlugin` (enum: `Sidebar`, `StatusBar`). Parse from configuration in `load()`.

In `render()`, dispatch based on mode:
- `Sidebar` → existing `sidebar::render_sidebar` (unchanged)
- `StatusBar` → `status_bar::render_status_bar`, print single line

### Module structure

- `src/status_bar.rs` — pure logic, no `zellij_tile` imports, fully unit-testable
- Add `pub mod status_bar;` to `src/lib.rs`
- Keep it simple: just one `render_status_bar` function + tests

### Unit tests for `status_bar.rs`

- Empty agents → shows just workspace name
- Multiple agents, none needing attention → shows count, no warning
- Some agents needing attention → shows warning count
- Very narrow cols → truncation behavior
- Workspace name included in output

### Verification

```sh
cargo build --target wasm32-wasip1 && cargo clippy --target wasm32-wasip1 && cargo test --lib
```

Make sure existing sidebar tests still pass. The new `status_bar` module tests should cover the rendering logic.
