Title: Create workspace data model and file persistence (`src/workspace.rs`)
Files: src/workspace.rs, src/lib.rs
Issue: none

## Context

Step 6 of the roadmap is "Workspace management." This task creates the foundational data model and file persistence layer. CLI commands (`zellai new`, `zellai attach`, `zellai list`, `zellai kill`) are a separate follow-up task.

SCHEMA.md defines `src/workspace.rs` responsibilities:
- Persists workspace layouts to `<user-data-dir>/zellai/workspaces/<name>.json`
- `save(name)` — serialize current pane layout + agent assignments
- `restore(name)` — reconstruct the layout, launch agents

The vision doc defines workspace templates: single agent, team, review, research.

## Implementation

### Data model

Create `src/workspace.rs` with these types:

```rust
use serde::{Deserialize, Serialize};

/// A saved workspace definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workspace {
    /// Workspace name (used as filename: `<name>.json`)
    pub name: String,
    /// Working directory for the workspace
    pub working_dir: String,
    /// Template this workspace was created from (if any)
    pub template: Option<WorkspaceTemplate>,
    /// Pane definitions
    pub panes: Vec<PaneConfig>,
    /// When this workspace was last saved (Unix epoch seconds)
    pub saved_at: u64,
}

/// Pre-defined workspace templates from the vision doc.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum WorkspaceTemplate {
    SingleAgent,
    Team,
    Review,
    Research,
}

/// Configuration for a single pane in a workspace.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaneConfig {
    /// Pane name/label shown in the sidebar
    pub name: String,
    /// Agent to run in this pane
    pub agent: String,
    /// Command to run (e.g., ["claude"], ["zellai", "run", "--agent", "codex", "--", "codex"])
    pub command: Vec<String>,
    /// Optional initial prompt/instruction
    pub prompt: Option<String>,
    /// Pane direction: "horizontal" or "vertical" split (relative to previous pane)
    pub direction: PaneDirection,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "lowercase")]
pub enum PaneDirection {
    #[default]
    Horizontal,
    Vertical,
}
```

### Persistence functions (pure, no Zellij API)

```rust
/// Default workspaces directory: `~/.local/share/zellai/workspaces`
pub fn default_workspaces_dir() -> String { ... }

/// Resolve workspaces dir, respecting $ZELLAI_DATA_DIR env var.
pub fn resolve_workspaces_dir() -> PathBuf { ... }

/// Save a workspace to `<workspaces_dir>/<name>.json`.
/// Creates the directory if it doesn't exist. Writes atomically (write to .tmp, rename).
pub fn save_workspace(workspace: &Workspace) -> Result<(), String> { ... }

/// Load a workspace by name from `<workspaces_dir>/<name>.json`.
pub fn load_workspace(name: &str) -> Result<Workspace, String> { ... }

/// List all saved workspace names (scan directory for .json files).
pub fn list_workspaces() -> Result<Vec<String>, String> { ... }

/// Delete a saved workspace file.
pub fn delete_workspace(name: &str) -> Result<(), String> { ... }

/// Create a workspace from a template.
pub fn from_template(name: &str, template: WorkspaceTemplate, working_dir: &str) -> Workspace { ... }
```

Templates produce these default configurations:
- **SingleAgent**: 1 pane, claude agent
- **Team**: orchestrator + 2 workers (from teams config defaults)
- **Review**: 2 panes — one claude for code review, one for the main agent
- **Research**: 1 pane with gemini agent (research role)

### Wire into lib.rs

- Uncomment `pub mod workspace;` in `src/lib.rs`
- The module must compile for both WASM and native targets, so persistence functions that use `std::fs` must be gated with `#[cfg(not(target_arch = "wasm32"))]`
- The data model types (Workspace, PaneConfig, etc.) should be available on all targets

### Unit tests

Write tests in the module for:
- Serialize/deserialize round-trip for `Workspace`
- `from_template` produces correct pane counts for each template
- `default_workspaces_dir` returns expected path
- For `save_workspace` / `load_workspace` / `list_workspaces` / `delete_workspace`: use a tempdir (via `std::env::temp_dir()` + random suffix) — test the full save-load-list-delete lifecycle
- Validate workspace name (reject empty, slashes, dots — prevent path traversal)

## Verification

```sh
cargo build --target wasm32-wasip1 && cargo clippy --target wasm32-wasip1 && cargo test --lib
```
