Title: AgentStatus model and status parsing — pure logic with unit tests
Files: src/status.rs, src/lib.rs (add mod declaration)
Issue: none

## Goal

Implement the `AgentStatus` struct and JSON parsing logic for status files. This is the data model that the entire plugin revolves around — step 2 of YOYO.md's build order ("Status bridge (reader)"). This task covers ONLY the pure-logic, unit-testable parts: the struct, deserialization, validation, and stale detection. It does NOT touch Zellij APIs.

## What to build

### src/status.rs

Create `src/status.rs` with:

1. **`AgentKind` enum** — the agent type:
   ```rust
   #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
   #[serde(rename_all = "lowercase")]
   pub enum AgentKind {
       Claude,
       Codex,
       Gemini,
       Aider,
       Opencode,
       Unknown,
   }
   ```

2. **`AgentStatusValue` enum** — the status field:
   ```rust
   #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
   #[serde(rename_all = "lowercase")]
   pub enum AgentStatusValue {
       Thinking,
       Waiting,
       Idle,
       Error,
   }
   ```

3. **`CiStatus` enum** — for `pr_ci_status`:
   ```rust
   #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
   #[serde(rename_all = "lowercase")]
   pub enum CiStatus {
       Passing,
       Failing,
       Pending,
   }
   ```

4. **`AgentStatus` struct** — the main data model, matching the JSON schema in SCHEMA.md:
   ```rust
   #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
   pub struct AgentStatus {
       pub version: u32,
       pub session_id: String,
       pub agent: AgentKind,
       pub status: AgentStatusValue,
       pub git_branch: Option<String>,
       pub git_dirty: bool,
       pub working_dir: String,
       pub last_message: Option<String>,
       pub ports: Vec<u16>,
       #[serde(default)]
       pub pr_number: Option<u32>,
       #[serde(default)]
       pub pr_ci_status: Option<CiStatus>,
       pub needs_attention: bool,
       pub updated_at: u64,
   }
   ```

5. **`AgentStatus::validate(&mut self)`** — enforce invariants from SCHEMA.md:
   - If `status != Waiting`, force `needs_attention = false`
   - If `status == Waiting`, force `needs_attention = true`

6. **`AgentStatus::is_stale(&self, now_epoch: u64, threshold_s: u64) -> bool`** — returns true if `now_epoch - updated_at > threshold_s`

7. **`parse_status(json: &str) -> Result<AgentStatus, serde_json::Error>`** — parse JSON string, then call `validate()` on the result before returning.

8. **`impl std::fmt::Display for AgentKind`** — display-friendly agent names.

9. **`impl std::fmt::Display for AgentStatusValue`** — display-friendly status names.

### Unit tests (in the same file)

Write a `#[cfg(test)]` module with these tests:

- `test_parse_valid_status` — full JSON with all fields, assert each field
- `test_parse_minimal_status` — JSON without optional fields (`pr_number`, `pr_ci_status`), assert defaults
- `test_validate_forces_needs_attention_false` — status=thinking + needs_attention=true → validate makes it false
- `test_validate_forces_needs_attention_true` — status=waiting + needs_attention=false → validate makes it true
- `test_stale_detection` — updated_at=100, now=200, threshold=60 → is_stale=true
- `test_not_stale` — updated_at=100, now=130, threshold=60 → is_stale=false
- `test_parse_invalid_json` — malformed JSON → Err
- `test_parse_unknown_agent` — unknown agent string handled (serde should fail or map to Unknown — decide which is better for robustness; prefer mapping to Unknown using a custom deserializer or `#[serde(other)]`)

### src/lib.rs modification

Add `pub mod status;` to src/lib.rs. Place it before the `ZellaiPlugin` struct. This must be an unconditional `pub mod`, not behind `#[cfg(test)]`, since the status module will be used by the plugin at runtime.

Important: use `serde` and `serde_json` from `zellij-tile`'s re-exports. In the status module:
```rust
use serde::{Serialize, Deserialize};
```

For unit tests that need `serde_json::from_str`, use the dev-dependency `serde_json` added in Cargo.toml.

## Verification

```sh
cargo build --target wasm32-wasip1
cargo clippy --target wasm32-wasip1
cargo test --lib
```

`cargo test --lib` should show 8 passing tests. The WASM build must also succeed — the status module contains no host-specific code.

## What NOT to do

- Don't use `std::fs` to read files — that's the bridge's job and uses Zellij APIs
- Don't import anything from `zellij_tile::prelude` in status.rs — keep it pure
- Don't create status_bridge.rs yet — that depends on Zellij event handling and is a separate task
- Don't add `serde` or `serde_json` to `[dependencies]` — they're already available through `zellij-tile`
