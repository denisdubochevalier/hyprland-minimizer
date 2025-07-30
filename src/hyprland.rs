//! Functions and data structures for interacting with Hyprland.
use anyhow::{anyhow, Context, Result};
use serde::Deserialize;
use std::cell::RefCell;
use std::process::{Command, Output, Stdio};

// --- Hyprland Data Structures ---
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

// --- Abstraction for Testability ---

/// A trait that abstracts the execution of `hyprctl` commands.
pub trait HyprctlExecutor {
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

// --- thread_local for holding the current executor ---
thread_local! {
    // We use a RefCell to allow for interior mutability.
    // This lets us swap the executor at runtime for tests.
    pub static EXECUTOR: RefCell<Box<dyn HyprctlExecutor>> = RefCell::new(Box::new(LiveExecutor));
}

// --- Hyprland Interaction Functions ---

/// Executes a hyprctl command and returns the parsed JSON output.
pub fn hyprctl<T: for<'de> Deserialize<'de>>(command: &str) -> Result<T> {
    EXECUTOR.with(|executor_cell| {
        let executor = executor_cell.borrow();
        let output = executor.execute_json(command)?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("hyprctl command '{}' failed: {}", command, stderr);
        }

        serde_json::from_slice(&output.stdout)
            .with_context(|| format!("Failed to parse JSON from hyprctl command: {command}"))
    })
}

/// Executes a hyprctl dispatch command.
pub fn hyprctl_dispatch(command: &str) -> Result<()> {
    EXECUTOR.with(|executor_cell| {
        let executor = executor_cell.borrow();
        let output = executor.execute_dispatch(command)?;
        if !output.status.success() {
            anyhow::bail!("hyprctl dispatch command '{}' failed", command);
        }
        Ok(())
    })
}

/// Finds a window by its address from the list of all clients.
pub fn get_window_by_address(address: &str) -> Result<WindowInfo> {
    let clients: Vec<WindowInfo> =
        hyprctl("clients").context("Failed to get client list from Hyprland.")?;
    clients
        .into_iter()
        .find(|c| c.address == address)
        .ok_or_else(|| anyhow!("Could not find a window with address '{}'", address))
}

// --- Unit Tests ---
#[cfg(test)]
mod tests {
    use super::*; // Import from parent
    use std::os::unix::process::ExitStatusExt;
    use std::process::ExitStatus;

    /// Mock executor for testing, same as before.
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

    /// Helper function to temporarily set a mock executor for a test.
    fn with_mock_executor(mock: MockExecutor, test_fn: impl FnOnce()) {
        EXECUTOR.with(|cell| {
            // Replace the live executor with our mock one
            *cell.borrow_mut() = Box::new(mock);
        });
        test_fn();
        EXECUTOR.with(|cell| {
            // Restore the live executor after the test
            *cell.borrow_mut() = Box::new(LiveExecutor);
        });
    }

    #[test]
    fn test_get_window_by_address_success() {
        let mock_json =
            r#"[{"address": "0x456", "workspace": {"id": 2}, "title": "Kitty", "class": "kitty"}]"#;
        let mock_executor = MockExecutor {
            stdout: mock_json.to_string(),
            is_success: true,
        };

        with_mock_executor(mock_executor, || {
            // Now we call the function with its original, clean signature!
            let result = get_window_by_address("0x456");

            assert!(result.is_ok());
            let window = result.unwrap();
            assert_eq!(window.title, "Kitty");
        });
    }

    #[test]
    fn test_hyprctl_command_failure() {
        let mock_executor = MockExecutor {
            stdout: "".to_string(),
            is_success: false, // Simulate a command failure.
        };

        with_mock_executor(mock_executor, || {
            let result = get_window_by_address("any");
            assert!(result.is_err());

            let err_string = format!("{:?}", result.unwrap_err());

            // The assertions now check the full error report.
            assert!(err_string.contains("Failed to get client list from Hyprland."));
            assert!(err_string.contains("hyprctl command 'clients' failed: Mock failure"));
        });
    }
}
