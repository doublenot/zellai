Title: PR/CI status collection via `gh` CLI in StatusWriter
Files: src/bin/zellai/status_writer.rs
Issue: none

## Goal

Add GitHub PR number and CI status collection to the `StatusWriter` so the
`pr_number` and `pr_ci_status` fields in the status JSON are actually populated.
This is gap #3 from the assessment — these fields are defined in the schema and
parsed by the bridge, but the writer always omits them.

## Implementation Details

### status_writer.rs changes

1. **Add a `collect_pr_info()` function** that runs `gh pr view --json number,statusCheckRollup`
   and parses the output. Signature:
   ```rust
   fn collect_pr_info() -> (Option<u32>, Option<String>) 
   ```
   Returns `(pr_number, ci_status_string)` where ci_status_string is one of
   "passing", "failing", "pending", or None.

   Implementation:
   - Run `gh pr view --json number,statusCheckRollup` via `std::process::Command`
   - If `gh` is not installed or not in a git repo or no PR exists, return `(None, None)` — 
     degrade gracefully (this is a YOYO.md rule: "degrade gracefully if `gh` is absent")
   - Parse the JSON output:
     - `number` → `pr_number`
     - `statusCheckRollup` is an array of check objects with `state` field
     - If all checks have state "SUCCESS" → "passing"
     - If any check has state "FAILURE" or "ERROR" → "failing"  
     - Otherwise (PENDING, QUEUED, IN_PROGRESS, etc.) → "pending"
   - If the `statusCheckRollup` array is empty or null → ci_status = None

2. **Update `write_status()`** to call `collect_pr_info()` and include the fields
   in the JSON output. The JSON template should include:
   ```json
   "pr_number": 42,
   "pr_ci_status": "passing",
   ```
   When `pr_number` is None, emit `"pr_number": null` (serde_json handles this).
   Same for `pr_ci_status`.

3. **Add a configurable `gh` timeout.** The `gh` CLI can be slow on first run
   or with network issues. Use a 5-second timeout on the Command:
   ```rust
   Command::new("gh")
       .args([...])
       .timeout(Duration::from_secs(5))
   ```
   Note: `Command::timeout` is not available on stable. Instead, spawn the process
   and use `child.wait_timeout()` — or simpler: just use `.output()` which blocks.
   Since `write_status` is called from the CLI wrapper (not the WASM plugin), 
   blocking is acceptable. Just let it block — the 2-second poll interval means
   `gh` is called every 2 seconds, which is fine.

   Actually, **do NOT call `gh` on every status write.** That would be too frequent
   (every 2 seconds). Instead:
   - Add a `last_pr_check: Option<std::time::Instant>` field to `StatusWriter`
   - Add `cached_pr_number: Option<u32>` and `cached_ci_status: Option<String>` fields
   - Only call `collect_pr_info()` if it's been more than 30 seconds since the last check
   - On first write, always check
   - Use the cached values for intermediate writes

4. **Handle `gh auth` not configured.** If `gh` returns exit code != 0 with
   stderr containing "not logged in" or "auth", just return (None, None).

### Testing

Add tests for `collect_pr_info` parsing logic. Since we can't run `gh` in CI,
extract the JSON parsing into a separate testable function:

```rust
fn parse_pr_json(json_str: &str) -> (Option<u32>, Option<String>)
```

Test cases:
- Valid JSON with passing checks → (Some(42), Some("passing"))
- Valid JSON with failing checks → (Some(42), Some("failing"))  
- Valid JSON with pending checks → (Some(42), Some("pending"))
- Valid JSON with no checks → (Some(42), None)
- Empty/invalid JSON → (None, None)
- JSON with mixed check states (some passing, one failing) → "failing"

### Verification

```sh
cargo build --target wasm32-wasip1 && cargo clippy --target wasm32-wasip1 && cargo test --lib
```

Note: The WASM build includes lib.rs (which has status.rs with CiStatus), and the
native build includes the binary. Both must compile. The `collect_pr_info` function
uses `std::process::Command` which is fine in the binary (not the WASM plugin).
