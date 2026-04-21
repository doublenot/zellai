Title: Sidebar rendering — agent cards with density modes
Files: src/sidebar.rs, src/lib.rs
Issue: none

## Description

Build `src/sidebar.rs` as a **pure-logic module** (no `zellij_tile` imports) that renders agent cards as strings. Then wire it into `lib.rs`'s `render()` method.

### What to build in `src/sidebar.rs`

1. **`render_sidebar(agents: &[&AgentStatus], config: &SidebarConfig, rows: usize, cols: usize) -> Vec<String>`**
   - Returns a Vec of strings, one per row, to be printed by the plugin's render() function
   - Each string should be exactly `cols` characters wide (padded/truncated)
   - Renders: top border, title bar, agent cards, empty fill space, bottom border

2. **Title bar:**
   - `"╭─ zellai ─...─╮"` — box-drawing top border with title

3. **Agent card rendering — two modes:**

   **Compact mode** (1 line per agent):
   - `"│ ● claude [feat/auth] thinking │"` — status dot, agent name, git branch, status
   - Status dot colors (using ANSI where the terminal supports it, but output plain text for now — Zellij plugin rendering uses `print!`):
     - Thinking → `◉` (filled circle)
     - Waiting → `⚠` (attention — this agent needs input)
     - Idle → `○` (hollow circle)  
     - Error → `✗` (cross)

   **Detailed mode** (3 lines per agent):
   - Line 1: `"│ ◉ claude — thinking │"`
   - Line 2: `"│   feat/auth ● /home/user/app │"` (branch + working dir, truncated)
   - Line 3: `"│   Reading src/auth.ts… │"` (last_message, truncated)

   **Adaptive mode** (default):
   - Calculate how many agents fit in available rows
   - If all agents fit in detailed mode (3 rows each + borders): use detailed
   - If all agents fit in compact mode (1 row each + borders): use compact
   - Otherwise: show `needs_attention` agents in detailed, rest in compact
   - This is the key UX insight from the vision: attention-needing agents get more space

4. **Empty state:**
   - When `agents` is empty, render `" No agents connected "` centered

5. **Bottom border:**
   - `"╰─...─╯"` — box-drawing bottom border

6. **Helper functions (pure, testable):**
   - `fn render_compact_card(agent: &AgentStatus, width: usize) -> String`
   - `fn render_detailed_card(agent: &AgentStatus, width: usize) -> Vec<String>` (returns 3 lines)
   - `fn status_icon(status: &AgentStatusValue) -> &'static str`
   - `fn truncate_with_ellipsis(s: &str, max_len: usize) -> String`
   - `fn choose_density(agent_count: usize, attention_count: usize, available_rows: usize, config_density: &CardDensity) -> ResolvedDensity`

7. **`ResolvedDensity` enum:**
   - `AllCompact`
   - `AllDetailed`  
   - `Mixed` — attention agents detailed, rest compact

### Wire into `src/lib.rs`

- Add `pub mod sidebar;` 
- In `render()`: call `sidebar::render_sidebar(...)` with the agents from the bridge and config, then `println!` each returned line

### Unit tests (in sidebar.rs)

Write tests for:
- `status_icon` returns correct icon for each status value
- `truncate_with_ellipsis` truncates long strings with "…", leaves short strings alone
- `render_compact_card` produces a properly formatted single line
- `render_detailed_card` produces 3 lines
- `choose_density` picks AllDetailed when space allows
- `choose_density` picks AllCompact when tight
- `choose_density` picks Mixed when some agents need attention and space is limited
- `render_sidebar` with empty agents shows "No agents connected"
- `render_sidebar` with agents renders correct number of rows

### Verification

```sh
cargo build --target wasm32-wasip1 && cargo clippy --target wasm32-wasip1 && cargo test --lib
```

All existing tests (16 from config + status, plus new ones from task_01 status_bridge) must pass. New sidebar tests must pass.
