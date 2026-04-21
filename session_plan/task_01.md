Title: CLI binary scaffold with dual-target build
Files: Cargo.toml, src/lib.rs, src/bin/zellai.rs
Issue: none

## Goal

Add a native CLI binary (`zellai`) to the project alongside the existing WASM plugin library. This is the prerequisite for all remaining roadmap steps (4–9), which require a native binary for `zellai init`, `zellai run`, workspace management, etc.

## What to do

### 1. Update `Cargo.toml`

Add `"rlib"` to `crate-type` so the library can be linked by the binary:

```toml
[lib]
crate-type = ["cdylib", "rlib"]
```

Add a `[[bin]]` section:

```toml
[[bin]]
name = "zellai"
path = "src/bin/zellai.rs"
```

Add `clap` as a dependency (only needed for the native binary, but it's fine to include — it won't bloat the WASM since the binary isn't compiled to WASM):

```toml
[dependencies]
clap = { version = "4", features = ["derive"] }
```

### 2. Gate WASM-only code in `src/lib.rs`

The `register_plugin!(ZellaiPlugin)` macro and the `ZellijPlugin` trait impl are only meaningful when compiling to WASM. Gate them:

```rust
#[cfg(target_arch = "wasm32")]
register_plugin!(ZellaiPlugin);

#[cfg(target_arch = "wasm32")]
impl ZellijPlugin for ZellaiPlugin {
    // ... existing impl
}
```

Also gate the `ZellaiPlugin` struct and its `Default` impl and `handle_run_command_result` impl with `#[cfg(target_arch = "wasm32")]`, since they use `zellij_tile` types.

The `use zellij_tile::prelude::*;` import should also be gated:

```rust
#[cfg(target_arch = "wasm32")]
use zellij_tile::prelude::*;
```

The module declarations (`pub mod attention`, `pub mod config`, etc.) must remain ungated — they're shared between WASM and native.

### 3. Create `src/bin/zellai.rs`

Create a minimal CLI entry point using clap with subcommands:

```rust
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "zellai", version, about = "AI agent workspace manager for Zellij")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize zellai hooks for the current project
    Init,
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Commands::Init => {
            println!("zellai init: not yet implemented");
            std::process::exit(1);
        }
    }
}
```

This is just the scaffold — task_02 will implement the actual `init` logic.

## Verification

Run all three checks — they must all pass:

```sh
cargo build --target wasm32-wasip1          # WASM plugin still builds
cargo build                                  # Native CLI binary builds
cargo test --lib                             # All 75 tests pass
cargo clippy --target wasm32-wasip1          # No WASM warnings
cargo clippy                                 # No native warnings
```

Also verify the binary runs:

```sh
./target/debug/zellai --help                 # Should show help with "init" subcommand
./target/debug/zellai --version              # Should show 0.1.0
```

## Key constraint

Do NOT add any `std::fs` or `std::process` calls to library modules (src/lib.rs, src/status.rs, etc.). Those are WASM-incompatible. File I/O belongs only in `src/bin/zellai.rs` and any CLI-specific modules.
