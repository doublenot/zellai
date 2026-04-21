Title: Create Claude Code hook scripts that write status JSON files
Files: hooks/on-stop.sh, hooks/on-notification.sh, hooks/on-post-tool-use.sh
Issue: none

## Description

Create the three Claude Code hook shell scripts that form the **write side** of the status bridge. Without these, the entire read pipeline (status_bridge → sidebar) has nothing to read. This is the core of roadmap item #4.

### Context

Claude Code supports three hook types that zellai uses:
- **Stop** — fired when the agent finishes a task or stops
- **Notification** — fired when the agent sends a notification to the user
- **PostToolUse** — fired after each tool use (file read, edit, command, etc.)

Each hook script writes a JSON status file to `$ZELLAI_SESSIONS_DIR/<session-id>.json`. The session ID comes from `$ZELLAI_SESSION_ID` (set by the wrapper/init at pane creation time).

### Status file location

The default sessions directory is `${XDG_DATA_HOME:-$HOME/.local/share}/zellai/sessions/`. Scripts should use `$ZELLAI_SESSIONS_DIR` if set, falling back to this default.

### Hook specifications (from SCHEMA.md)

**`hooks/on-stop.sh`**
- Receives no special arguments from Claude Code
- Writes status file with `"status": "idle"`, `"needs_attention": false`
- Then **deletes** the status file (clean exit = session over)
- If `$ZELLAI_SESSION_ID` is not set, exit silently (not running under zellai)

**`hooks/on-notification.sh`**
- Claude Code passes the notification text. The hook receives it as the first argument or via stdin (check Claude Code hook docs — use `$1` or read from stdin)
- Writes status file with `"status": "waiting"`, `"needs_attention": true`, `"last_message"` set to the notification text
- If `$ZELLAI_SESSION_ID` is not set, exit silently

**`hooks/on-post-tool-use.sh`**
- Claude Code passes tool information. The hook receives the tool name as context
- Writes status file with `"status": "thinking"`, `"needs_attention": false`, `"last_message"` set to the tool description (e.g., "Using Read file…")
- If `$ZELLAI_SESSION_ID` is not set, exit silently

### Common pattern for all hooks

Each hook should:
1. Check `$ZELLAI_SESSION_ID` is set; exit 0 if not (graceful no-op)
2. Compute sessions dir: `${ZELLAI_SESSIONS_DIR:-${XDG_DATA_HOME:-$HOME/.local/share}/zellai/sessions}`
3. Create sessions dir if it doesn't exist: `mkdir -p "$sessions_dir"`
4. Collect git info: `git_branch=$(git rev-parse --abbrev-ref HEAD 2>/dev/null || echo "")`, `git_dirty` via `git diff --quiet`
5. Get working directory: `working_dir=$(pwd)`
6. Get current timestamp: `updated_at=$(date +%s)`
7. Write the JSON file atomically (write to `.tmp` then `mv`) to avoid partial reads
8. Use `cat <<EOF` or printf for JSON generation — no jq dependency required

### JSON schema reminder

```json
{
  "version": 1,
  "session_id": "$ZELLAI_SESSION_ID",
  "agent": "claude",
  "status": "thinking|waiting|idle",
  "git_branch": "branch-name-or-null",
  "git_dirty": true|false,
  "working_dir": "/absolute/path",
  "last_message": "message or null",
  "ports": [],
  "needs_attention": true|false,
  "updated_at": 1706000101
}
```

Note: `ports`, `pr_number`, and `pr_ci_status` are not set by hooks — use empty array `[]` for ports and omit the PR fields (they're optional in the schema).

### Verification

- All three scripts must be executable (`chmod +x`)
- Run `bash -n hooks/on-stop.sh`, `bash -n hooks/on-notification.sh`, `bash -n hooks/on-post-tool-use.sh` to syntax-check
- If shellcheck is available, run `shellcheck hooks/*.sh`
- Rust build must still pass: `cargo build --target wasm32-wasip1 && cargo clippy --target wasm32-wasip1 && cargo test --lib`
- Scripts should have a header comment explaining their purpose and expected environment variables
