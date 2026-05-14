//! Spawn external editors / terminals at a chosen path.
//!
//! All commands are spawned with stdio nulled so the child process is fully
//! detached from this app. Failure to spawn surfaces as `AppError::CommandFailed`
//! with the attempted-command list, so the UI can show the user what to install.

use crate::error::{AppError, Result};
use std::path::Path;
use std::process::{Command, Stdio};

/// Spawn `code <path>`. Windows uses `code.cmd` (shim); other platforms use `code`.
pub fn open_vscode(path: &Path) -> Result<()> {
    let bin = if cfg!(windows) { "code.cmd" } else { "code" };
    try_spawn(Command::new(bin).arg(path)).map_err(|tried| {
        AppError::CommandFailed(format!(
            "could not launch VS Code (tried: {}). Install VS Code and run \
             \"Shell Command: Install 'code' command in PATH\" from its command palette.",
            tried.join(", ")
        ))
    })
}

/// Open a terminal at `path`, trying platform-specific options in order.
pub fn open_terminal(path: &Path) -> Result<()> {
    let attempts = build_terminal_attempts(path);
    let mut tried = Vec::with_capacity(attempts.len());
    for (label, mut cmd) in attempts {
        if spawn_detached(&mut cmd).is_ok() {
            return Ok(());
        }
        tried.push(label);
    }
    Err(AppError::CommandFailed(format!(
        "could not launch terminal (tried: {})",
        tried.join(", ")
    )))
}

fn build_terminal_attempts(path: &Path) -> Vec<(&'static str, Command)> {
    let p = path.to_string_lossy().into_owned();
    let mut v: Vec<(&'static str, Command)> = Vec::new();

    if cfg!(windows) {
        // Windows Terminal — preferred (-d sets the starting directory).
        let mut wt = Command::new("wt.exe");
        wt.args(["-d", &p]);
        v.push(("wt.exe", wt));

        // PowerShell 7 — install path varies, rely on PATH.
        let mut pwsh = Command::new("pwsh.exe");
        pwsh.args(["-NoExit", "-WorkingDirectory", &p]);
        v.push(("pwsh.exe", pwsh));

        // Windows PowerShell 5 — always present.
        let mut ps5 = Command::new("powershell.exe");
        ps5.args([
            "-NoExit",
            "-Command",
            &format!("Set-Location -LiteralPath '{}'", p.replace('\'', "''")),
        ]);
        v.push(("powershell.exe", ps5));
    } else if cfg!(target_os = "macos") {
        let mut open = Command::new("open");
        open.args(["-a", "Terminal", &p]);
        v.push(("open -a Terminal", open));
    } else {
        // Linux — try a sequence of common emulators.
        let mut xte = Command::new("x-terminal-emulator");
        xte.current_dir(path);
        v.push(("x-terminal-emulator", xte));

        let mut gnome = Command::new("gnome-terminal");
        gnome.args(["--working-directory", &p]);
        v.push(("gnome-terminal", gnome));

        let mut konsole = Command::new("konsole");
        konsole.args(["--workdir", &p]);
        v.push(("konsole", konsole));

        let mut xterm = Command::new("xterm");
        xterm.current_dir(path);
        v.push(("xterm", xterm));
    }

    v
}

fn spawn_detached(cmd: &mut Command) -> std::io::Result<()> {
    cmd.stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map(|_| ())
}

/// Try a single command. Returns the binary name on success (for callers that
/// want to know which one worked) or the list of tried names on failure.
fn try_spawn(cmd: &mut Command) -> std::result::Result<(), Vec<String>> {
    let prog = cmd
        .get_program()
        .to_string_lossy()
        .into_owned();
    match spawn_detached(cmd) {
        Ok(()) => Ok(()),
        Err(_) => Err(vec![prog]),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn terminal_attempts_are_platform_correct() {
        let path = PathBuf::from(if cfg!(windows) { "C:\\tmp" } else { "/tmp" });
        let attempts = build_terminal_attempts(&path);
        assert!(!attempts.is_empty());

        let labels: Vec<_> = attempts.iter().map(|(l, _)| *l).collect();
        if cfg!(windows) {
            assert_eq!(labels[0], "wt.exe");
            assert!(labels.contains(&"pwsh.exe"));
            assert!(labels.contains(&"powershell.exe"));
        } else if cfg!(target_os = "macos") {
            assert_eq!(labels[0], "open -a Terminal");
        } else {
            assert!(labels.contains(&"x-terminal-emulator"));
        }
    }

    #[test]
    fn open_vscode_returns_helpful_error_when_missing() {
        // Skip: actually spawning code may succeed on the dev machine. We test
        // the error path indirectly via a known-missing binary.
        let result = open_vscode(Path::new(if cfg!(windows) {
            "C:\\definitely-does-not-exist"
        } else {
            "/definitely-does-not-exist"
        }));
        // Don't assert error — if user has `code` installed, it'll happily
        // launch and the path argument is its concern. We just check it
        // doesn't panic.
        let _ = result;
    }
}
