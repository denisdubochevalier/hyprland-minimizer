//! Command-line interface definition.
use clap::Parser;
use serde::Serialize;

#[derive(Parser, Debug, Serialize, Clone)]
#[command(author, version, about, long_about = None)]
#[serde(rename_all = "lowercase")]
pub struct Args {
    /// The launcher used for menu selection of windows to restore. Must follow dmenu
    /// syntax.
    #[arg(long, short = 'l')]
    pub launcher: Option<String>,

    /// The address of the window to minimize. If not provided, minimizes the active window.
    #[arg(long, short = 'w' , conflicts_with_all = ["restore_last", "generate_config_file"])]
    pub window_address: Option<String>,

    /// The workspace to restore the window to: active or original.
    #[arg(long, short = 'r')]
    pub restore_to: Option<String>,

    /// The base directory to store the stack tmp file.
    #[arg(long, short = 's')]
    pub stack_base_directory: Option<String>,

    /// The poll interval used to check weither the window is still minimized (seconds).
    #[arg(long, short = 'p')]
    pub poll_interval_seconds: u64,

    /// Restore the last minimized window to the current workspace.
    #[arg(long, short = 'r', action, default_value_t = false)]
    pub restore_last: bool,

    /// Generate config file.
    #[arg(long, short = 'g', default_value_t = false, conflicts_with_all = ["window_address", "restore_last"])]
    pub generate_config_file: bool,
}
