//! Allows parsing of the config file
use crate::cli::RestoreTarget;

use anyhow::{Context, Result};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Config {
    pub launcher: Option<String>,
    pub stack_base_directory: Option<String>,
    pub workspace: Option<String>,
    pub restore_to: Option<RestoreTarget>,
    pub poll_interval_ms: Option<u64>,
    pub auto_unminimize_on_focus: Option<bool>,
}

// This ensures that Config::default() uses our custom default values.
impl Default for Config {
    fn default() -> Self {
        Self {
            launcher: Some(default_launcher()),
            stack_base_directory: Some(default_stack_base_directory()),
            workspace: Some(default_workspace()),
            restore_to: Some(default_restore_target()),
            poll_interval_ms: Some(default_poll_interval()),
            auto_unminimize_on_focus: Some(default_unminimize_on_focus()),
        }
    }
}
// --- Default value functions for serde ---

fn default_launcher() -> String {
    "wofi -dmenu".to_string()
}

fn default_stack_base_directory() -> String {
    "/tmp".to_string()
}

fn default_workspace() -> String {
    "special:minimized".to_string()
}

fn default_restore_target() -> RestoreTarget {
    RestoreTarget::Active
}

fn default_poll_interval() -> u64 {
    2000
}

fn default_unminimize_on_focus() -> bool {
    false
}

/// Finds the project's configuration directory using XDG standards.
pub fn get_config_dir() -> Result<PathBuf> {
    let Some(proj_dirs) = ProjectDirs::from("fr", "denischevalier", "hyprland-minimizer") else {
        anyhow::bail!("Could not find a valid home directory.");
    };
    Ok(proj_dirs.config_dir().to_path_buf())
}

/// Creates a default configuration file if one does not already exist.
pub fn generate_default_config(config_dir: &Path) -> Result<()> {
    let config_path = config_dir.join("config.toml");

    if config_path.exists() {
        println!("Config file already exists at: {:?}", config_path);
        println!("Not overwriting.");
        return Ok(());
    }

    // Create the parent directory if it doesn't exist
    fs::create_dir_all(config_dir)
        .with_context(|| format!("Failed to create config directory at {:?}", config_dir))?;

    // Serialize the default Config struct to a TOML string
    let default_config = Config::default();
    let toml_string = toml::to_string_pretty(&default_config)
        .context("Failed to serialize default config to TOML.")?;

    // Write the string to the new file
    let mut file = fs::File::create(&config_path)
        .with_context(|| format!("Failed to create config file at {:?}", config_path))?;

    file.write_all(toml_string.as_bytes())
        .context("Failed to write default config to file.")?;

    println!(
        "Successfully created default config file at: {:?}",
        config_path
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_generate_config_creates_file() -> Result<()> {
        // --- 1. Setup ---
        // Create a temporary directory for the test.
        let temp_dir = tempdir()?;
        let config_dir = temp_dir.path();
        let config_file_path = config_dir.join("config.toml");

        // --- 2. Execution ---
        // Run the function to generate the config.
        generate_default_config(config_dir)?;

        // --- 3. Assertions ---
        // Check that the file was created.
        assert!(config_file_path.exists(), "Config file should be created");

        // Check that the file content is correct.
        let content = fs::read_to_string(config_file_path)?;
        let expected_content = toml::to_string_pretty(&Config::default())?;
        assert_eq!(content, expected_content);

        Ok(())
    }

    #[test]
    fn test_generate_config_does_not_overwrite() -> Result<()> {
        // --- 1. Setup ---
        let temp_dir = tempdir()?;
        let config_dir = temp_dir.path();
        let config_file_path = config_dir.join("config.toml");

        // Create a dummy file with different content.
        let initial_content = "do_not_overwrite = true";
        fs::write(&config_file_path, initial_content)?;

        // --- 2. Execution ---
        // Run the function again.
        generate_default_config(config_dir)?;

        // --- 3. Assertions ---
        // Check that the file content has not changed.
        let final_content = fs::read_to_string(config_file_path)?;
        assert_eq!(initial_content, final_content);

        Ok(())
    }
}
