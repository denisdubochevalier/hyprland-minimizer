# Hyprland Minimizer

A small utility to add true "minimize to tray" functionality to Hyprland,
allowing windows to be hidden and restored from a system tray icon.

---

## ⚠️ Warning: Experimental Project ⚠️

This project is currently in a very early and unstable phase. It
began as a proof-of-concept and is now undergoing significant refactoring and
development.

**Use it at your own risk.** While the goal is to create a stable and reliable
tool, the current version may have bugs, unexpected behavior, or breaking
changes in future updates. It is primarily a learning project for exploring
Rust and is not yet recommended for critical daily use.

---

## Features

- **Minimize to Tray:** Hides the active window and creates a corresponding icon
  in your system tray (e.g., Waybar's `tray` module).
- **Restore Window:** Click the tray icon to restore the window to your active workspace.
- **Restore Last Minimized:** A command-line option to restore the most recently
  minimized window without needing to use the tray.
- **Context Menu:** Right-click the tray icon for options like restoring to the
  original workspace or closing the window directly.
- **Interactive Restore**: Use a dmenu-style launcher (like rofi or wofi) to select
  any minimized window to restore.
- **Configuration**: Customize behavior through a simple TOML configuration file.

## How It Works

The utility uses a simple but effective trick:

1. When a window is "minimized," it is moved to a special, hidden workspace in
   Hyprland (specifically, `special:minimized`).
1. A D-Bus service is created for the window, allowing it to register as a
   `StatusNotifierItem` with your system tray.
1. A temporary file (`/tmp/hypr-minimizer-stack-your user`) keeps track of the
   order of minimized windows, enabling the "restore last" feature.
1. When the tray icon is activated, a `hyprctl` command is dispatched to move
   the window back to a visible workspace.

## Installation

The recommended way to install is to build from source using the provided
Makefile, which will also install the man pages.

### Prerequisites

- [Rust toolchain](https://www.rust-lang.org/tools/install)
- `pandoc` (for generating man pages)
- `hyprctl` (comes with Hyprland)
- A status bar with a system tray module (e.g., Waybar)

### Build and Install Steps

1. Clone the repository:

   ```sh
   git clone [https://github.com/denisdubochevalier/hyprland-minimizer.git](https://github.com/denisdubochevalier/hyprland-minimizer.git)
   cd hyprland-minimizer
   ```

1. Build the application:

   ```sh
   make
   ```

   To build without generating man pages (which avoids the `pandoc` dependency),
   run:

   ```sh
   make build-no-man
   ```

1. Install the files:

   ```sh
   sudo make install
   ```

   You can also install to a local directory by specifying:

   ```sh
   make install PREFIX=~/.local
   ```

## Configuration

The first time you run the application, you can generate a default configuration
file.

1. Generate the config:

   ```sh
   hyprland-minimizer --generate-config-file
   ```

1. This will create a file at `~/.config/hyprland-minimizer/config.toml` with
   default settings you can customize. See `man 5 hyprland-minimizer` for details
   on all available options.

## Usage

The application has several modes. For detailed information on all commands and
flags, you can view the man pages installed on your system:

```sh
man 1 hyprland-minimizer # Command documentation
man 5 hyprland-minimizer # Configuration documentation
```

For daily use, you will likely bind the main functions to hotkeys in your `hyprland.conf`.

### Keybindings

Bind the commands to hotkeys in your `hyprland.conf`.

```ini
# In hyprland.conf

# Minimize the active window
bind = $mainMod, M, exec, hyprland-minimizer

# Restore the last minimized window
bind = $mainMod SHIFT, M, exec, hyprland-minimizer --restore-last

# Interactively select a window to restore
bind = $mainMod, C, exec, hyprland-minimizer --menu
```

## Contributing

Contributions are welcome!

Feel free to open an issue to discuss a new feature or submit a pull request
with your changes.

## License

This project is licensed under the BSD 2-Clauses License.
