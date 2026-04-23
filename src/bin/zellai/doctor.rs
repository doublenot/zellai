//! `zellai doctor` — check environment and diagnose issues.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use super::status_writer;

/// Run the `zellai doctor` command.
///
/// Checks the user's environment and reports issues. Returns `Ok(())` on
/// success (even if some checks fail — they're informational). Returns
/// `Err` only on unexpected fatal errors.
pub fn run() -> Result<(), String> {
    println!("zellai doctor\n");

    check_zellij();
    check_wasm_plugin();
    check_sessions_dir();
    check_claude_hooks();
    check_config();
    check_git();
    check_gh();

    Ok(())
}

// ---------------------------------------------------------------------------
// Individual checks
// ---------------------------------------------------------------------------

/// Check if Zellij is installed and report its version.
fn check_zellij() {
    match run_command("zellij", &["--version"]) {
        Some(output) => {
            let version = output.trim().to_string();
            println!("  ✓ {version}");
        }
        None => {
            println!("  ✗ zellij not found");
        }
    }
}

/// Check if the WASM plugin binary exists (informational only).
fn check_wasm_plugin() {
    let debug_path = Path::new("target/wasm32-wasip1/debug/zellai.wasm");
    let release_path = Path::new("target/wasm32-wasip1/release/zellai.wasm");

    if release_path.exists() {
        println!("  ✓ plugin WASM found (release)");
    } else if debug_path.exists() {
        println!("  ✓ plugin WASM found (debug)");
    } else {
        println!("  ⊘ plugin WASM not built (run `cargo build --target wasm32-wasip1`)");
    }
}

/// Check the sessions directory: exists, is writable, count active sessions.
fn check_sessions_dir() {
    let sessions_dir = status_writer::resolve_sessions_dir();
    let display_path = display_home_relative(&sessions_dir);

    if !sessions_dir.exists() {
        // Try to create it
        match fs::create_dir_all(&sessions_dir) {
            Ok(()) => {
                println!("  ✓ sessions dir ({display_path}) created");
            }
            Err(e) => {
                println!("  ✗ sessions dir ({display_path}) missing and could not create: {e}");
                return;
            }
        }
    } else {
        println!("  ✓ sessions dir ({display_path}) exists");
    }

    // Check writable by trying to create and remove a temp file
    let probe = sessions_dir.join(".zellai-doctor-probe");
    match fs::write(&probe, b"probe") {
        Ok(()) => {
            let _ = fs::remove_file(&probe);
        }
        Err(_) => {
            println!("  ✗ sessions dir ({display_path}) is not writable");
        }
    }

    // Count active session files
    match count_json_files(&sessions_dir) {
        0 => println!("    0 active sessions"),
        1 => println!("    1 active session"),
        n => println!("    {n} active sessions"),
    }
}

/// Check if Claude Code hooks are installed.
fn check_claude_hooks() {
    let cwd = match std::env::current_dir() {
        Ok(p) => p,
        Err(_) => {
            println!("  ⊘ could not determine current directory");
            return;
        }
    };

    let claude_dir = cwd.join(".claude");
    if !claude_dir.is_dir() {
        println!("  ⊘ no .claude/ directory (Claude hooks not applicable)");
        return;
    }

    let hooks_dir = claude_dir.join("hooks");
    let expected_hooks = ["on-stop.sh", "on-notification.sh", "on-post-tool-use.sh"];

    let mut found = 0;
    let mut missing: Vec<&str> = Vec::new();

    for hook in &expected_hooks {
        if hooks_dir.join(hook).exists() {
            found += 1;
        } else {
            missing.push(hook);
        }
    }

    if found == expected_hooks.len() {
        println!("  ✓ Claude hooks installed (.claude/hooks/)");
    } else if found > 0 {
        println!(
            "  ⊘ Claude hooks partially installed ({found}/{} present)",
            expected_hooks.len()
        );
        for m in &missing {
            println!("    missing: .claude/hooks/{m}");
        }
    } else {
        println!("  ✗ Claude hooks not installed (run `zellai init`)");
    }
}

