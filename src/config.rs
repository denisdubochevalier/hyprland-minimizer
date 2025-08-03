//! Allows parsing of the config file
use serde::{Deserialize, Serialize};

// Enum for the restore target, which is safer than a raw string.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, Default, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum RestoreTarget {
    #[default]
    Active,
    Original,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Config {
    #[serde(default = "default_launcher")]
    pub launcher: String,

    #[serde(default = "default_stack_base_directory")]
    pub stack_base_directory: String,

    #[serde(default = "default_restore_target")]
    pub restore_to: RestoreTarget,

    #[serde(default = "default_poll_interval")]
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
    "/tmp/".to_string()
}

fn default_restore_target() -> RestoreTarget {
    RestoreTarget::Active
}

fn default_poll_interval() -> u64 {
    2
}
