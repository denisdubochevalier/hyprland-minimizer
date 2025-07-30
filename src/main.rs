//! Main application entry point for the hyprland-minimize utility.
mod cli;
mod dbus;
mod hyprland;
mod minimize;
mod restore;
mod stack;

use anyhow::{Context, Result};
use clap::Parser;
use std::sync::Arc;

use crate::cli::Args;
use crate::hyprland::{Hyprland, LiveExecutor};
use crate::minimize::{LiveDbus, Minimizer};
use crate::restore::restore_last_minimized;
use crate::stack::Stack;

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    let hyprland = Hyprland::new(Arc::new(LiveExecutor));
    let stack = Stack::at_default_path();

    if args.restore_last {
        return restore_last_minimized(&stack, &hyprland).await;
    }

    let window_info = if let Some(address) = args.window_address {
        hyprland.get_window_by_address(&address)?
    } else {
        hyprland
            .exec("activewindow")
            .context("Failed to get active window. Is a window focused?")?
    };

    let minimizer = Minimizer::new(&stack, window_info, hyprland, &LiveDbus);
    minimizer.minimize().await
}
