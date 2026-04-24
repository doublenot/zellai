Title: Make hook scripts agent-aware via ZELLAI_AGENT env var
Files: hooks/on-stop.sh, hooks/on-notification.sh, hooks/on-post-tool-use.sh
Issue: none

All three hook scripts hardcode `"agent": "claude"` in the JSON they write. This is incorrect if hooks are ever installed for non-Claude agents. The fix is simple: read from `$ZELLAI_AGENT` with a fallback to `"claude"` (since these hooks ship as Claude Code hooks).

## What to do

1. In each of the three hook scripts (`hooks/on-stop.sh`, `hooks/on-notification.sh`, `hooks/on-post-tool-use.sh`), add a line near the top (after the `ZELLAI_SESSION_ID` guard):
   ```bash
   # Agent name — defaults to "claude" for Claude Code hooks
   agent_name="${ZELLAI_AGENT:-claude}"
   ```

2. Replace the hardcoded `"agent": "claude"` in the JSON template with `"agent": "$agent_name"`.

3. That's it. Three files, one-line addition + one-line change each.

## Why this matters

- The `zellai run` wrapper already sets `ZELLAI_AGENT` in the environment. If someone installs these hooks in a context where `ZELLAI_AGENT` is set to something else, the hooks will respect it.
- Backward compatible: without `ZELLAI_AGENT` set, defaults to `"claude"` — same behavior as before.
- Assessment flagged this as a friction point: "Hooks assume `claude` agent."

## Verification

Manually verify the scripts are syntactically valid:
```sh
bash -n hooks/on-stop.sh && bash -n hooks/on-notification.sh && bash -n hooks/on-post-tool-use.sh
```

Also run the standard build verification (hooks don't affect Rust, but confirm nothing broke):
```sh
cargo build --target wasm32-wasip1 && cargo clippy --target wasm32-wasip1 && cargo test --lib
```
