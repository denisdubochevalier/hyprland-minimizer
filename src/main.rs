//! Main application entry point for the hyprland-minimize utility.
mod cli;
mod config;
mod dbus;
mod hyprland;
mod minimize;
mod restore;
mod stack;

use anyhow::{Context, Result};
use clap::Parser;
use figment::{
    Figment,
    providers::{Format, Serialized, Toml},
};
use std::sync::Arc;

use crate::cli::Args;
use crate::config::Config;
use crate::hyprland::{Hyprland, LiveExecutor};
use crate::minimize::{LiveDbus, Minimizer};
use crate::restore::restore_last_minimized;
use crate::stack::Stack;

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    let config: Config = Figment::new()
        // 1. Start with hardcoded defaults
        .merge(Serialized::defaults(Config::default()))
        // 2. Merge the config file (it's okay if it doesn't exist)
        .merge(Toml::file("~/.config/hyprland-minimizer/config.toml"))
        // 3. Merge CLI arguments, which have the highest priority
        .merge(Serialized::defaults(args.clone()))
        .extract()
        .expect("Failed to load configuration");

    let hyprland = Hyprland::new(Arc::new(LiveExecutor));
    let stack = Stack::at_default_path(config.clone())
        .expect("Failed to initialize the application stack. Ensure $USER is set.");

    if args.restore_last {
        return restore_last_minimized(config.clone(), &stack, &hyprland).await;
    }

    let window_info = if let Some(address) = args.window_address {
        hyprland.get_window_by_address(&address)?
    } else {
        hyprland
            .exec("activewindow")
            .context("Failed to get active window. Is a window focused?")?
    };

    let minimizer = Minimizer::new(config.clone(), &stack, window_info, hyprland, &LiveDbus);
    minimizer.minimize().await
}
