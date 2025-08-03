//! Handles the interactive window selection logic using a dmenu-style launcher.
use crate::config::Config;
use crate::hyprland::{Hyprland, WindowInfo, Workspace};
use crate::stack::Stack;

use anyhow::{Context, Result};
use std::io::{Read, Write};
use std::process::{Command, Stdio};

/// A struct to manage the interactive window selection menu.
pub struct Menu<'a> {
    config: &'a Config,
    stack: &'a Stack,
    hyprland: &'a Hyprland,
}

impl<'a> Menu<'a> {
    /// Creates a new Menu instance.
    pub fn new(config: &'a Config, stack: &'a Stack, hyprland: &'a Hyprland) -> Self {
        Menu {
            config,
            stack,
            hyprland,
        }
    }

    /// Presents a list of minimized windows to the user and restores the selected one.
    pub async fn show_and_restore(&self) -> Result<()> {
        let windows = self.stack.minimized(self.hyprland)?;
        if windows.is_empty() {
            println!("No windows to restore.");
            return Ok(());
        }

        let choices = windows
            .iter()
            .map(|w| format!("{} ({})", w.title, w.address))
            .collect::<Vec<_>>()
            .join("\n");

        let selection = self.run_launcher(&choices)?;
        if selection.is_empty() {
            println!("No window selected.");
            return Ok(());
        }

        // Parse the address from the selection string "Title (Address)".
        if let Some(address) = self.parse_address_from_selection(&selection) {
            if let Some(selected_window) = windows.into_iter().find(|w| w.address == address) {
                self.restore_selected_window(&selected_window)?;
                println!("Restored window: {}", selected_window.title);
            } else {
                println!("No window selected or selection was invalid.");
            }
        } else {
            println!(
                "Could not parse window address from selection: '{}'",
                selection
            );
        }

        Ok(())
    }

    /// Executes the launcher command, pipes the choices to it, and returns the user's selection.
    fn run_launcher(&self, choices: &str) -> Result<String> {
        let mut child = Command::new("sh")
            .arg("-c")
            .arg(&self.config.launcher.clone().unwrap())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .with_context(|| {
                format!(
                    "Failed to spawn launcher command: '{}'",
                    self.config.launcher.clone().unwrap()
                )
            })?;

        if let Some(mut stdin) = child.stdin.take() {
            stdin
                .write_all(choices.as_bytes())
                .context("Failed to write to launcher stdin")?;
        }

        let mut output = String::new();
        if let Some(mut stdout) = child.stdout.take() {
            stdout
                .read_to_string(&mut output)
                .context("Failed to read from launcher stdout")?;
        }

        let status = child.wait().context("Launcher command failed to run")?;
        if !status.success() {
            return Ok(String::new());
        }

        Ok(output.trim().to_string())
    }

    /// Restores the selected window to the active workspace and removes it from the stack.
    fn restore_selected_window(&self, window: &WindowInfo) -> Result<()> {
        let active_workspace: Workspace = self.hyprland.exec("activeworkspace")?;
        self.hyprland.dispatch(&format!(
            "movetoworkspace {},address:{}",
            active_workspace.id, window.address
        ))?;
        self.hyprland
            .dispatch(&format!("focuswindow address:{}", window.address))?;
        self.stack.remove(&window.address)
    }

    /// Extracts the window address from a string formatted as "Title (Address)".
    fn parse_address_from_selection(&self, selection: &str) -> Option<String> {
        selection
            .rfind('(')
            .and_then(|start| selection.rfind(')').map(|end| (start, end)))
            .map(|(start, end)| selection[start + 1..end].to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hyprland::{self, Workspace};
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

    #[test]
    fn test_parse_address_from_selection() {
        let config = Config::default();
        let stack = Stack::new(""); // Path doesn't matter for this test
        let mock_executor = Arc::new(MockExecutor::default());
        let hyprland = Hyprland::new(mock_executor);
        let menu = Menu::new(&config, &stack, &hyprland);

        assert_eq!(
            menu.parse_address_from_selection("Title (0x123abc)"),
            Some("0x123abc".to_string())
        );
        assert_eq!(menu.parse_address_from_selection("No Address"), None);
        assert_eq!(
            menu.parse_address_from_selection("Mismatched (Brackets]"),
            None
        );
        assert_eq!(
            menu.parse_address_from_selection("Empty ()"),
            Some("".to_string())
        );
    }

    #[test]
    fn test_restore_selected_window() -> Result<()> {
        // --- Setup ---
        let temp_file = NamedTempFile::new()?;
        let stack = Stack::new(temp_file.path());
        let config = Config::default();
        let mock_executor = Arc::new(MockExecutor::default());
        let hyprland = Hyprland::new(mock_executor.clone());
        let menu = Menu::new(&config, &stack, &hyprland);

        let window_to_restore = WindowInfo {
            address: "0xRESTORE".to_string(),
            title: "Test".to_string(),
            class: "Test".to_string(),
            workspace: Workspace { id: 1 },
        };

        // Mock the hyprland response for `activeworkspace`
        mock_executor.add_json_response(r#"{"id": 5}"#);

        // --- Execute ---
        menu.restore_selected_window(&window_to_restore)?;

        // --- Assert ---
        let dispatched = mock_executor.dispatched_commands();
        assert_eq!(dispatched.len(), 2);
        assert_eq!(dispatched[0], "movetoworkspace 5,address:0xRESTORE");
        assert_eq!(dispatched[1], "focuswindow address:0xRESTORE");

        Ok(())
    }
}
