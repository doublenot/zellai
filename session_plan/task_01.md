Title: Status bridge — pure data management layer
Files: src/status_bridge.rs, src/lib.rs
Issue: none

## Description

Build `src/status_bridge.rs` as a **pure-logic module** (no `zellij_tile` imports) that manages the set of active agent sessions. This is the data layer that sits between file I/O events (handled by `lib.rs`) and rendering (handled by `sidebar.rs` in a future task).

### What to build

Create `src/status_bridge.rs` with:

1. **`StatusBridge` struct** containing:
   - `agents: HashMap<String, AgentStatus>` — keyed by session_id
   - `stale_threshold_s: u64` — from config, default 60
   - `sessions_dir: String` — from config, default `~/.local/share/zellai/sessions`

2. **`StatusBridge::new(sessions_dir: &str, stale_threshold_s: u64) -> Self`** — constructor

3. **`StatusBridge::update_from_json(&mut self, session_id: &str, json: &str) -> Result<(), String>`**
   - Calls `parse_status(json)` from `status.rs`
   - Stores/replaces the entry in `agents`
   - Returns `Err` with a description on parse failure (don't panic)

4. **`StatusBridge::remove_session(&mut self, session_id: &str)`**
   - Removes a session from the map (called when a status file is deleted)

5. **`StatusBridge::mark_stale(&mut self, now_epoch: u64)`**
   - Iterates all agents; if `is_stale(now_epoch, self.stale_threshold_s)` is true, sets `status` to `Idle` and `needs_attention` to `false`

6. **`StatusBridge::agents_sorted(&self) -> Vec<&AgentStatus>`**
   - Returns agents sorted: `needs_attention == true` first, then by session_id alphabetically
   - This is what the sidebar renderer will consume

7. **`StatusBridge::session_ids(&self) -> Vec<String>`**
   - Returns all tracked session IDs (used by the plugin to know which files to re-read)

8. **`StatusBridge::has_agents(&self) -> bool`**
   - Convenience: returns whether any agents are tracked

### Wire into lib.rs

- Add `pub mod status_bridge;` to `src/lib.rs` (uncomment/add the module declaration)
- This task does NOT change any event handling in `lib.rs` — that's task 02

### Unit tests (in status_bridge.rs)

Write tests for:
- `update_from_json` with valid JSON → agent appears in map
- `update_from_json` with invalid JSON → returns Err, map unchanged
- `update_from_json` replaces existing session
- `remove_session` removes an agent
- `remove_session` on non-existent session is a no-op
- `mark_stale` transitions old agents to Idle
- `mark_stale` leaves fresh agents unchanged
- `agents_sorted` returns needs_attention agents first
- `has_agents` returns false when empty, true when populated

### Verification

```sh
cargo build --target wasm32-wasip1 && cargo clippy --target wasm32-wasip1 && cargo test --lib
```

All existing tests must continue to pass. New tests in `status_bridge.rs` must pass.
