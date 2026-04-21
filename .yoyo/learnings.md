# zellai Learnings

Project-specific lessons recorded by yoyo. Updated when something non-obvious is discovered.

---

## Factor pure logic out of the plugin module aggressively
**Context:** Built config parsing, status model, and the ZellijPlugin impl in one session. `cargo test --lib` only works for code that doesn't touch Zellij API types — the host API is only available inside the WASM sandbox. Had to keep `config.rs` and `status.rs` completely free of `zellij_tile` imports so they could be unit-tested on the host target.
**Takeaway:** For Zellij WASM plugins, the unit-testability boundary is the Zellij API boundary. Every function that can be expressed as pure data transformation (parsing, validation, staleness checks, rendering to strings) should live in modules that never import `zellij_tile`. The plugin module (`lib.rs`) becomes a thin orchestration layer that wires host events to pure logic. This maximizes test coverage since plugin-API code can only be verified by actually loading the WASM in Zellij.

## Use `run_command` context as continuation state
**Context:** The plugin needs to list session files then read each one, but `run_command` is async — you call it and get the result later in a separate `RunCommandResult` event. To chain `ls` → `cat` for each file, the `context: BTreeMap<String, String>` passed to `run_command` carries a `zellai_cmd` discriminator ("list_sessions" / "read_status") plus any data the next step needs (e.g. `sessions_dir`, `session_file`). The `handle_run_command_result` dispatcher routes on `zellai_cmd` and extracts state from the same map.
**Takeaway:** In Zellij WASM plugins, `run_command`'s context map is your only mechanism for passing state between async steps. Treat it as a manual continuation: tag each call with a command name and stuff in whatever the handler will need. Keep the dispatcher (`match cmd.as_str()`) in one place so the async flow is readable. This pattern will recur for every multi-step async operation (git status, gh CLI, etc.).

## File presence as liveness signal in file-based IPC
**Context:** Writing the Claude Code hook scripts, specifically `on-stop.sh`. The hook writes a final "idle" status then immediately `rm -f`s the file. A clean exit leaves no status file behind; a crash or hang leaves a stale file. The plugin's `mark_stale` logic (Timer-based age check) then detects the orphaned file and flags it for attention. This creates three distinct observable states from a single file: present+fresh = alive, present+stale = probably crashed, absent = cleanly exited.
**Takeaway:** For file-based IPC where the writer may die unexpectedly, treat file presence itself as the liveness signal — not a field inside the file. Clean exits delete the file; crashes leave it behind for the reader's staleness detector. This avoids the need for heartbeat fields or lock files: the filesystem *is* the health check. The atomic write pattern (`write .tmp` → `mv`) ensures the reader never sees a half-written file.

<!-- yoyo appends learnings here -->
