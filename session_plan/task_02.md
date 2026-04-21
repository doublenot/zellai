Title: Implement `zellai init` — auto-detect and install Claude Code hooks
Files: src/bin/zellai.rs, src/bin/init.rs (new)
Issue: none

## Goal

Implement the `zellai init` CLI command that auto-detects Claude Code projects and installs the zellai hook scripts. This completes roadmap step 4 ("Claude Code hooks").

## What to do

### 1. Create `src/bin/init.rs`

This module contains the `init` command logic. It should:

**a) Detect Claude Code projects:**
- Look for a `.claude/` directory in the current working directory
- If `.claude/` doesn't exist, print a message: "No .claude/ directory found. Are you in a Claude Code project?" and suggest creating it or running from the project root. Exit with code 1.

**b) Create the hooks directory:**
- Create `.claude/hooks/` if it doesn't exist

**c) Install hook scripts:**
- The hook script content should be embedded in the binary as string constants (use `include_str!("../../hooks/on-stop.sh")` etc.)
- Write three files:
  - `.claude/hooks/on-stop.sh` ← content from `hooks/on-stop.sh`
  - `.claude/hooks/on-notification.sh` ← content from `hooks/on-notification.sh`
  - `.claude/hooks/on-post-tool-use.sh` ← content from `hooks/on-post-tool-use.sh`
- Make each file executable (`chmod +x` equivalent — use `std::os::unix::fs::PermissionsExt`)
- If a hook file already exists, check if it contains "zellai" (case-insensitive). If it does, overwrite it (it's ours). If it doesn't contain "zellai", warn the user and skip that file: "Skipping {path}: existing hook not managed by zellai. Use --force to overwrite."

**d) Support `--force` flag:**
- If `--force` is passed, overwrite all hook files regardless

**e) Print summary:**
- Print which files were installed/skipped
- Print: "Done! Hook scripts installed. Set ZELLAI_SESSION_ID in your environment to activate."

### 2. Update `src/bin/zellai.rs`

Wire the `Init` subcommand to call `init::run(force)`. Add the `--force` flag to the `Init` variant:

```rust
#[derive(Subcommand)]
enum Commands {
    /// Initialize zellai hooks for the current project
    Init {
        /// Overwrite existing hook files even if not managed by zellai
        #[arg(long)]
        force: bool,
    },
}
```

### 3. Module structure

```
src/bin/
  zellai.rs    — CLI entry point (clap parsing)
  init.rs      — init command implementation
```

Note: Rust's `[[bin]]` system allows `src/bin/zellai.rs` to be a multi-file binary by using `mod init;` inside it (with `init.rs` as a sibling file, or `src/bin/zellai/main.rs` + `src/bin/zellai/init.rs`). Use the directory form:
- Rename `src/bin/zellai.rs` → `src/bin/zellai/main.rs`
- Create `src/bin/zellai/init.rs`

Update `Cargo.toml` if needed:
```toml
[[bin]]
name = "zellai"
path = "src/bin/zellai/main.rs"
```

## Verification

```sh
cargo build --target wasm32-wasip1          # WASM plugin still builds
cargo build                                  # Native CLI builds
cargo test --lib                             # All tests pass
cargo clippy --target wasm32-wasip1          # Clean
cargo clippy                                 # Clean
```

Manual test:
```sh
# Test without .claude/ directory
mkdir -p /tmp/test-init && cd /tmp/test-init
/path/to/target/debug/zellai init
# Should print "No .claude/ directory found" and exit 1

# Test with .claude/ directory
mkdir -p /tmp/test-init/.claude && cd /tmp/test-init
/path/to/target/debug/zellai init
# Should install 3 hook files and print summary

# Verify hooks are executable
ls -la /tmp/test-init/.claude/hooks/
# Should show -rwxr-xr-x for all 3 scripts

# Test idempotency — run again
/path/to/target/debug/zellai init
# Should overwrite (hooks contain "zellai")

# Clean up
rm -rf /tmp/test-init
```

## Key constraints

- Hook content is embedded via `include_str!` — no runtime file reading from the hooks/ directory
- All file I/O is in `src/bin/` only — never in library modules
- Use `std::os::unix::fs::PermissionsExt` for chmod (Linux-only is fine per YOYO.md)
