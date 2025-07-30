//! Main application entry point for the hyprland-minimize utility.
mod cli;
mod dbus;
mod hyprland;
mod minimize;
mod restore;
mod stack;

use anyhow::{Context, Result};
use clap::Parser;

use crate::cli::Args;
use crate::hyprland::{get_window_by_address, hyprctl};
use crate::minimize::run_minimize_workflow;
use crate::restore::restore_last_minimized;
use crate::stack::Stack;

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    let stack = Stack::at_default_path();

    if args.restore_last {
        return restore_last_minimized(&stack).await;
    }

    let window_info = if let Some(address) = args.window_address {
        get_window_by_address(&address)?
    } else {
        hyprctl("activewindow").context("Failed to get active window. Is a window focused?")?
    };

    run_minimize_workflow(&stack, window_info).await
}
