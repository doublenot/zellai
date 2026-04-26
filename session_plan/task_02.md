Title: Implement pane focusing for next_attention and jump_to keybindings
Files: src/lib.rs, src/sidebar.rs
Issue: none

## Goal

Complete the core UX promise: when the user presses the `next_attention` keybinding (Ctrl+a), the plugin should not only cycle the attention cursor internally but also **switch Zellij focus to that agent's terminal pane**. Also implement the `jump_to` keybinding (Ctrl+g) which is defined in config but has no handler.

This is the fix for the #1 user-facing gap identified in the assessment: "Attention cycling doesn't actually focus panes."

## What to do

### 1. Fix `next_attention` handler to call `focus_terminal_pane` (src/lib.rs)

In the `Event::Key` handler where `next_attention` is matched (around line 239-243), after calling `self.attention.next_attention()`, use the `pane_id_for_session` helper (from Task 1) to resolve the Zellij pane ID and call `focus_terminal_pane`:

```rust
if key == expected {
    if let Some(session_id) = self.attention.next_attention().map(|s| s.to_string()) {
        if let Some(pane_id) = self.pane_id_for_session(&session_id) {
            focus_terminal_pane(pane_id, false, false);
        }
    }
    return true;
}
```

Note: `focus_terminal_pane` is from `zellij_tile::prelude::*`. The two bool params are `should_float_if_hidden` and `should_be_in_place_if_hidden` — both `false` means "just focus the pane where it is."

### 2. Implement `jump_to` keybinding handler (src/lib.rs)

Add a `key_jump_to: Option<(bool, char)>` field to `ZellaiPlugin`, parsed from `self.config.keybindings.jump_to` in `load()` (same pattern as `key_next_attention` and `key_dismiss`).

In the `Event::Key` handler, add a match for `jump_to`. When pressed, focus the agent pane currently highlighted by the attention cursor (if any). This is similar to `next_attention` but doesn't advance the cursor — it just jumps to whichever agent is currently selected:

```rust
if key == jump_to_expected {
    if let Some(session_id) = self.attention.current().map(|s| s.to_string()) {
        if let Some(pane_id) = self.pane_id_for_session(&session_id) {
            focus_terminal_pane(pane_id, false, false);
        }
    }
    return true;
}
```

### 3. Visual indicator for cursor-selected agent in sidebar (src/sidebar.rs)

Add an optional `selected_session_id: Option<&str>` parameter to `render_sidebar` (or pass it through the existing config/state). When a card's session_id matches the selected one, render a `▶` arrow prefix or a highlight border to show which agent the cursor is on.

Keep this minimal — just a `▶` prefix on the agent name line in both compact and detailed card modes. Example:
```
▶ 🟡 Claude [waiting]
```
vs:
```
  🟢 Codex [thinking]
```

Update the `render_sidebar` call site in `lib.rs` to pass `self.attention.current()`.

## Verification

```sh
cargo build --target wasm32-wasip1 && cargo clippy --target wasm32-wasip1 && cargo test --lib
```

All existing sidebar rendering tests must pass. If `render_sidebar` signature changes, update test call sites. The `selected_session_id` parameter should default to `None` so existing tests work with minimal changes.
