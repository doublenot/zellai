Title: Fix "1 agents" pluralization + add init.rs unit tests
Files: src/status_bar.rs, src/bin/zellai/init.rs
Issue: none (assessment gaps #11 and #9)

## Description

Two small, high-value fixes in one task:

### Part A: Fix "1 agents" pluralization in status bar

In `src/status_bar.rs`, `render_status_bar()` always formats as `"{} agents"` regardless of count. When there's exactly 1 agent, it should say "1 agent" (singular).

**Current code** (around line 20-25):
```rust
format!(" ⬡ {} | {} agents | {}⚠ ", workspace_name, agent_count, attention_count)
// and
format!(" ⬡ {} | {} agents ", workspace_name, agent_count)
```

**Fix**: Use a helper or inline conditional:
```rust
let agent_word = if agent_count == 1 { "agent" } else { "agents" };
format!(" ⬡ {} | {} {} | {}⚠ ", workspace_name, agent_count, agent_word, attention_count)
```

**Update existing test** `test_single_agent_attention` which currently asserts `"1 agents"` — change to assert `"1 agent"` (without trailing 's'):
```rust
assert!(result.contains("1 agent"));
assert!(!result.contains("1 agents"));
```

Add a new test `test_single_agent_no_attention` that checks singular form without attention indicator.

### Part B: Add unit tests for init.rs

`src/bin/zellai/init.rs` has testable logic around `install_hook()` (deciding whether to install, overwrite, or skip) and `write_hook()`. Add tests using `tempfile` or `std::env::temp_dir()`.

**Tests to add** (in a `#[cfg(test)] mod tests` block at the bottom of `init.rs`):

1. `test_install_hook_fresh` — target doesn't exist → returns `Installed`
2. `test_install_hook_existing_zellai` — target exists with "zellai" in content → returns `Overwritten`
3. `test_install_hook_existing_foreign` — target exists without "zellai" → returns `Skipped`
4. `test_install_hook_force` — target exists without "zellai" + force=true → returns `Overwritten`
5. `test_write_hook_creates_file` — writes content, verifies content matches
6. `test_write_hook_executable` — on unix, verify permissions are 0o755

**Note**: `install_hook` and `write_hook` are currently private functions. They can remain private — tests within the same module can access them. The `HookAction` enum also needs to derive `PartialEq, Debug` for test assertions.

### Verification:
```bash
cargo test --lib                    # all existing + new tests pass
cargo build --target wasm32-wasip1  # plugin still builds
cargo clippy --target wasm32-wasip1 # no warnings
```

Note: tests for `init.rs` need `cargo test --bin zellai` since it's in the binary crate, not the lib. Verify both:
```bash
cargo test --lib
cargo test --bin zellai
```
