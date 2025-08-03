//! Allows parsing of the config file
use anyhow::{Context, Result};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;

// Enum for the restore target, which is safer than a raw string.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum RestoreTarget {
    Active,
    Original,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Config {
    pub launcher: String,
    pub stack_base_directory: String,
    pub restore_to: RestoreTarget,
    pub poll_interval_seconds: u64,
}

// This ensures that Config::default() uses our custom default values.
impl Default for Config {
    fn default() -> Self {
        Self {
            launcher: default_launcher(),
            stack_base_directory: default_stack_base_directory(),
            restore_to: default_restore_target(),
            poll_interval_seconds: default_poll_interval(),
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

fn default_restore_target() -> RestoreTarget {
    RestoreTarget::Active
}

fn default_poll_interval() -> u64 {
    2
}

/// Creates a default configuration file if one does not already exist.
pub fn generate_default_config() -> Result<()> {
    let Some(proj_dirs) = ProjectDirs::from("fr", "denischevalier", "hyprland-minimizer") else {
        anyhow::bail!("Could not find a valid home directory to create config file.");
    };
    let config_dir = proj_dirs.config_dir();
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
