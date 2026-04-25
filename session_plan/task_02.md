Title: Port detection for child processes in StatusWriter
Files: src/bin/zellai/status_writer.rs, src/bin/zellai/run.rs
Issue: none

## Description

The `ports` field in status files is always `[]`. The vision promises "What ports its dev server is listening on" and the sidebar already renders ports when present. The gap is **detection logic** — we need to discover which TCP ports the child process (and its descendants) are listening on.

### Approach: Parse `/proc/net/tcp` filtered by child PID's inode set

This is Linux-specific and appropriate since zellai targets Linux (Zellij is Linux-native). The approach:

1. **Add `fn collect_ports(pid: u32) -> Vec<u16>`** to `status_writer.rs`:
   - Get the child PID and all descendant PIDs (read `/proc/<pid>/task/*/children` recursively, or walk `/proc/` entries whose PPIDs match).
   - Simpler approach: read `/proc/<child_pid>/net/tcp` (shows all TCP sockets in the child's network namespace — for most dev servers this is the same as the host namespace).
   - Parse each line: columns are hex-encoded `local_address:port` and the state field (0A = LISTEN).
   - Filter to LISTEN state only.
   - Cross-reference with file descriptors: read `/proc/<pid>/fd/` → for each symlink pointing to `socket:[inode]`, check if the inode matches one of the LISTEN entries.
   - Return the list of ports as `Vec<u16>`.

   **Simplified version (recommended for this task):** Just read `/proc/<child_pid>/net/tcp`, filter LISTEN (state=0A) entries, and return all listening ports. This is good enough for dev servers — we don't need to match inodes to specific child PIDs initially. If the child runs a dev server on :3000, it will show up.

2. **Integrate into the status write loop:**
   - In `run.rs`, after spawning the child, store `child.id()` (the PID).
   - Pass the PID to the background status update thread.
   - In the background thread's periodic loop, call `collect_ports(pid)` and include the result when writing status.
   - Modify `StatusWriter::write_status()` to accept an optional `ports: &[u16]` parameter (add it as a new parameter or as a field on `StatusWriter`).

3. **Update `write_status` to include ports:**
   - Change `"ports": []` to use the actual collected ports.
   - The JSON field already exists in the schema, so no schema changes needed.

### Edge cases
- `/proc/net/tcp` not readable → return empty vec (graceful degradation)
- Child exits before port check → return empty vec
- Ports > 65535 (impossible but guard anyway)
- Non-Linux platforms: `#[cfg(target_os = "linux")]` gate the proc parsing; return empty vec on other platforms

### Tests to add (6-8 tests)

- `parse_proc_net_tcp` with sample `/proc/net/tcp` content → extracts correct listening ports
- `parse_proc_net_tcp` with no LISTEN entries → empty vec
- `parse_proc_net_tcp` with malformed lines → skips gracefully
- `parse_proc_net_tcp` with multiple listeners → returns all ports
- `parse_proc_net_tcp` filters out non-LISTEN states (ESTABLISHED, etc.)
- Integration: verify `write_status` JSON includes non-empty ports when provided

### Important constraints
- This code only runs on the native target (not WASM) — it's in `src/bin/`, so `#[cfg(not(target_arch = "wasm32"))]` is implicit.
- Do NOT use `std::fs` in the WASM plugin code — this is all in the CLI binary.
- Keep the function pure for testing: `parse_proc_net_tcp(content: &str) -> Vec<u16>` takes a string, not a file path.

### Verification

```sh
cargo build --target wasm32-wasip1 && cargo clippy --target wasm32-wasip1 && cargo test --lib
```
