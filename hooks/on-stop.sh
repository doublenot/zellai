#!/usr/bin/env bash
# hooks/on-stop.sh — Claude Code "Stop" hook for zellai
#
# Fired when the Claude Code agent finishes a task or stops.
# Writes a final "idle" status, then deletes the status file (clean exit = session over).
#
# Environment variables:
#   ZELLAI_SESSION_ID   — (required) unique session identifier; exits silently if unset
#   ZELLAI_AGENT        — (optional) agent name; defaults to "claude"
#   ZELLAI_SESSIONS_DIR — (optional) override for sessions directory
#   XDG_DATA_HOME       — (optional) base data dir; defaults to $HOME/.local/share

set -euo pipefail

# Escape a string for safe JSON interpolation
json_escape() {
    printf '%s' "$1" | sed 's/\\/\\\\/g; s/"/\\"/g; s/\t/\\t/g' | tr -d '\n'
}

# Exit silently if not running under zellai
[[ -z "${ZELLAI_SESSION_ID:-}" ]] && exit 0

# Agent name — defaults to "claude" for Claude Code hooks
agent_name="${ZELLAI_AGENT:-claude}"

# Compute sessions directory
sessions_dir="${ZELLAI_SESSIONS_DIR:-${XDG_DATA_HOME:-$HOME/.local/share}/zellai/sessions}"
mkdir -p "$sessions_dir"

status_file="$sessions_dir/${ZELLAI_SESSION_ID}.json"
tmp_file="$status_file.tmp"

# Collect git info
git_branch=$(git rev-parse --abbrev-ref HEAD 2>/dev/null || echo "")
git_dirty=false
if ! git diff --quiet 2>/dev/null; then
    git_dirty=true
fi

# Current state
working_dir=$(pwd)
updated_at=$(date +%s)

# Escape all interpolated strings for JSON safety
session_id_json=$(json_escape "$ZELLAI_SESSION_ID")
agent_name_json=$(json_escape "$agent_name")
working_dir_json=$(json_escape "$working_dir")

# Format git_branch as JSON (null if empty, escaped otherwise)
if [[ -n "$git_branch" ]]; then
    git_branch_json="\"$(json_escape "$git_branch")\""
else
    git_branch_json="null"
fi

# Write idle status atomically
cat > "$tmp_file" <<EOF
{
  "version": 1,
  "session_id": "$session_id_json",
  "agent": "$agent_name_json",
  "status": "idle",
  "git_branch": $git_branch_json,
  "git_dirty": $git_dirty,
  "working_dir": "$working_dir_json",
  "last_message": null,
  "ports": [],
  "needs_attention": false,
  "updated_at": $updated_at
}
EOF
mv "$tmp_file" "$status_file"

# Clean exit — remove the status file (session is over)
rm -f "$status_file"
