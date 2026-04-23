Title: Add signal handling to `zellai run` and fix redundant agent detection
Files: Cargo.toml, src/bin/zellai/run.rs
Issue: none

## Problem

Two bugs from the assessment:

1. **No signal handling in `zellai run`** (Bug #1): When the user hits Ctrl-C, the wrapper process is killed without writing a final "idle" status or cleaning up the status file. The stale detection eventually catches it after 60 seconds, but during that window the sidebar shows a "thinking" agent that is actually dead. This is confusing UX.

2. **Redundant agent detection** (Bug #2): In `run.rs` lines 72-76, `detect_agent` is called again on the command name even though the `agent` variable was already resolved earlier (line 31-35). The resolved `agent` string should just be cloned for the background thread.

## Implementation

### Signal handling

Add the `ctrlc` crate as a dependency (it only applies to the native binary, not the WASM plugin — the WASM build won't link it because `run.rs` is only compiled for the `[[bin]]` target).

In `run()`, before spawning the child:
- Set up a `ctrlc::set_handler` that sets an `Arc<AtomicBool>` flag (reuse the existing `running` flag or add a separate `interrupted` flag).
- After `child.wait()` returns (or in the signal handler), write a final "idle" status with message "Interrupted" and then call `writer.cleanup()`.

**Alternative (simpler, no new dependency):** Use the existing Unix signal infrastructure. The child process receives SIGINT directly (it's in the same process group). When the child exits due to signal, `child.wait()` returns with `ExitStatus` indicating the signal. The current code already writes "idle" + format_exit_message after wait. The real issue is: if the *parent* (zellai run) also receives SIGINT, `child.wait()` might error or the parent exits before writing status.

**Recommended approach:** Use `ctrlc` crate for simplicity. In the handler:
1. Set `interrupted` flag to true
2. The main loop checks this after `child.wait()`
3. Write "idle" status with "Interrupted by signal" message
4. Cleanup the status file
5. Exit

If `ctrlc` is problematic for `wasm32-wasip1` compilation, gate it with `#[cfg(not(target_arch = "wasm32"))]` in Cargo.toml as a target-specific dependency, or use raw `libc::signal` which is simpler.

### Fix redundant detection

Replace lines 71-76 in `run.rs`:
```rust
let bg_agent = command[0].clone();
let bg_agent_name = if status_writer::detect_agent(&bg_agent) != "unknown" {
    status_writer::detect_agent(&bg_agent).to_string()
} else {
    "unknown".to_string()
};
```

With:
```rust
let bg_agent_name = agent.clone();
```

The `agent` variable at this point already contains the resolved agent name (either user-supplied or auto-detected).

## Verification

```sh
cargo build --target wasm32-wasip1 && cargo clippy --target wasm32-wasip1 && cargo test --lib
```

Also verify the native binary builds:
```sh
cargo build && cargo clippy
```
