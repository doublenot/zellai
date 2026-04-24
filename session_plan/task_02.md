Title: Fix clippy warnings — &PathBuf → &Path in workspace.rs
Files: src/workspace.rs
Issue: none

## Context

`cargo clippy` on the host target produces 3 warnings:

1. `writing &PathBuf instead of &Path` at `load_workspace_from` (line 237)
2. `writing &PathBuf instead of &Path` at `delete_workspace_from` (line 282)
3. `function resolve_workspaces_dir_with is never used` (line 201) — dead_code warning

For consistency, also fix `save_workspace_to` (line 213) and `list_workspaces_in` (line 256) which take `&PathBuf` but weren't flagged (likely because clippy's heuristic didn't trigger, but they should still use `&Path` for idiomatic Rust).

## Steps

1. In `src/workspace.rs`, change the function signatures:
   - `save_workspace_to(workspace: &Workspace, dir: &PathBuf)` → `dir: &Path`
   - `load_workspace_from(name: &str, dir: &PathBuf)` → `dir: &Path`
   - `list_workspaces_in(dir: &PathBuf)` → `dir: &Path`
   - `delete_workspace_from(name: &str, dir: &PathBuf)` → `dir: &Path`
2. Add `use std::path::Path;` if not already imported (check existing imports)
3. Fix the dead code warning for `resolve_workspaces_dir_with`: either:
   - Mark it `pub(crate)` if it's used in tests via the `_with_env` wrapper, OR
   - Add `#[cfg(test)]` if it's only used in tests
   Looking at the code: `resolve_workspaces_dir_with` is called by `resolve_workspaces_dir_with_env` (line 307), which is `pub(crate)` and used in tests. So `resolve_workspaces_dir_with` needs to be `#[cfg(test)]` or the caller chain needs adjustment. Actually since `resolve_workspaces_dir_with_env` calls it, and that function is `pub(crate)`, just check if `resolve_workspaces_dir_with_env` is only used in tests. If so, gate both with `#[cfg(test)]`.
4. Update any call sites if the type change causes issues (callers pass `&resolve_workspaces_dir()` which returns `PathBuf` — `&PathBuf` auto-derefs to `&Path`, so callers should work without changes)
5. Verify: `cargo clippy && cargo clippy --target wasm32-wasip1 && cargo test --lib` — expect 0 warnings
6. Commit: `git add src/workspace.rs && git commit -m "yoyo: fix clippy warnings in workspace.rs"`
