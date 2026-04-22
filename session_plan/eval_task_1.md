## Evaluation: Fix output filename collision and tilde expansion bugs

### Checklist

- [x] **Bug 1 — filename collision**: `[[bin]]` renamed from `"zellai"` to `"zellai-cli"` in `Cargo.toml`. `cargo build --target wasm32-wasip1` produces 0 collision warnings.
- [x] **Bug 2 — tilde expansion**: `home_dir: Option<String>` added to `ZellaiPlugin`; `load()` fires `run_command(&["sh", "-c", "echo ~"])` with `resolve_home` context; handler stores result; `resolved_sessions_dir()` replaces leading `~` with resolved home. Both `Timer` and `list_sessions` fallback use `resolved_sessions_dir()`.
- [x] **No blocking in render()**: `render()` only calls `agents_sorted()`, `render_sidebar()`, and `println!` — returns immediately.
- [x] **No forbidden APIs**: No `std::fs`, `std::net`, or `std::process` in plugin code.
- [x] **No shell injection**: `ls` and `cat` use array-form `run_command` (no `sh -c` with user data).
- [x] **Graceful degradation**: If `resolve_home` hasn't returned yet, `resolved_sessions_dir()` returns the raw path — first timer tick may fail silently, subsequent ticks work after home resolves.
- [x] **Build**: `cargo build --target wasm32-wasip1` — PASS
- [x] **Tests**: `cargo test --lib` — 75 passed, 0 failed
- [x] **Clippy**: `cargo clippy --target wasm32-wasip1` — clean

Verdict: **PASS**
Reason: Both bugs are correctly fixed — bin target renamed to eliminate collision warning, and tilde expansion is handled via async `echo ~` with no forbidden APIs or shell injection.
