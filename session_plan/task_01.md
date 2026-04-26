Title: Add pane_id to status schema and PaneUpdate tracking in plugin
Files: src/status.rs, src/bin/zellai/status_writer.rs, src/lib.rs, SCHEMA.md
Issue: none

## Goal

Add foundational pane-to-session mapping so the plugin can focus specific Zellij panes when the user cycles attention. This is prerequisite infrastructure for fixing the #1 user-facing gap: "attention cycling doesn't actually focus panes."

## What to do

### 1. Add `pane_id` field to `AgentStatus` (src/status.rs)

Add an optional `pane_id: Option<u32>` field to `AgentStatus` with `#[serde(default)]` so existing status files without this field still parse correctly. Place it after `session_id`.

Update the `full_json()` test helper to include `"pane_id": 42`. Add a test that parsing without `pane_id` still works (it already should via `#[serde(default)]`, but verify).

### 2. Write `pane_id` from StatusWriter (src/bin/zellai/status_writer.rs)

In `StatusWriter::write_status()`, populate `pane_id` by reading `$ZELLIJ_PANE_ID` env var (Zellij 0.41+ sets this for terminals). Use `env::var("ZELLIJ_PANE_ID").ok().and_then(|s| s.parse::<u32>().ok())`. Store it in the `StatusWriter` struct as `pane_id: Option<u32>`, resolved once in `new()`.

In the `generate_session_id_with_env` function, also try to use `ZELLIJ_PANE_ID` as part of the session ID for more meaningful IDs: `<hostname>-pane-<pane_id>` when available, falling back to `<hostname>-<pid>`.

### 3. Subscribe to PaneUpdate in plugin (src/lib.rs)

Add `EventType::PaneUpdate` to the `subscribe()` call in `load()`.

Add a `pane_manifest: Option<PaneManifest>` field to `ZellaiPlugin` (only under `#[cfg(target_arch = "wasm32")]`). Initialize to `None` in `Default`.

In the `update()` method, handle the `Event::PaneUpdate(manifest)` variant:
```rust
Event::PaneUpdate(manifest) => {
    self.pane_manifest = Some(manifest);
    true // re-render to show updated pane info
}
```

### 4. Add `pane_id_for_session` helper method to `ZellaiPlugin` (src/lib.rs)

Add a method that takes a session_id and returns `Option<u32>` (the terminal pane ID):
- First, check if the agent's `AgentStatus.pane_id` is `Some(id)` — return it directly (most reliable).
- If not, fall back to heuristic: iterate panes from `self.pane_manifest`, find a terminal pane (not plugin) whose `title` contains the agent's `working_dir` basename. Return its `id`.
- This fallback handles sessions started by hook scripts that don't set `pane_id`.

### 5. Update SCHEMA.md

Add `pane_id` to the status file schema table:
| `pane_id` | integer \| null | no | Zellij terminal pane ID (for focus switching) |

## Verification

```sh
cargo build --target wasm32-wasip1 && cargo clippy --target wasm32-wasip1 && cargo test --lib
```

All existing tests must continue to pass. The new `pane_id` field must be backward-compatible (existing JSON without it must parse).
