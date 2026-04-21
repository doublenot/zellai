Title: Config model with defaults and unit tests
Files: src/config.rs, src/lib.rs (add mod declaration)
Issue: none

## Goal

Implement the `ZellaiConfig` struct and TOML parsing logic. This is the config layer from SCHEMA.md. Like the status model, this task covers ONLY the pure-logic parts: the struct with sensible defaults, TOML deserialization, and unit tests. It does NOT read files from disk (that requires Zellij APIs).

## What to build

### Cargo.toml change

Add `toml` as a dependency (needed for config parsing, not re-exported by zellij-tile):

```toml
[dependencies]
zellij-tile = "0.44.1"
toml = "0.8"

[dev-dependencies]
serde_json = "1.0"
```

### src/config.rs

Create `src/config.rs` with:

1. **`SidebarPosition` enum**:
   ```rust
   #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
   #[serde(rename_all = "lowercase")]
   pub enum SidebarPosition {
       Left,
       Right,
       Bottom,
   }
   impl Default for SidebarPosition {
       fn default() -> Self { Self::Left }  // YOYO.md mandates left as default
   }
   ```

2. **`CardDensity` enum**:
   ```rust
   #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
   #[serde(rename_all = "lowercase")]
   pub enum CardDensity {
       Compact,
       Detailed,
       Adaptive,
   }
   impl Default for CardDensity {
       fn default() -> Self { Self::Adaptive }  // YOYO.md mandates adaptive as default
   }
   ```

3. **`TeamsLayout` enum**:
   ```rust
   #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
   #[serde(rename_all = "kebab-case")]
   pub enum TeamsLayout {
       OrchestratorTop,
       OrchestratorLeft,
       EqualGrid,
       Custom,
   }
   impl Default for TeamsLayout {
       fn default() -> Self { Self::OrchestratorTop }  // YOYO.md mandates orchestrator-top
   }
   ```

4. **Config section structs** — each with `#[serde(default)]` on all fields:

   ```rust
   #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
   #[serde(default)]
   pub struct SidebarConfig {
       pub position: SidebarPosition,
       pub card_density: CardDensity,
       pub attention_animation: bool,  // default: true (per YOYO.md)
   }

   #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
   #[serde(default)]
   pub struct TeamsConfig {
       pub default_layout: TeamsLayout,
       pub orchestrator_agent: String,  // default: "claude"
       pub worker_agent: String,        // default: "claude"
       pub worker_count: u32,           // default: 2
   }

   #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
   #[serde(default)]
   pub struct BridgeConfig {
       pub sessions_dir: String,        // default: "~/.local/share/zellai/sessions"
       pub poll_interval_ms: u64,       // default: 500
       pub stale_threshold_s: u64,      // default: 60
   }
   ```

5. **`ZellaiConfig` — the top-level struct**:
   ```rust
   #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
   #[serde(default)]
   pub struct ZellaiConfig {
       pub sidebar: SidebarConfig,
       pub teams: TeamsConfig,
       pub bridge: BridgeConfig,
   }
   ```

6. **`impl Default` for every struct** — with the exact defaults mandated by YOYO.md and SCHEMA.md:
   - `attention_animation`: `true`
   - `sessions_dir`: `"~/.local/share/zellai/sessions"`
   - `poll_interval_ms`: `500`
   - `stale_threshold_s`: `60`
   - `orchestrator_agent`: `"claude"`
   - `worker_agent`: `"claude"`
   - `worker_count`: `2`

7. **`parse_config(toml_str: &str) -> Result<ZellaiConfig, toml::de::Error>`** — parse TOML string into config.

8. **`ZellaiConfig::default()`** — returns fully-populated defaults (this is what's used when no config file exists).

### Unit tests

Write a `#[cfg(test)]` module with:

- `test_default_config` — `ZellaiConfig::default()` has all the mandated defaults
- `test_parse_empty_toml` — empty string `""` → all defaults (because `#[serde(default)]`)
- `test_parse_partial_config` — TOML with only `[sidebar]\nposition = "right"` → position is right, everything else is default
- `test_parse_full_config` — TOML with all sections filled → assert all values
- `test_parse_invalid_toml` — garbage string → Err

### src/lib.rs modification

Add `pub mod config;` to src/lib.rs, after `pub mod status;`.

## Verification

```sh
cargo build --target wasm32-wasip1
cargo clippy --target wasm32-wasip1
cargo test --lib
```

All tests from task_02 (status) should still pass, plus the new config tests. The WASM build must succeed — the config module is pure logic with no host dependencies.

## What NOT to do

- Don't read files from disk — config file loading happens through Zellij APIs later
- Don't import `zellij_tile` in config.rs — keep it pure (use `serde` directly)
- Don't implement `[[teams.layout]]` custom layout blocks yet — that's a future task
- Don't implement keybindings config yet — that needs Zellij key handling
