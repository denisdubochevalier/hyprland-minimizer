/// Command-line interface definition.
use clap::{Parser, ValueEnum};
use serde::{Deserialize, Serialize};

// Enum for the restore target, which is safer than a raw string.
#[derive(ValueEnum, Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum RestoreTarget {
    Active,
    Original,
}

#[derive(Parser, Debug, Serialize, Clone)]
#[command(
    author,
    version,
    about = "A utility to minimize Hyprland windows to the system tray.",
    long_about = "A small utility to add true 'minimize to tray' functionality to Hyprland, allowing windows to be hidden and restored from a system tray icon.

It works by moving windows to a special workspace and creating a D-Bus service to register a tray icon with Waybar or other status bars."
)]
#[serde(rename_all = "lowercase")]
pub struct Args {
    /// The launcher used for menu selection of windows to restore. Must follow dmenu
    /// syntax.
    #[arg(long, short = 'l')]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub launcher: Option<String>,

    /// The address of the window to minimize. If not provided, minimizes the active window.
    #[arg(long, short = 'w' , conflicts_with_all = ["restore_last", "generate_config_file"])]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub window_address: Option<String>,

    /// The workspace to restore the window to: active or original.
    #[arg(long, short = 't')]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub restore_to: Option<RestoreTarget>,

    /// The base directory to store the stack tmp file.
    #[arg(long, short = 's')]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stack_base_directory: Option<String>,

    /// The workspace where the minimized windows are moved to.
    #[arg(long, short = 'u')]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace: Option<String>,

    /// The poll interval used to check weither the window is still minimized (seconds).
    #[arg(long, short = 'p')]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub poll_interval_seconds: Option<u64>,

    /// Unminimize on focus. Set it to true to integrate with docks like hypr-dock.
    #[arg(long, short = 'a', action, default_value_t = false)]
    pub auto_unminimize_on_focus: bool,

    /// Restore the last minimized window to the current workspace.
    #[arg(long, short = 'r', action, default_value_t = false, conflicts_with_all = ["generate_config_file", "menu", "window_address"])]
    pub restore_last: bool,

    /// Generate config file.
    #[arg(long, short = 'g', action, default_value_t = false, conflicts_with_all = ["menu", "window_address", "restore_last"])]
    pub generate_config_file: bool,

    /// Open selection menu.
    #[arg(long, short = 'm', action, default_value_t = false, conflicts_with_all = ["window_address", "restore_last", "generate_config_file"])]
    pub menu: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_args_serialization_skips_nones() {
        // --- 1. Setup ---
        // Create an instance of Args where some optional fields have values
        // and others are None (the default).
        let args = Args {
            window_address: Some("0x123".to_string()),
            launcher: None,
            stack_base_directory: None,
            workspace: None,
            restore_to: Some(RestoreTarget::Original),
            poll_interval_seconds: None,
            auto_unminimize_on_focus: false,
            restore_last: false,
            generate_config_file: false,
            menu: false,
        };

        // --- 2. Execution ---
        // Serialize the struct to a serde_json::Value, which is easy to inspect.
        let json_value = serde_json::to_value(&args).unwrap();

        // --- 3. Assertions ---
        // Check that the serialized JSON is what we expect.
        let expected_json = json!({
            "window_address": "0x123",
            "restore_to": "original",
            "auto_unminimize_on_focus": false,
            "restore_last": false,
            "menu": false,
            "generate_config_file": false
        });

        assert_eq!(json_value, expected_json);

        // Explicitly check that the `None` fields were not included.
        let obj = json_value.as_object().unwrap();
        assert!(!obj.contains_key("launcher"));
        assert!(!obj.contains_key("stack_base_directory"));
        assert!(!obj.contains_key("poll_interval_seconds"));
        assert!(!obj.contains_key("command"));
    }
}