/// Check for zellai.toml configuration file.
fn check_config() {
    let cwd_config = std::env::current_dir().ok().map(|p| p.join("zellai.toml"));
    let home_config = std::env::var("HOME").ok().map(|h| {
        PathBuf::from(h)
            .join(".config")
            .join("zellai")
            .join("zellai.toml")
    });

    // Try current directory first, then ~/.config/zellai/
    let found_path = cwd_config
        .as_ref()
        .filter(|p| p.exists())
        .or_else(|| home_config.as_ref().filter(|p| p.exists()));

    match found_path {
        Some(path) => match fs::read_to_string(path) {
            Ok(content) => match zellai::config::parse_config(&content) {
                Ok(_) => {
                    let display = display_home_relative(path);
                    println!("  ✓ {display} is valid");
                }
                Err(e) => {
                    let display = display_home_relative(path);
                    println!("  ✗ {display} has errors: {e}");
                }
            },
            Err(e) => {
                let display = display_home_relative(path);
                println!("  ✗ could not read {display}: {e}");
            }
        },
        None => {
            println!("  ✗ zellai.toml not found (using defaults)");
        }
    }
}

/// Check if git is available.
fn check_git() {
    match run_command("git", &["--version"]) {
        Some(output) => {
            // Output is like "git version 2.43.0"
            let version = output.trim().to_string();
            // Extract just the version number
            let short = version.strip_prefix("git version ").unwrap_or(&version);
            println!("  ✓ git {short}");
        }
        None => {
            println!("  ✗ git not found");
        }
    }
}

/// Check if gh CLI is available.
fn check_gh() {
    match run_command("gh", &["--version"]) {
        Some(output) => {
            // Output is like "gh version 2.42.0 (2024-01-15)\nhttps://..."
            let first_line = output.lines().next().unwrap_or(&output);
            let version = first_line
                .strip_prefix("gh version ")
                .and_then(|s| s.split_whitespace().next())
                .unwrap_or(first_line.trim());
            println!("  ✓ gh {version}");
        }
        None => {
            println!("  ✗ gh CLI not found (optional — PR/CI features disabled)");
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Run a command and return its stdout if successful.
fn run_command(program: &str, args: &[&str]) -> Option<String> {
    Command::new(program)
        .args(args)
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                String::from_utf8(o.stdout).ok()
            } else {
                None
            }
        })
}

/// Count `.json` files in a directory.
fn count_json_files(dir: &Path) -> usize {
    fs::read_dir(dir)
        .map(|entries| {
            entries
                .filter_map(|e| e.ok())
                .filter(|e| e.path().extension().is_some_and(|ext| ext == "json"))
                .count()
        })
        .unwrap_or(0)
}

/// Display a path relative to $HOME for cleaner output.
fn display_home_relative(path: &Path) -> String {
    if let Ok(home) = std::env::var("HOME") {
        let home_path = Path::new(&home);
        if let Ok(rel) = path.strip_prefix(home_path) {
            return format!("~/{}", rel.display());
        }
    }
    path.display().to_string()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_count_json_files_empty() {
        let dir = std::env::temp_dir().join("zellai-doctor-test-empty");
        let _ = fs::create_dir_all(&dir);

        // Clean up any previous test files
        if let Ok(entries) = fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let _ = fs::remove_file(entry.path());
            }
        }

        assert_eq!(count_json_files(&dir), 0);
        let _ = fs::remove_dir(&dir);
    }

    #[test]
    fn test_count_json_files_with_files() {
        let dir = std::env::temp_dir().join("zellai-doctor-test-json");
        let _ = fs::create_dir_all(&dir);

        // Clean up any previous test files
        if let Ok(entries) = fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let _ = fs::remove_file(entry.path());
            }
        }

        fs::write(dir.join("session1.json"), b"{}").unwrap();
        fs::write(dir.join("session2.json"), b"{}").unwrap();
        fs::write(dir.join("readme.txt"), b"not json").unwrap();

        assert_eq!(count_json_files(&dir), 2);

        // Clean up
        let _ = fs::remove_file(dir.join("session1.json"));
        let _ = fs::remove_file(dir.join("session2.json"));
        let _ = fs::remove_file(dir.join("readme.txt"));
        let _ = fs::remove_dir(&dir);
    }

    #[test]
    fn test_count_json_files_nonexistent() {
        let dir = Path::new("/tmp/zellai-doctor-nonexistent-dir-12345");
        assert_eq!(count_json_files(dir), 0);
    }

    #[test]
    fn test_display_home_relative() {
        // This test depends on $HOME being set, which it should be in CI
        if let Ok(home) = std::env::var("HOME") {
            let path = PathBuf::from(&home)
                .join(".local")
                .join("share")
                .join("zellai");
            let display = display_home_relative(&path);
            assert_eq!(display, "~/.local/share/zellai");
        }
    }

    #[test]
    fn test_display_home_relative_no_home_prefix() {
        let path = Path::new("/tmp/some/path");
        let display = display_home_relative(path);
        assert_eq!(display, "/tmp/some/path");
    }
}
