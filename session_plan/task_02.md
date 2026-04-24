Title: Fix shell hook JSON injection vulnerability
Files: hooks/on-stop.sh, hooks/on-notification.sh, hooks/on-post-tool-use.sh
Issue: none (assessment gap #2 — critical bug)

## Description

All three shell hook scripts interpolate `$working_dir`, `$ZELLAI_SESSION_ID`, `$git_branch`, and `$agent_name` directly into JSON heredocs without escaping. If any of these contain `"`, `\`, tabs, newlines, or other special characters, the resulting JSON will be malformed and the status bridge will silently fail to parse it.

The `$notification` and `$tool_message` variables ARE properly escaped (via `sed`), but the other variables are not.

### Variables that need escaping (all 3 hooks):
- `$working_dir` — paths can contain spaces, quotes, backslashes (e.g., `/home/user/"weird" dir/`)
- `$ZELLAI_SESSION_ID` — user-supplied, could contain anything
- `$agent_name` — derived from `$ZELLAI_AGENT` env var, user-supplied
- `$git_branch` — branch names can contain `/`, but theoretically `"` or `\` in branch names

### Fix approach:

Create a reusable `json_escape` shell function at the top of each hook:

```bash
# Escape a string for safe JSON interpolation
json_escape() {
    printf '%s' "$1" | sed 's/\\/\\\\/g; s/"/\\"/g; s/\t/\\t/g' | tr -d '\n'
}
```

Then use it for every interpolated variable:

```bash
working_dir_json=$(json_escape "$working_dir")
session_id_json=$(json_escape "$ZELLAI_SESSION_ID")
agent_name_json=$(json_escape "$agent_name")
```

And in the heredoc:
```json
  "session_id": "$session_id_json",
  "agent": "$agent_name_json",
  "working_dir": "$working_dir_json",
```

Note: `$git_branch` is already handled specially (formatted as `null` or `"$git_branch"`) — it also needs escaping when non-null.

### Files to modify:
1. `hooks/on-stop.sh` — escape `working_dir`, `ZELLAI_SESSION_ID`, `agent_name`, `git_branch`
2. `hooks/on-notification.sh` — same variables + `notification` (already escaped, but unify the approach)
3. `hooks/on-post-tool-use.sh` — same variables + `tool_message` (already escaped, but unify the approach)

### Testing:
Since these are shell scripts, verify manually:
```bash
# Test with pathological inputs
ZELLAI_SESSION_ID='test"quote' ZELLAI_AGENT='agent\backslash' \
  bash -c 'cd /tmp && source hooks/on-notification.sh "hello"' 2>/dev/null
# Should produce valid JSON (validate with jq if available)
```

Also verify the embedded scripts in `init.rs` will pick up changes (they use `include_str!` so recompilation is needed):
```bash
cargo build --target wasm32-wasip1
cargo test --lib
cargo clippy --target wasm32-wasip1
```
