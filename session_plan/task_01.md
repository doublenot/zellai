Title: Parse [keybindings] config section
Files: src/config.rs, SCHEMA.md
Issue: none

Add the `[keybindings]` section to `ZellaiConfig` as specified in SCHEMA.md. This is a prerequisite for wiring keyboard navigation in the plugin.

## What to do

1. In `src/config.rs`, add a `KeybindingsConfig` struct:
   ```rust
   #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
   #[serde(default)]
   pub struct KeybindingsConfig {
       pub next_attention: String,
       pub dismiss: String,
       pub jump_to: String,
   }
   ```
   With defaults from SCHEMA.md:
   - `next_attention` = `"Ctrl a"` — cycle to next pane needing attention
   - `dismiss` = `"Ctrl d"` — dismiss current pane notification
   - `jump_to` = `"Ctrl g"` — open pane picker (placeholder for future use)

2. Add `pub keybindings: KeybindingsConfig` to `ZellaiConfig`.

3. Add a helper function `pub fn parse_key(s: &str) -> Option<(bool, char)>` that parses the string format `"Ctrl x"` or `"x"` into a `(has_ctrl, char)` tuple. This gives the plugin event loop something to match against without importing Zellij types into the config module. Return `None` for unparseable strings.

4. Add tests:
   - Default keybindings values are correct
   - `parse_key("Ctrl a")` returns `Some((true, 'a'))`
   - `parse_key("Ctrl d")` returns `Some((true, 'd'))`
   - `parse_key("x")` returns `Some((false, 'x'))`
   - `parse_key("")` returns `None`
   - `parse_key("Ctrl")` returns `None` (incomplete)
   - Roundtrip: serialize default config → deserialize → keybindings match
   - Partial TOML with only `[keybindings]` section overrides only those fields
   - Full TOML with keybindings parses correctly

5. Keep SCHEMA.md unchanged — it already documents `[keybindings]`.

## Verification

```sh
cargo build --target wasm32-wasip1 && cargo clippy --target wasm32-wasip1 && cargo test --lib
```

All existing tests must still pass. New tests must pass.
