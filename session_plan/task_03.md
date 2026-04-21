Title: Add `zellai init` integration tests and update SCHEMA.md
Files: tests/init_test.rs (new), SCHEMA.md, src/bin/zellai/main.rs
Issue: none

## Goal

Add integration tests for the `zellai init` command to verify correct behavior, and update SCHEMA.md to document the CLI binary component. Also add a `--sessions-dir` global option to the CLI for overriding the default sessions directory (useful for testing and non-standard setups).

## What to do

### 1. Create `tests/init_test.rs`

Integration tests that run the `zellai` binary as a subprocess (using `std::process::Command` against the compiled binary). These tests exercise the real init flow:

```rust
use std::process::Command;
use std::path::Path;

fn zellai_bin() -> std::path::PathBuf {
    // cargo sets OUT_DIR during test builds; use CARGO_BIN_EXE_zellai 
    // or construct the path manually
    let mut path = std::env::current_exe().unwrap();
    path.pop(); // remove test binary name
    path.pop(); // remove deps/
    path.push("zellai");
    path
}
```

Tests to write:

**a) `test_init_no_claude_dir`** — Run `zellai init` in a temp dir with no `.claude/`. Assert exit code is non-zero. Assert stderr/stdout contains "No .claude/ directory".

**b) `test_init_installs_hooks`** — Create a temp dir with `.claude/` inside. Run `zellai init`. Assert exit code is 0. Assert `.claude/hooks/on-stop.sh`, `.claude/hooks/on-notification.sh`, `.claude/hooks/on-post-tool-use.sh` all exist. Assert each file is executable. Assert each file contains "zellai".

**c) `test_init_skips_foreign_hooks`** — Create a temp dir with `.claude/hooks/on-stop.sh` containing "my custom hook" (no "zellai"). Run `zellai init` (without --force). Assert the foreign hook was NOT overwritten. Assert stdout contains "Skipping".

**d) `test_init_force_overwrites`** — Same setup as (c), but run `zellai init --force`. Assert the foreign hook WAS overwritten with zellai content.

**e) `test_init_idempotent`** — Run `zellai init` twice in a row. Both should succeed (exit code 0). The hook content should be identical after both runs.

### 2. Update SCHEMA.md

Add a new section after "Component Responsibilities" that documents the CLI binary:

```markdown
### `src/bin/zellai/` — CLI Binary

Native binary for host-side operations. Does not compile to WASM.

- `main.rs` — Entry point, clap argument parsing
- `init.rs` — `zellai init`: auto-detect Claude Code projects, install hook scripts

Subcommands:
- `zellai init [--force]` — Install zellai hooks into `.claude/hooks/`
```

### 3. Add `--sessions-dir` global option

Add a global option to the CLI for overriding the sessions directory. This is useful for testing and for users with non-standard XDG setups:

```rust
#[derive(Parser)]
struct Cli {
    /// Override the sessions directory path
    #[arg(long, global = true)]
    sessions_dir: Option<String>,
    
    #[command(subcommand)]
    command: Commands,
}
```

This option isn't used by `init` yet, but it will be needed by `zellai run` (task for a future session). Adding it now establishes the pattern.

## Verification

```sh
cargo build --target wasm32-wasip1          # WASM plugin still builds
cargo build                                  # Native CLI builds  
cargo test --lib                             # Unit tests pass
cargo test --test init_test                  # Integration tests pass
cargo clippy --target wasm32-wasip1          # Clean
cargo clippy                                 # Clean
```

## Key constraints

- Integration tests use `std::process::Command` to run the binary — they are NOT unit tests
- Each test creates its own temp directory and cleans up after itself
- Tests must not depend on any external state (no real `.claude/` directory)
