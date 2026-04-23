Title: Add shell completions generation to the CLI
Files: src/bin/zellai/main.rs, Cargo.toml
Issue: none

## Description

Add shell completion generation for bash, zsh, and fish (step 9 from the roadmap). Use clap's built-in `clap_complete` crate to generate completions from the `Cli` struct.

### Implementation

1. Add `clap_complete = "4"` to `[dependencies]` in `Cargo.toml`.

2. Add a `Completions` variant to the `Commands` enum in `src/bin/zellai/main.rs`:

```rust
/// Generate shell completions
Completions {
    /// Shell to generate completions for (bash, zsh, fish)
    #[arg(value_enum)]
    shell: clap_complete::Shell,
},
```

3. In the `main()` match arm for `Commands::Completions { shell }`:

```rust
Commands::Completions { shell } => {
    let mut cmd = Cli::command();
    clap_complete::generate(shell, &mut cmd, "zellai", &mut std::io::stdout());
}
```

Note: The completions output uses the name `"zellai"` (not `"zellai-cli"`) because that's the user-facing command name. Users are expected to alias or rename the binary.

4. This command does NOT need `#[cfg(not(target_arch = "wasm32"))]` gating — it's pure computation that writes to stdout.

### Verification

```sh
cargo build --target wasm32-wasip1 && cargo clippy --target wasm32-wasip1 && cargo test --lib
```

Also verify native output works:
```sh
cargo run --bin zellai-cli -- completions bash | head -5
cargo run --bin zellai-cli -- completions zsh | head -5
cargo run --bin zellai-cli -- completions fish | head -5
```

Each should produce non-empty shell completion scripts.
