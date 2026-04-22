Title: Fix output filename collision and tilde expansion bugs
Files: Cargo.toml, src/config.rs
Issue: none

## Problem

Two bugs identified in the assessment:

### Bug 1: Output filename collision (medium severity)
Both `[[bin]]` and `[lib]` produce `zellai.wasm` when building for `wasm32-wasip1`. Cargo warns:
```
warning: output filename collision at target/wasm32-wasip1/debug/zellai.wasm
```
This will become a hard error in a future Rust release.

**Fix:** Rename the bin target in `Cargo.toml` to `name = "zellai-cli"`. The binary is a native CLI tool — it should never be compiled to WASM. The lib target is the plugin and must keep `name = "zellai"` (since it produces `zellai.wasm`).

Also update the binary's `path` comment and the README if it references the binary name.

### Bug 2: Tilde `~` in default sessions_dir won't expand (low-medium severity)
The default `sessions_dir` is `"~/.local/share/zellai/sessions"`. When passed to `run_command(&["ls", "-1", &sessions_dir])` in `lib.rs`, the `~` is a literal character — `ls` won't expand it. The plugin will silently fail to find any sessions.

**Fix:** In `config.rs`, change the default `sessions_dir` to use `$HOME` via the XDG convention. The most robust approach: use `/tmp/zellai/sessions` as a fallback, BUT the correct fix is to resolve `~` at load time. Since the plugin runs in WASM and can't call `std::env`, the best approach is:
1. Change the default to use an XDG-style path without tilde: change `BridgeConfig::default()` to use a path that doesn't need expansion.
2. In `lib.rs` `load()`, use `run_command(&["sh", "-c", "echo $HOME"])` to resolve the home directory, then substitute `~` in the sessions_dir with the resolved value.

Actually, the simplest correct fix: in `lib.rs`, when constructing the `ls` command, use `sh -c` so tilde expansion happens:
- Change `run_command(&["ls", "-1", &sessions_dir])` to `run_command(&["sh", "-c", &format!("ls -1 {}", sessions_dir)])`
- Similarly for the `cat` command: `run_command(&["sh", "-c", &format!("cat '{}'", filepath)])`

Wait — that introduces shell injection risks. Better approach:
1. In `lib.rs` `load()`, run `run_command(&["sh", "-c", "echo ~"], context)` with a `zellai_cmd: "resolve_home"` context.
2. In the `resolve_home` handler, store the resolved home dir on `ZellaiPlugin`.
3. Add a helper method `resolved_sessions_dir(&self) -> String` that replaces leading `~` with the stored home.
4. Use `resolved_sessions_dir()` in the timer handler for `ls` and in `list_sessions` for `cat`.

This is a clean fix that doesn't introduce shell injection and works for any `sessions_dir` value.

## Implementation steps

1. In `Cargo.toml`: change `[[bin]]` name from `"zellai"` to `"zellai-cli"`
2. In `src/lib.rs`: add a `home_dir: Option<String>` field to `ZellaiPlugin`
3. In `src/lib.rs` `load()`: after permissions request, fire `run_command(&["sh", "-c", "echo ~"], BTreeMap::from([("zellai_cmd".to_string(), "resolve_home".to_string())]))`
4. In `handle_run_command_result`: add a `"resolve_home"` arm that stores the result in `self.home_dir`
5. Add a `resolved_sessions_dir(&self) -> String` method that replaces leading `~` with `self.home_dir`
6. Use `resolved_sessions_dir()` in the `Timer` handler and `list_sessions` handler
7. Update tests for `BridgeConfig::default()` if needed (the default value itself stays the same — the expansion happens at runtime)

## Verification

```sh
cargo build --target wasm32-wasip1 2>&1 | grep -c "filename collision"  # should be 0
cargo build --target wasm32-wasip1 --lib 2>&1  # must succeed
cargo build 2>&1  # native build with renamed binary must succeed
cargo test --lib  # all 75 tests pass
cargo clippy --target wasm32-wasip1  # clean
```
