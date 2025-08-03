//! Main application entry point for the hyprland-minimize utility.
mod cli;
mod config;
mod dbus;
mod hyprland;
mod menu;
mod minimize;
mod restore;
mod stack;

use anyhow::{Context, Result};
use clap::Parser;
use directories::ProjectDirs;
use figment::{
    providers::{Format, Serialized, Toml},
    Figment,
};
use std::path::PathBuf;
use std::sync::Arc;

use crate::cli::Args;
use crate::config::{generate_default_config, get_config_dir, Config};
use crate::hyprland::{Hyprland, LiveExecutor};
use crate::menu::Menu;
use crate::minimize::{LiveDbus, Minimizer};
use crate::restore::restore_last_minimized;
use crate::stack::Stack;

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Check for the generate_config_file flag first.
    if args.generate_config_file {
        // If the flag is present, generate the file and exit.
        let config_dir = get_config_dir()?;
        return generate_default_config(&config_dir);
    }

    // Find the config file path using the directories crate.
    let config_path =
        if let Some(proj_dirs) = ProjectDirs::from("fr", "denischevalier", "hyprland-minimizer") {
            proj_dirs.config_dir().join("config.toml")
        } else {
            // Fallback for environments where home directory can't be determined.
            PathBuf::from("hyprland-minimizer.toml")
        };

    let config: Config = Figment::new()
        // 1. Start with hardcoded defaults
        .merge(Serialized::defaults(Config::default()))
        // 2. Merge the config file (it's okay if it doesn't exist)
        .merge(Toml::file(&config_path))
        // 3. Merge CLI arguments, which have the highest priority
        .merge(Serialized::defaults(args.clone()))
        .extract()
        .expect("Failed to load configuration");

    let hyprland = Hyprland::new(Arc::new(LiveExecutor));
    let stack = Stack::at_default_path(config.clone())
        .expect("Failed to initialize the application stack. Ensure $USER is set.");

    if args.menu {
        let menu = Menu::new(&config, &stack, &hyprland);
        return menu.show_and_restore().await;
    }

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
