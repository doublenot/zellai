//! `zellai run <command>` — generic wrapper that writes status files for any agent.
//!
//! Spawns the child process, writes status files periodically, and cleans up on exit.

use std::process::{Command, ExitStatus};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

use crate::status_writer::{self, StatusWriter};

/// How often to refresh the status file (git info, timestamp) while the child runs.
const UPDATE_INTERVAL: Duration = Duration::from_secs(5);

/// Run a command with zellai status tracking.
///
/// Returns `Ok(())` on success or an error message on failure.
pub fn run(agent: String, command: Vec<String>) -> Result<(), String> {
    if command.is_empty() {
        return Err(
            "No command specified. Usage: zellai run [--agent NAME] -- <command> [args...]"
                .to_string(),
        );
    }

    let session_id = status_writer::generate_session_id();
    let sessions_dir = status_writer::resolve_sessions_dir();

    // Auto-detect agent from command[0] if not explicitly set
    let agent = if agent == "unknown" {
        status_writer::detect_agent(&command[0]).to_string()
    } else {
        agent
    };

    let writer = StatusWriter::new(session_id.clone(), agent, sessions_dir);

    // Create sessions directory
    std::fs::create_dir_all(
        writer
            .status_file_path()
            .parent()
            .unwrap_or(std::path::Path::new("/")),
    )
    .map_err(|e| format!("Failed to create sessions directory: {e}"))?;

    // Write initial status
    writer
        .write_status("thinking", None, false)
        .map_err(|e| format!("Failed to write initial status: {e}"))?;

    // Spawn the child process
    let mut child = Command::new(&command[0])
        .args(&command[1..])
        .env("ZELLAI_SESSION_ID", &session_id)
        .spawn()
        .map_err(|e| {
            // Clean up status file on spawn failure
            let _ = writer.cleanup();
            format!("Failed to start '{}': {e}", command[0])
        })?;

    // Background thread for periodic status updates
    let running = Arc::new(AtomicBool::new(true));
    let running_clone = Arc::clone(&running);

    // Clone what the background thread needs
    let bg_session_id = writer.session_id().to_string();
    let bg_sessions_dir = writer.status_file_path().parent().unwrap().to_path_buf();
    let bg_agent = command[0].clone();
    let bg_agent_name = if status_writer::detect_agent(&bg_agent) != "unknown" {
        status_writer::detect_agent(&bg_agent).to_string()
    } else {
        "unknown".to_string()
    };

    let bg_thread = thread::spawn(move || {
        let bg_writer = StatusWriter::new(bg_session_id, bg_agent_name, bg_sessions_dir);
        while running_clone.load(Ordering::Relaxed) {
            thread::sleep(UPDATE_INTERVAL);
            if running_clone.load(Ordering::Relaxed) {
                let _ = bg_writer.write_status("thinking", None, false);
            }
        }
    });

    // Wait for child to exit
    let exit_status = child.wait();

    // Stop the background updater regardless of wait result
    running.store(false, Ordering::Relaxed);
    let _ = bg_thread.join();

    let exit_status = exit_status.map_err(|e| {
        let _ = writer.cleanup();
        format!("Failed to wait for child process: {e}")
    })?;

    // Write final status with exit info
    let exit_message = format_exit_message(&exit_status);
    let _ = writer.write_status("idle", Some(&exit_message), false);

    // Clean up the status file
    let _ = writer.cleanup();

    // Exit with the child's exit code
    std::process::exit(exit_status.code().unwrap_or(1));
}

/// Format exit status as a human-readable message.
fn format_exit_message(status: &ExitStatus) -> String {
    match status.code() {
        Some(code) => format!("Exited with code {code}"),
        None => {
            // Process was killed by a signal
            #[cfg(unix)]
            {
                use std::os::unix::process::ExitStatusExt;
                if let Some(sig) = status.signal() {
                    return format!("Killed by signal {sig}");
                }
            }
            "Killed by signal".to_string()
        }
    }
}
