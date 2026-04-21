Title: Wire up mark_stale and session cleanup in plugin event loop
Files: src/lib.rs, src/status_bridge.rs
Issue: none

## Description

Fix two functional bugs identified in the assessment:

1. **`mark_stale` is never called** — `StatusBridge::mark_stale(now_epoch)` exists and is tested but the plugin's timer handler never invokes it. Stale agents remain in their last known status forever.

2. **Session cleanup for disappeared files** — When a status file is deleted, the session stays in the bridge map until the next `cat` fails. A more robust approach: compare `ls` results against known sessions and remove entries whose files are gone.

### Bug 1: Wire up mark_stale

The challenge: WASM plugins can't use `std::time::SystemTime`. We need the current epoch timestamp to call `mark_stale()`.

**Solution**: Add a new `run_command` call to `date +%s` in the timer handler, with context `"zellai_cmd": "get_time"`. When the result arrives, parse the stdout as a u64 and call `self.bridge.mark_stale(now_epoch)`.

**Implementation in `src/lib.rs`**:

In the `Event::Timer` handler, after the `list_sessions` command, also issue:
```rust
run_command(
    &["date", "+%s"],
    BTreeMap::from([("zellai_cmd".to_string(), "get_time".to_string())]),
);
```

Add a new match arm in `handle_run_command_result`:
```rust
"get_time" => {
    if exit_code == Some(0) {
        let stdout_str = String::from_utf8_lossy(&stdout);
        if let Ok(epoch) = stdout_str.trim().parse::<u64>() {
            self.bridge.mark_stale(epoch);
            return true; // re-render to reflect staleness changes
        }
    }
    false
}
```

### Bug 2: Session cleanup via retain_sessions

**Add `retain_sessions` method to `StatusBridge`** in `src/status_bridge.rs`:

```rust
/// Remove tracked sessions whose IDs are not in the given set.
/// Call this after `ls` to clean up sessions whose files have been deleted.
pub fn retain_sessions(&mut self, active_ids: &[&str]) -> usize {
    let before = self.agents.len();
    self.agents.retain(|k, _| active_ids.contains(&k.as_str()));
    before - self.agents.len()
}
```

**Wire into `list_sessions` handler in `src/lib.rs`**:

After processing the `ls` output, collect the session IDs from filenames and call `retain_sessions`. Modify the `list_sessions` match arm to:

1. Collect all `.json` filenames from `ls` output into a `Vec<&str>` of session IDs (strip `.json` suffix)
2. Call `self.bridge.retain_sessions(&session_ids)` to remove any sessions not in the current listing
3. Still issue `cat` commands for each file as before

### Tests

Add unit tests for `retain_sessions` in `src/status_bridge.rs`:
- Test that sessions not in the active set are removed
- Test that sessions in the active set are kept
- Test with empty active set removes all sessions
- Test with empty bridge is a no-op

### Verification

```sh
cargo build --target wasm32-wasip1 && cargo clippy --target wasm32-wasip1 && cargo test --lib
```

All existing tests must continue to pass. New tests for `retain_sessions` must pass.
