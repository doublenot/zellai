Title: Plugin scaffold — Cargo.toml and minimal ZellijPlugin
Files: Cargo.toml, src/main.rs
Issue: none

## Goal

Create the foundational Rust project that compiles to a Zellij WASM plugin. This is step 1 of the YOYO.md build order and unblocks everything else.

## What to build

### Cargo.toml

Create `Cargo.toml` at the project root with:

```toml
[package]
name = "zellai"
version = "0.1.0"
edition = "2024"
authors = ["zellai contributors"]
description = "A Zellij plugin for managing AI coding agents"
license = "MIT"

[lib]
# Zellij plugins are loaded as WASM — the crate type must be cdylib
crate-type = ["cdylib"]

[dependencies]
zellij-tile = "0.44.1"

[dev-dependencies]
serde_json = "1.0"
```

Key decisions:
- **`crate-type = ["cdylib"]`** — required for Zellij WASM plugins (produces .wasm)
- **`edition = "2024"`** — we're on Rust 1.95, this is fine
- **`[lib]`** not `[[bin]]` — Zellij plugins are libraries, not executables. The entry point is the `register_plugin!` macro which generates `main()`, `load()`, `update()`, `render()`, `pipe()` exports.
- **Source file should be `src/lib.rs`** (not `src/main.rs`) since `crate-type = ["cdylib"]`.
- `zellij-tile` 0.44.1 already re-exports `serde`, `serde_json`, `strum`, `zellij-utils` — no need to add them as direct deps for the plugin itself. Add `serde_json` as dev-dependency for unit tests.

### src/lib.rs

Create `src/lib.rs` implementing the minimal `ZellijPlugin` trait:

```rust
use zellij_tile::prelude::*;
use std::collections::BTreeMap;

// Module declarations for future files (commented out until they exist)
// mod sidebar;
// mod status_bridge;
// mod config;
// mod attention;
// mod workspace;
// mod teams;

#[derive(Default)]
struct ZellaiPlugin {
    loading: bool,
}

register_plugin!(ZellaiPlugin);

impl ZellijPlugin for ZellaiPlugin {
    fn load(&mut self, _configuration: BTreeMap<String, String>) {
        self.loading = true;
        // Request permissions needed for file watching and running commands
        request_permission(&[
            PermissionType::ReadApplicationState,
            PermissionType::ChangeApplicationState,
            PermissionType::RunCommands,
        ]);
        // Subscribe to events we'll need
        subscribe(&[
            EventType::Timer,
            EventType::FileSystemUpdate,
            EventType::FileSystemCreate,
            EventType::FileSystemDelete,
            EventType::RunCommandResult,
            EventType::PermissionRequestResult,
        ]);
        // Set a periodic timer (500ms) for polling status files
        set_timeout(0.5);
    }

    fn update(&mut self, event: Event) -> bool {
        match event {
            Event::Timer(_) => {
                // Re-arm the timer
                set_timeout(0.5);
                // Will trigger re-render when we have status data to show
                true
            }
            Event::PermissionRequestResult(PermissionStatus::Granted) => {
                // Permissions granted — start watching filesystem
                watch_filesystem();
                true
            }
            _ => false,
        }
    }

    fn render(&mut self, rows: usize, cols: usize) {
        // Placeholder rendering — proves the plugin loads and renders
        println!("╭{}╮", "─".repeat(cols.saturating_sub(2)));
        if rows > 2 {
            let title = " zellai ";
            let padding = cols.saturating_sub(2 + title.len());
            println!("│{}{: <width$}│", title, "", width = padding);
        }
        if rows > 3 {
            let msg = " No agents connected ";
            let padding = cols.saturating_sub(2 + msg.len());
            println!("│{}{: <width$}│", msg, "", width = padding);
        }
        for _ in 0..rows.saturating_sub(4).min(rows) {
            println!("│{: <width$}│", "", width = cols.saturating_sub(2));
        }
        if rows > 1 {
            println!("╰{}╯", "─".repeat(cols.saturating_sub(2)));
        }
    }
}
```

Important notes:
- `println!` in a Zellij plugin writes to the plugin's pane output — this IS how rendering works. It's not stdout in the traditional sense.
- `watch_filesystem()` DOES exist in zellij-tile 0.44.1 (contrary to what YOYO.md says). Call it after permissions are granted.
- The render function draws a simple bordered box with a title. Keep it simple — sidebar.rs will take over rendering later.
- Don't import or use `std::fs`, `std::net`, `std::process` — these won't work in WASM.

## Verification

Run these commands and all must pass:

```sh
cargo build --target wasm32-wasip1
cargo clippy --target wasm32-wasip1
cargo test --lib
```

The build should produce `target/wasm32-wasip1/debug/zellai.wasm`.

Note: `cargo test --lib` runs against the host target (not wasm32), so any code that references Zellij host FFI will fail at link time in tests. The scaffold has no unit-testable pure logic yet, so `cargo test --lib` should just pass with 0 tests. That's fine.

## What NOT to do

- Don't create src/main.rs — the plugin uses `[lib]` with `src/lib.rs`
- Don't add `serde` or `serde_json` as direct dependencies — `zellij-tile` re-exports them
- Don't implement any real status reading or sidebar rendering — that's task 2 and 3
- Don't add module files that don't exist yet (sidebar.rs, etc.) — only commented-out `mod` declarations
