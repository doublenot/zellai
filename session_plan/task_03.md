Title: Prune AttentionTracker dismissed set and StatusBridge cleanup
Files: src/attention.rs, src/lib.rs
Issue: none

## Goal

Fix unbounded memory growth in `AttentionTracker::dismissed` (assessment Gap #13). When sessions are removed from `StatusBridge` via `retain_sessions`, the dismissed set in `AttentionTracker` retains references to those dead sessions forever. Over long-running sessions with many agents starting and stopping, this leaks memory.

## What to do

### 1. Add `prune_dismissed` method to `AttentionTracker` (src/attention.rs)

Add a method that takes a list of active session IDs and removes any dismissed entries that are no longer tracked:

```rust
/// Remove entries from the dismissed set for sessions that no longer exist.
/// Call after `StatusBridge::retain_sessions` to bound memory usage.
pub fn prune_dismissed(&mut self, active_session_ids: &[&str]) {
    self.dismissed.retain(|id| active_session_ids.contains(&id.as_str()));
}
```

### 2. Add tests for `prune_dismissed` (src/attention.rs)

Add these test cases:

- **test_prune_dismissed_removes_dead_sessions**: Dismiss agent-b, then prune with only agent-a active → agent-b should be removed from dismissed set.
- **test_prune_dismissed_keeps_active_sessions**: Dismiss agent-a, then prune with agent-a still active → agent-a stays in dismissed set.
- **test_prune_dismissed_empty_active_clears_all**: Prune with empty active list → dismissed set should be empty.
- **test_prune_dismissed_no_op_when_empty**: Prune when dismissed is empty → no panic, no change.

### 3. Call `prune_dismissed` from plugin after `retain_sessions` (src/lib.rs)

In the `list_sessions` command handler (around line 339), after `self.bridge.retain_sessions(&session_ids)`, add:

```rust
self.attention.prune_dismissed(&session_ids);
```

This ensures dismissed entries are cleaned up every polling cycle when the directory listing returns.

### 4. Also prune when `remove_session` is called (src/lib.rs)

Check if there are any other places where sessions are removed from the bridge. If `remove_session` is called individually, the dismissed set should also be notified. However, looking at the code, `retain_sessions` is the main cleanup path, so the single call site should suffice.

## Verification

```sh
cargo build --target wasm32-wasip1 && cargo clippy --target wasm32-wasip1 && cargo test --lib
```

All 204+ existing tests must pass. The 4 new tests must pass. The `prune_dismissed` call in `lib.rs` must compile under `wasm32-wasip1`.
