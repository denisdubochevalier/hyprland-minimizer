//! Functions and data structures for interacting with Hyprland.
use anyhow::{Context, Result, anyhow};
use serde::Deserialize;
use std::process::{Command, Output, Stdio};
use std::sync::Arc;

#[derive(Deserialize, Debug, Clone, PartialEq)]
pub struct Workspace {
    pub id: i32,
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
#[allow(dead_code)]
pub struct WindowInfo {
    pub address: String,
    pub workspace: Workspace,
    pub title: String,
    pub class: String,
}

/// A trait that abstracts the execution of `hyprctl` commands.
/// It must be Send + Sync to be used across threads.
pub trait HyprctlExecutor: Send + Sync {
    fn execute_json(&self, command: &str) -> Result<Output>;
    fn execute_dispatch(&self, command: &str) -> Result<Output>;
}

/// The executor that runs the actual `hyprctl` command.
pub struct LiveExecutor;
impl HyprctlExecutor for LiveExecutor {
    fn execute_json(&self, command: &str) -> Result<Output> {
        Command::new("hyprctl")
            .arg("-j")
            .arg(command)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .with_context(|| format!("Failed to execute hyprctl json command: {command}"))
    }

    fn execute_dispatch(&self, command: &str) -> Result<Output> {
        Command::new("hyprctl")
            .arg("dispatch")
            .arg(command)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .with_context(|| format!("Failed to execute hyprctl dispatch: {command}"))
    }
}

#[derive(Clone)]
pub struct Hyprland {
    executor: Arc<dyn HyprctlExecutor>,
}

impl Hyprland {
    pub fn new(executor: Arc<dyn HyprctlExecutor>) -> Self {
        Hyprland { executor }
    }

    /// Executes a hyprctl command and returns the parsed JSON output.
    pub fn exec<T: for<'de> Deserialize<'de>>(&self, command: &str) -> Result<T> {
        let output = self.executor.execute_json(command)?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("hyprctl command '{command}' failed: {stderr}");
        }

        serde_json::from_slice(&output.stdout)
            .with_context(|| format!("Failed to parse JSON from hyprctl command: {command}"))
    }

    /// Executes a hyprctl dispatch command.
    pub fn dispatch(&self, command: &str) -> Result<()> {
        let output = self.executor.execute_dispatch(command)?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("hyprctl dispatch command '{command}' failed: {stderr}");
        }
        Ok(())
    }

    /// Finds a window by its address from the list of all clients.
    pub fn get_window_by_address(&self, address: &str) -> Result<WindowInfo> {
        let clients: Vec<WindowInfo> = self
            .exec("clients")
            .context("Failed to get client list from Hyprland.")?;
        clients
            .into_iter()
            .find(|c| c.address == address)
            .ok_or_else(|| anyhow!("Could not find a window with address '{address}'"))
    }
}

// --- Unit Tests ---
#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::process::ExitStatusExt;
    use std::process::ExitStatus;

    /// Mock executor for testing.
    #[derive(Default)]
    struct MockExecutor {
        stdout: String,
        is_success: bool,
    }
    impl HyprctlExecutor for MockExecutor {
        fn execute_json(&self, _command: &str) -> Result<Output> {
            Ok(Output {
                status: ExitStatus::from_raw(if self.is_success { 0 } else { 1 }),
                stdout: self.stdout.as_bytes().to_vec(),
                stderr: b"Mock failure".to_vec(),
            })
        }
        fn execute_dispatch(&self, _command: &str) -> Result<Output> {
            Ok(Output {
                status: ExitStatus::from_raw(if self.is_success { 0 } else { 1 }),
                stdout: vec![],
                stderr: vec![],
            })
        }
    }

    #[test]
    fn test_get_window_by_address_success() {
        let mock_json =
            r#"[{"address": "0x456", "workspace": {"id": 2}, "title": "Kitty", "class": "kitty"}]"#;
        let mock_executor = Arc::new(MockExecutor {
            stdout: mock_json.to_string(),
            is_success: true,
        });
        let hyprland = Hyprland::new(mock_executor);

        let result = hyprland.get_window_by_address("0x456");

        assert!(result.is_ok());
        let window = result.unwrap();
        assert_eq!(window.title, "Kitty");
    }

    #[test]
    fn test_hyprctl_command_failure() {
        let mock_executor = Arc::new(MockExecutor {
            stdout: "".to_string(),
            is_success: false, // Simulate a command failure.
        });
        let hyprland = Hyprland::new(mock_executor);

        let result = hyprland.get_window_by_address("any");
        assert!(result.is_err());

        let err_string = format!("{:?}", result.unwrap_err());

        assert!(err_string.contains("Failed to get client list from Hyprland."));
        assert!(err_string.contains("hyprctl command 'clients' failed: Mock failure"));
    }
}
