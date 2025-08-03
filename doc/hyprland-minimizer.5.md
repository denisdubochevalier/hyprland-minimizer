% hyprland-minimizer(5) | General Commands Manual

# NAME

hyprland-minimizer - Configuration file for the hyprland-minimizer utility.

# DESCRIPTION

The hyprland-minimizer utility uses a TOML configuration file to control its
default behavior. This page documents the available configuration options.

The configuration file is located at
$XDG_CONFIG_HOME/hyprland-minimizer/config.toml
(typically ~/.config/hyprland-minimizer/config.toml).

If the file does not exist, the application will use hardcoded default values.
You can generate a default configuration file by running:
hyprland-minimizer --generate-config-file

# CONFIGURATION

All settings are optional. If a setting is omitted from the file, the default
value will be used.

## launcher

The command used to launch an interactive window selector (e.g., wofi, rofi).
This command must follow the dmenu syntax, meaning it should accept a
newline-separated list of choices on its standard input and print the user's
selection to its standard output.

- **Type:** String
- **Default:** `"wofi -dmenu"`

## stack_base_directory

The base directory where the temporary stack file is stored. The final path wil
l be [stack_base_directory]/hypr-minimizer-stack-[USER].

- **Type:** String
- **Default:** `"/tmp"`

## workspace

The name of the workspace where the minimized windows are moved to.

- **Type:** String
- **Default:** `"special:minimized"`

## restore_to

Determines which workspace a window should be restored to when activated from
the tray icon or interactive menu.

- **Type:** String
- **Values:**
  - `active`: Restores the window to the currently focused workspace.
  - `original`: Restores the window to the workspace it was on when it was minimized.
- **Default:** `"active"`

## poll_interval_seconds

The interval, in seconds, at which the application checks if a minimized window
has been closed or restored externally.

- **Type:** Integer
- **Default:** `2`

## auto_unminimize_on_focus

When set to true, the application will automatically restore the window if during
its poll it detects that it is focused. Use it to have hyprland-minimizer
interact nicely with docks such as hypr-dock.

- **Type:** Boolean
- **Default:** `false`

# EXAMPLES

Here is an example of a config.toml file that uses rofi and restores windows to
their original workspace.

```toml
# ~/.config/hyprland-minimizer/config.toml

launcher = "rofi -dmenu -i -p 'Restore Window:'"
restore_to = "original"
```

# SEE ALSO

hyprland-minimizer(1), rofi(1), wofi(7)

# AUTHORS

Denis Chevalier
