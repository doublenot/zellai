Title: Wire keyboard navigation in the plugin event loop
Files: src/lib.rs
Issue: none

The `AttentionTracker` has `next_attention()` and `dismiss()` methods, and task_01 adds keybinding config parsing. This task wires them together in the plugin's event loop so users can actually cycle through and dismiss attention notifications via keyboard.

## What to do

1. In `src/lib.rs`, add `EventType::Key` to the `subscribe` call in `load()`.

2. In `load()`, after parsing the config, call `config::parse_key()` on each keybinding string and store the results in the plugin struct. Add two fields to `ZellaiPlugin`:
   ```rust
   /// Parsed keybinding for next_attention (ctrl, char)
   key_next_attention: Option<(bool, char)>,
   /// Parsed keybinding for dismiss (ctrl, char)  
   key_dismiss: Option<(bool, char)>,
   ```
   Initialize them from `self.config.keybindings.next_attention` and `.dismiss` using `config::parse_key()`.

3. Add a `Event::Key(key)` match arm in `update()`. Match against the parsed keybindings:
   - **next_attention**: Call `self.attention.next_attention()`. If it returns a session_id, call `focus_terminal_pane` (from `zellij_tile::prelude`) to jump to that pane. Note: the session_id is NOT a Zellij pane ID — we don't have a mapping yet. For now, just cycle the attention tracker and trigger a re-render (return `true`). Add a comment `// TODO: map session_id to Zellij pane ID for focus_terminal_pane`.
   - **dismiss**: Call `self.attention.dismiss()` on the current session. Get the current session via `self.attention.current()` first, then dismiss it. Return `true` to re-render.

4. For matching the key: Zellij's `Key` enum uses `Key::Char(c)` and `Key::Ctrl(c)`. Match like:
   ```rust
   Event::Key(key) => {
       if let Some((ctrl, ch)) = self.key_next_attention {
           let matches = if ctrl {
               key == Key::Ctrl(ch)
           } else {
               key == Key::Char(ch)
           };
           if matches {
               self.attention.next_attention();
               return true;
           }
       }
       // ... same pattern for dismiss
       false
   }
   ```

5. The `Key` type is from `zellij_tile::prelude::*` which is already imported under `#[cfg(target_arch = "wasm32")]`.

## Important constraints

- All new code must be gated with `#[cfg(target_arch = "wasm32")]` since `Key`, `Event`, etc. only exist in the WASM target.
- Do NOT call `focus_terminal_pane` yet — we don't have session_id→pane_id mapping. Just cycle the tracker.
- Do NOT add any blocking I/O.

## Verification

```sh
cargo build --target wasm32-wasip1 && cargo clippy --target wasm32-wasip1 && cargo test --lib
```

All existing 123+ tests must still pass. No new unit tests needed for this task (the event handling code is WASM-only and untestable in `--lib`).
