Title: Implement `zellai doctor` diagnostics command
Files: src/bin/zellai/doctor.rs, src/bin/zellai/main.rs
Issue: none

## Description

Implement the `zellai doctor` command (step 9 from the roadmap). This is a diagnostics tool that checks the user's environment and reports issues. It should check:

1. **Zellij installed** — run `zellij --version`, report version or "not found"
2. **WASM plugin exists** — check if `target/wasm32-wasip1/debug/zellai.wasm` or release variant exists (informational only, not an error)
3. **Sessions directory** — check if the sessions dir (`~/.local/share/zellai/sessions/` or custom) exists and is writable. Create it if missing.
4. **Active sessions** — count `.json` files in sessions dir, report count
5. **Claude Code hooks** — check if `.claude/` exists in current dir, and if so whether the three hook scripts (`on-stop.sh`, `on-notification.sh`, `on-post-tool-use.sh`) are present in `.claude/hooks/`
6. **Config file** — check if `zellai.toml` exists in current dir or `~/.config/zellai/zellai.toml`, attempt to parse it, report errors
7. **Git available** — run `git --version`, report version or "not found"
8. **gh CLI available** — run `gh --version`, report version or "not found (optional)"

### Output format

Use a clean diagnostic format with check marks and X marks:

```
zellai doctor

✓ zellij 0.41.2
✓ sessions dir (~/.local/share/zellai/sessions/) exists
  2 active sessions
✓ Claude hooks installed (.claude/hooks/)
✗ zellai.toml not found (using defaults)
✓ git 2.43.0
✗ gh CLI not found (optional — PR/CI features disabled)
```

### Implementation

Create `src/bin/zellai/doctor.rs` with a `pub fn run() -> Result<(), String>` function. Each check is a separate function that returns a status line. Use `std::process::Command` to run external tools. Use `std::fs` to check paths.

Add a `Doctor` variant to the `Commands` enum in `src/bin/zellai/main.rs` with doc comment `/// Check environment and diagnose issues`.

### Verification

```sh
cargo build --target wasm32-wasip1 && cargo clippy --target wasm32-wasip1 && cargo test --lib
```

Also verify the native CLI builds: `cargo build` and `cargo run --bin zellai-cli -- doctor` should produce output.
