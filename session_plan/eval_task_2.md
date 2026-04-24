## Evaluation: Fix shell hook JSON injection vulnerability

Verdict: PASS

Reason: All three hooks (`on-stop.sh`, `on-notification.sh`, `on-post-tool-use.sh`) now include an identical `json_escape` function and apply it to every interpolated variable (`session_id`, `agent_name`, `working_dir`, `git_branch`, `notification`, `tool_message`). Tested with pathological inputs containing quotes, backslashes, and tabs — all produce valid JSON confirmed by Python's `json.load`. No forbidden APIs in plugin code; build and tests pass.
