//! Contains the logic for restoring the last minimized window.
use crate::config::{Config, RestoreTarget};
use crate::hyprland::{Hyprland, WindowInfo, Workspace};
use crate::stack::Stack;

use anyhow::{Context, Result};

/// Restores the last minimized window from the stack.
pub async fn restore_last_minimized(
    config: Config,
    stack: &Stack,
    hyprland: &Hyprland,
) -> Result<()> {
    let Some(address) = stack.pop()? else {
        println!("No minimized windows in the stack to restore.");
        return Ok(());
    };

    println!("Restoring last minimized window: {address}");

    let clients: Vec<WindowInfo> = hyprland
        .exec("clients")
        .context("Failed to get client list to verify window existence.")?;

    let is_minimized = clients
        .iter()
        .any(|c| c.address == address && c.workspace.id < 0);

    if !is_minimized {
        println!("Window {address} no longer exists or is not minimized. Stack is clean.");
        return Ok(());
    }

    if config.restore_to == RestoreTarget::Active {
        let active_workspace: Workspace = hyprland
            .exec("activeworkspace")
            .context("Failed to get active workspace for restoration.")?;

        hyprland.dispatch(&format!(
            "movetoworkspace {},address:{}",
            active_workspace.id, address
        ))?;
        println!("Window restored to workspace {}.", active_workspace.id);
    }
    hyprland.dispatch(&format!("focuswindow address:{address}"))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hyprland;
    use std::os::unix::process::ExitStatusExt;
    use std::process::{ExitStatus, Output};
    use std::sync::{Arc, Mutex};
    use tempfile::NamedTempFile;

    // --- Mocking Setup ---

    #[derive(Default, Clone)]
    struct MockExecutor {
        dispatched_commands: Arc<Mutex<Vec<String>>>,
        json_responses: Arc<Mutex<Vec<String>>>,
    }
    impl MockExecutor {
        fn add_json_response(&self, json: &str) {
            self.json_responses.lock().unwrap().push(json.to_string());
        }
        fn dispatched_commands(&self) -> Vec<String> {
            self.dispatched_commands.lock().unwrap().clone()
        }
    }
    impl hyprland::HyprctlExecutor for MockExecutor {
        fn execute_json(&self, _command: &str) -> Result<Output> {
            let response = self
                .json_responses
                .lock()
                .unwrap()
                .pop()
                .unwrap_or_default();
            Ok(Output {
                status: ExitStatus::from_raw(0),
                stdout: response.as_bytes().to_vec(),
                stderr: vec![],
            })
        }
        fn execute_dispatch(&self, command: &str) -> Result<Output> {
            self.dispatched_commands
                .lock()
                .unwrap()
                .push(command.to_string());
            Ok(Output {
                status: ExitStatus::from_raw(0),
                stdout: vec![],
                stderr: vec![],
            })
        }
    }

    #[tokio::test]
    async fn test_restore_with_window_in_special_workspace() -> Result<()> {
        let temp_file = NamedTempFile::new()?;
        let stack = Stack::new(temp_file.path());
        stack.push("0xRESTORE_TEST")?;

        let mock_executor = Arc::new(MockExecutor::default());
        let hyprland = Hyprland::new(mock_executor.clone() as Arc<dyn hyprland::HyprctlExecutor>);

        // Mock responses are popped in reverse order of calls.
        // 1. `hyprctl activeworkspace` will be called second.
        mock_executor.add_json_response(r#"{"id": 3}"#);
        // 2. `hyprctl clients` will be called first.
        mock_executor.add_json_response(r#"[{"address": "0xRESTORE_TEST", "workspace": {"id": -99}, "title": "Test", "class": "Test"}]"#);

        // Directly .await the function with the mock-powered hyprland instance.
        restore_last_minimized(Config::default(), &stack, &hyprland).await?;

        let dispatched = mock_executor.dispatched_commands();
        assert_eq!(dispatched.len(), 2);
        assert_eq!(dispatched[0], "movetoworkspace 3,address:0xRESTORE_TEST");
        assert_eq!(dispatched[1], "focuswindow address:0xRESTORE_TEST");

        // The stack should be empty after a successful restore.
        assert!(stack.pop()?.is_none());

        Ok(())
    }

    #[tokio::test]
    async fn test_restore_when_window_not_minimized() -> Result<()> {
        let temp_file = NamedTempFile::new()?;
        let stack = Stack::new(temp_file.path());
        stack.push("0xALREADY_OPEN")?;

        let mock_executor = Arc::new(MockExecutor::default());
        let hyprland = Hyprland::new(mock_executor.clone() as Arc<dyn hyprland::HyprctlExecutor>);

        // The window is on workspace 2, not a special workspace.
        mock_executor.add_json_response(r#"[{"address": "0xALREADY_OPEN", "workspace": {"id": 2}, "title": "Test", "class": "Test"}]"#);

        restore_last_minimized(Config::default(), &stack, &hyprland).await?;

        // No commands should be dispatched if the window isn't minimized.
        assert!(mock_executor.dispatched_commands().is_empty());
        // The stack should still be empty as the item was popped and consumed.
        assert!(stack.pop()?.is_none());

        Ok(())
    }

    #[tokio::test]
    async fn test_restore_with_empty_stack() -> Result<()> {
        let temp_file = NamedTempFile::new()?;
        let stack = Stack::new(temp_file.path()); // An empty stack

        let mock_executor = Arc::new(MockExecutor::default());
        let hyprland = Hyprland::new(mock_executor.clone() as Arc<dyn hyprland::HyprctlExecutor>);

        restore_last_minimized(Config::default(), &stack, &hyprland).await?;

        // No commands should be dispatched if the stack is empty.
        assert!(mock_executor.dispatched_commands().is_empty());

        Ok(())
    }
}
