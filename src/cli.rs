//! Command-line interface definition.
use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// The address of the window to minimize. If not provided, minimizes the active window.
    #[arg(conflicts_with = "restore_last")]
    pub window_address: Option<String>,

    /// Restore the last minimized window to the current workspace.
    #[arg(long, short = 'r', action, default_value_t = false)]
    pub restore_last: bool,
}
