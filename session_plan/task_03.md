Title: Add attention state tracking module (attention.rs)
Files: src/attention.rs, src/lib.rs
Issue: none

## Description

Create the `attention.rs` module specified in SCHEMA.md. This module tracks which agent sessions need attention and provides keyboard-navigation primitives (`next_attention`, `dismiss`). The sidebar already renders attention indicators visually, but there's no state tracking for cycling through them or dismissing them.

### Module specification (from SCHEMA.md)

> ### `src/attention.rs` — Attention State
> Owns the cross-component attention state.
> - Tracks which sessions have `needs_attention == true`
> - Provides `next_attention()` for keyboard navigation cycling
> - Provides `dismiss(session_id)` to clear attention for a pane

### Design

Create `src/attention.rs` with:

```rust
/// Tracks attention state across agent sessions for keyboard navigation.
pub struct AttentionTracker {
    /// Ordered list of session IDs needing attention (maintained externally).
    attention_ids: Vec<String>,
    /// Index into `attention_ids` for cycling. None = no selection.
    cursor: Option<usize>,
    /// Set of dismissed session IDs (user explicitly dismissed attention).
    dismissed: HashSet<String>,
}
```

**Methods:**

1. `pub fn new() -> Self` — empty tracker

2. `pub fn update(&mut self, agents: &[&AgentStatus])` — rebuild the attention list from current agent state. Filter to agents where `needs_attention == true` AND not in `dismissed` set. Preserve cursor position if the currently-selected session is still in the list. Sort by session_id for stable ordering.

3. `pub fn next_attention(&mut self) -> Option<&str>` — advance cursor to the next session needing attention, wrapping around. Returns the session_id or None if no sessions need attention.

4. `pub fn current(&self) -> Option<&str>` — return the currently-selected session_id without advancing.

5. `pub fn dismiss(&mut self, session_id: &str)` — add session_id to the dismissed set. Remove it from attention_ids. Adjust cursor if needed.

6. `pub fn clear_dismissed(&mut self)` — clear all dismissals (useful when agent state changes significantly).

7. `pub fn attention_count(&self) -> usize` — number of sessions currently needing attention (excluding dismissed).

8. `pub fn is_dismissed(&self, session_id: &str) -> bool` — check if a session has been dismissed.

### Integration in lib.rs

- Add `mod attention;` (uncomment the existing commented-out line)
- Add `attention: attention::AttentionTracker` field to `ZellaiPlugin`
- In `handle_run_command_result` for `read_status`, after updating the bridge, call `self.attention.update(&self.bridge.agents_sorted())`
- Do NOT wire up keybindings yet — that's a future task. Just have the module available.

### Tests

Write thorough unit tests in `attention.rs`:

1. `test_empty_tracker` — new tracker has no attention, next returns None
2. `test_update_adds_attention` — agents with needs_attention=true appear
3. `test_update_excludes_non_attention` — agents with needs_attention=false don't appear
4. `test_next_cycles` — calling next repeatedly cycles through all attention sessions
5. `test_next_wraps_around` — cursor wraps from last to first
6. `test_dismiss_removes_session` — dismissed session is excluded
7. `test_dismiss_adjusts_cursor` — cursor adjusts when dismissed session was before cursor
8. `test_clear_dismissed` — clearing dismissed brings sessions back on next update
9. `test_update_preserves_cursor` — if current session still needs attention, cursor stays

### Verification

```sh
cargo build --target wasm32-wasip1 && cargo clippy --target wasm32-wasip1 && cargo test --lib
```

All existing 58 tests must continue to pass. New attention tests must pass.
