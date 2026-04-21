//! `zellai init` — auto-detect Claude Code projects and install zellai hook scripts.

use std::fs;
use std::path::Path;

/// Hook script content, embedded at compile time.
const ON_STOP_SH: &str = include_str!("../../../hooks/on-stop.sh");
const ON_NOTIFICATION_SH: &str = include_str!("../../../hooks/on-notification.sh");
const ON_POST_TOOL_USE_SH: &str = include_str!("../../../hooks/on-post-tool-use.sh");

/// Hooks to install: (filename, content).
const HOOKS: &[(&str, &str)] = &[
    ("on-stop.sh", ON_STOP_SH),
    ("on-notification.sh", ON_NOTIFICATION_SH),
    ("on-post-tool-use.sh", ON_POST_TOOL_USE_SH),
];

/// Run the `zellai init` command.
///
/// Returns `Ok(())` on success or an error message on failure.
pub fn run(force: bool) -> Result<(), String> {
    let cwd =
        std::env::current_dir().map_err(|e| format!("Failed to get current directory: {e}"))?;
    let claude_dir = cwd.join(".claude");

    // Step 1: detect Claude Code project
    if !claude_dir.is_dir() {
        return Err(
            "No .claude/ directory found. Are you in a Claude Code project?\n\
             Hint: create it with `mkdir .claude` or run `zellai init` from your project root."
                .to_string(),
        );
    }

    // Step 2: create hooks directory
    let hooks_dir = claude_dir.join("hooks");
    fs::create_dir_all(&hooks_dir)
        .map_err(|e| format!("Failed to create {}: {e}", hooks_dir.display()))?;

    // Step 3: install hook scripts
    let mut installed: Vec<String> = Vec::new();
    let mut skipped: Vec<String> = Vec::new();

    for &(filename, content) in HOOKS {
        let target = hooks_dir.join(filename);
        let action = install_hook(&target, content, force);
        match action {
            HookAction::Installed => {
                write_hook(&target, content)?;
                installed.push(filename.to_string());
            }
            HookAction::Overwritten => {
                write_hook(&target, content)?;
                installed.push(format!("{filename} (overwritten)"));
            }
            HookAction::Skipped => {
                eprintln!(
                    "Skipping {}: existing hook not managed by zellai. Use --force to overwrite.",
                    target.display()
                );
                skipped.push(filename.to_string());
            }
        }
    }

    // Step 4: print summary
    if !installed.is_empty() {
        println!("Installed hooks:");
        for name in &installed {
            println!("  ✓ .claude/hooks/{name}");
        }
    }
    if !skipped.is_empty() {
        println!("Skipped hooks:");
        for name in &skipped {
            println!("  ⊘ .claude/hooks/{name}");
        }
    }
    println!();
    println!(
        "Done! Hook scripts installed. Set ZELLAI_SESSION_ID in your environment to activate."
    );

    Ok(())
}

enum HookAction {
    /// File doesn't exist — fresh install.
    Installed,
    /// File exists but is ours (contains "zellai") or --force is set — overwrite.
    Overwritten,
    /// File exists, is NOT ours, and --force is not set — skip.
    Skipped,
}

/// Decide what to do with a hook file.
fn install_hook(target: &Path, _content: &str, force: bool) -> HookAction {
    if !target.exists() {
        return HookAction::Installed;
    }

    if force {
        return HookAction::Overwritten;
    }

    // Read existing content and check if it's managed by zellai
    match fs::read_to_string(target) {
        Ok(existing) => {
            if existing.to_ascii_lowercase().contains("zellai") {
                HookAction::Overwritten
            } else {
                HookAction::Skipped
            }
        }
        // Can't read it — treat as not ours, skip
        Err(_) => HookAction::Skipped,
    }
}

/// Write hook content to a file and make it executable (0o755 on Unix).
fn write_hook(target: &Path, content: &str) -> Result<(), String> {
    fs::write(target, content).map_err(|e| format!("Failed to write {}: {e}", target.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = fs::Permissions::from_mode(0o755);
        fs::set_permissions(target, perms)
            .map_err(|e| format!("Failed to chmod {}: {e}", target.display()))?;
    }
    Ok(())
}
