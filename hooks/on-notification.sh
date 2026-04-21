#!/usr/bin/env bash
# hooks/on-notification.sh — Claude Code "Notification" hook for zellai
#
# Fired when the Claude Code agent sends a notification to the user.
# Writes a "waiting" status with needs_attention=true and captures the notification text.
#
# Arguments:
#   $1 — notification text (may also be provided via stdin as fallback)
#
# Environment variables:
#   ZELLAI_SESSION_ID   — (required) unique session identifier; exits silently if unset
#   ZELLAI_SESSIONS_DIR — (optional) override for sessions directory
#   XDG_DATA_HOME       — (optional) base data dir; defaults to $HOME/.local/share

set -euo pipefail

# Exit silently if not running under zellai
[[ -z "${ZELLAI_SESSION_ID:-}" ]] && exit 0

# Compute sessions directory
sessions_dir="${ZELLAI_SESSIONS_DIR:-${XDG_DATA_HOME:-$HOME/.local/share}/zellai/sessions}"
mkdir -p "$sessions_dir"

status_file="$sessions_dir/${ZELLAI_SESSION_ID}.json"
tmp_file="$status_file.tmp"

# Get notification text from $1 or stdin
if [[ -n "${1:-}" ]]; then
    notification="$1"
elif [[ ! -t 0 ]]; then
    notification=$(cat)
else
    notification=""
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

# Format git_branch as JSON (null if empty)
if [[ -n "$git_branch" ]]; then
    git_branch_json="\"$git_branch\""
else
    git_branch_json="null"
fi

# Format last_message as JSON (null if empty, escaped otherwise)
if [[ -n "$notification" ]]; then
    # Escape backslashes, double quotes, and control characters for JSON safety
    escaped=$(printf '%s' "$notification" | sed 's/\\/\\\\/g; s/"/\\"/g; s/\t/\\t/g')
    last_message_json="\"$escaped\""
else
    last_message_json="null"
fi

# Write waiting status atomically
cat > "$tmp_file" <<EOF
{
  "version": 1,
  "session_id": "$ZELLAI_SESSION_ID",
  "agent": "claude",
  "status": "waiting",
  "git_branch": $git_branch_json,
  "git_dirty": $git_dirty,
  "working_dir": "$working_dir",
  "last_message": $last_message_json,
  "ports": [],
  "needs_attention": true,
  "updated_at": $updated_at
}
EOF
mv "$tmp_file" "$status_file"
