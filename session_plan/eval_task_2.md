## Evaluation: Fix clippy warnings — &PathBuf → &Path in workspace.rs

Verdict: PASS
Reason: All four `&PathBuf` → `&Path` signature changes are correct and idiomatic; `resolve_workspaces_dir_with` is properly gated with `#[cfg(test)]` since its only caller (`resolve_workspaces_dir_with_env`) is also `#[cfg(test)]`. `cargo clippy` (host + wasm32-wasip1) produces zero warnings and all 133 tests pass. No forbidden APIs in plugin code.
