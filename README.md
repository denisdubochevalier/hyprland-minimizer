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

## How It Works

The utility uses a simple but effective trick:

1. When a window is "minimized," it is moved to a special, hidden workspace in
   Hyprland (specifically, `special:minimized`).
1. A D-Bus service is created for the window, allowing it to register as a
   `StatusNotifierItem` with your system tray.
1. A temporary file (`/tmp/hypr-minimizer-stack`) keeps track of the order of
   minimized windows, enabling the "restore last" feature.
1. When the tray icon is activated, a `hyprctl` command is dispatched to move
   the window back to a visible workspace.

## Installation

Currently, you must build the project from the source.

### Prerequisites

- [Rust toolchain](https://www.rust-lang.org/tools/install)
- `hyprctl` (comes with Hyprland)
- A status bar with a system tray module (e.g., Waybar)

### Build Steps

1. Clone the repository:

   ```sh
   git clone [https://github.com/your-username/hyprland-minimizer.git](https://github.com/your-username/hyprland-minimizer.git)
   cd hyprland-minimizer
   ```

1. Build the release binary:

   ```sh
   cargo build --release
   ```

1. The executable will be located at `target/release/hyprland-minimizer`. You
   can copy it to a directory in your `$PATH`, such as `~/.local/bin/`.

## Usage

The tool has two main modes of operation.

### To Minimize a Window

Bind the command to a hotkey in your `hyprland.conf`.

In `hyprland.conf`

```ini
bind = $mainMod, M, exec, hyprland-minimizer
```

Pressing `$mainMod + M` will minimize the currently active window.

### To Restore the Last Minimized Window

You can bind the `--restore-last` (or `-r`) flag to another hotkey.

In `hyprland.conf`

```ini
bind = $mainMod SHIFT, M, exec, hyprland-minimizer --restore-last
```

This will pop the most recently minimized window from the stack and restore it
to your active workspace.

## Contributing

Contributions are welcome!

Feel free to open an issue to discuss a new feature or submit a pull request
with your changes.

## License

This project is licensed under the BSD 2-Clauses License.
