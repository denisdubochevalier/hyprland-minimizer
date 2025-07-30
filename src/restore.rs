//! Contains the logic for restoring the last minimized window.
use crate::hyprland::{Hyprland, WindowInfo, Workspace};
use crate::stack::Stack;
use anyhow::{Context, Result};

/// Restores the last minimized window from the stack.
pub async fn restore_last_minimized(stack: &Stack, hyprland: &Hyprland) -> Result<()> {
    // Use `if let` to handle the case where the stack is empty, and return early.
    let Some(address) = stack.pop()? else {
        println!("No minimized windows in the stack to restore.");
        return Ok(());
    };

    println!("Restoring last minimized window: {address}");

    let clients: Vec<WindowInfo> = hyprland
        .exec("clients")
        .context("Failed to get client list to verify window existence.")?;

    // Use a guard clause to check if the window is still minimized.
    let is_minimized = clients
        .iter()
        .any(|c| c.address == address && c.workspace.id < 0);

    if !is_minimized {
        println!("Window {address} no longer exists or is not minimized. Stack is clean.");
        return Ok(());
    }

    // The main logic now proceeds without extra indentation.
    let active_workspace: Workspace = hyprland
        .exec("activeworkspace")
        .context("Failed to get active workspace for restoration.")?;

    hyprland.dispatch(&format!(
        "movetoworkspace {},address:{}",
        active_workspace.id, address
    ))?;
    hyprland.dispatch(&format!("focuswindow address:{address}"))?;
    println!("Window restored to workspace {}.", active_workspace.id);

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

    // An RAII guard to manage the mock's lifetime.
    // When this guard is dropped at the end of the test, it restores the real executor.
    struct MockGuard;
    impl Drop for MockGuard {
        fn drop(&mut self) {
            hyprland::EXECUTOR.with(|cell| {
                *cell.borrow_mut() = Box::new(hyprland::LiveExecutor);
            });
        }
    }

    // Helper function to set the mock and return the guard.
    fn set_mock_executor(mock: MockExecutor) -> MockGuard {
        hyprland::EXECUTOR.with(|cell| {
            *cell.borrow_mut() = Box::new(mock);
        });
        MockGuard
    }

    // --- The Tests (FIXED) ---

    #[tokio::test]
    async fn test_restore_with_window_in_special_workspace() -> Result<()> {
        let temp_file = NamedTempFile::new()?;
        let stack = Stack::new(temp_file.path());
        stack.push("0xRESTORE_TEST")?;
        let hyprland = Hyprland::new();

        let mock_executor = MockExecutor::default();
        // Mock responses are popped in reverse order of calls.
        // 1. `hyprctl activeworkspace` will be called second.
        mock_executor.add_json_response(r#"{"id": 3}"#);
        // 2. `hyprctl clients` will be called first.
        // FIXED: Added missing "title" and "class" fields.
        mock_executor.add_json_response(r#"[{"address": "0xRESTORE_TEST", "workspace": {"id": -99}, "title": "Test", "class": "Test"}]"#);

        // Set the mock executor. The guard ensures it's cleaned up after.
        let _guard = set_mock_executor(mock_executor.clone());

        // Directly .await the function.
        restore_last_minimized(&stack, &hyprland).await?;

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
        let hyprland = Hyprland::new();

        let mock_executor = MockExecutor::default();
        // The window is on workspace 2, not a special workspace.
        // FIXED: Added missing "title" and "class" fields.
        mock_executor.add_json_response(r#"[{"address": "0xALREADY_OPEN", "workspace": {"id": 2}, "title": "Test", "class": "Test"}]"#);

        let _guard = set_mock_executor(mock_executor.clone());

        restore_last_minimized(&stack, &hyprland).await?;

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
        let hyprland = Hyprland::new();

        let mock_executor = MockExecutor::default();

        let _guard = set_mock_executor(mock_executor.clone());

        restore_last_minimized(&stack, &hyprland).await?;

        // No commands should be dispatched if the stack is empty.
        assert!(mock_executor.dispatched_commands().is_empty());

        Ok(())
    }
}
