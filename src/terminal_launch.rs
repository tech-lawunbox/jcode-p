use anyhow::Result;
pub use jcode_terminal_launch::{
    SpawnAttempt, TerminalCommand, detected_resume_terminal, resume_terminal_candidates, sh_escape,
    shell_command,
};
use std::path::Path;

pub fn spawn_command_in_new_terminal(command: &TerminalCommand, cwd: &Path) -> Result<bool> {
    jcode_terminal_launch::spawn_command_in_new_terminal_with(command, cwd, |cmd| {
        let program = cmd.get_program().to_string_lossy().to_string();
        let mut child = crate::platform::spawn_detached(cmd)?;
        // Wait briefly to catch immediate failures (e.g., tmux split-window pane too small).
        // Short-lived commands (tmux, open, osascript) exit quickly — if they fail, propagate
        // the error so the caller tries the next terminal candidate instead of silently
        // proceeding with a failed spawn.
        let deadline = std::time::Instant::now() + std::time::Duration::from_millis(200);
        loop {
            match child.try_wait() {
                Ok(Some(status)) => {
                    if !status.success() {
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::Other,
                            format!("{program} exited with {status}"),
                        ));
                    }
                    return Ok(());
                }
                Ok(None) => {
                    if std::time::Instant::now() >= deadline {
                        return Ok(());
                    }
                    std::thread::sleep(std::time::Duration::from_millis(10));
                }
                Err(e) => return Err(e),
            }
        }
    })
}
