#!/usr/bin/env bash
# hooks/on-post-tool-use.sh — Claude Code "PostToolUse" hook for zellai
#
# Fired after each tool use (file read, edit, command, etc.) by the Claude Code agent.
# Writes a "thinking" status with the tool description as last_message.
#
# Arguments:
#   $1 — tool name/description (may also be provided via stdin as fallback)
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

# Get tool name from $1 or stdin
if [[ -n "${1:-}" ]]; then
    tool_name="$1"
elif [[ ! -t 0 ]]; then
    tool_name=$(cat)
else
    tool_name=""
fi

# Build a human-readable tool message
if [[ -n "$tool_name" ]]; then
    tool_message="Using $tool_name…"
else
    tool_message="Processing…"
fi

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
tool_message_json=$(json_escape "$tool_message")

# Format git_branch as JSON (null if empty, escaped otherwise)
if [[ -n "$git_branch" ]]; then
    git_branch_json="\"$(json_escape "$git_branch")\""
else
    git_branch_json="null"
fi

# Write thinking status atomically
cat > "$tmp_file" <<EOF
{
  "version": 1,
  "session_id": "$session_id_json",
  "agent": "$agent_name_json",
  "status": "thinking",
  "git_branch": $git_branch_json,
  "git_dirty": $git_dirty,
  "working_dir": "$working_dir_json",
  "last_message": "$tool_message_json",
  "ports": [],
  "needs_attention": false,
  "updated_at": $updated_at
}
EOF
mv "$tmp_file" "$status_file"
