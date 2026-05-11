# BlackArch Hyprland TUI

A terminal user interface for browsing, searching, installing, removing, and launching BlackArch tools on Arch Linux and Hyprland-oriented desktop environments.

The application uses `ratatui` and `crossterm` for the TUI, `pacman` for package discovery and package operations, and `pkexec` for privileged install/remove actions.

## Features

- Browse BlackArch tool categories and packages.
- Filter tools by category, installation status, favorites, and recent usage.
- Search tools directly from the TUI.
- View package details, versions, descriptions, categories, and available executables.
- Install or update a selected package through `pkexec pacman -S --needed --noconfirm`.
- Queue multiple packages and install them in one batch.
- Remove installed packages through `pkexec pacman -Rns --noconfirm`.
- Launch installed tools in a supported terminal emulator.
- Store favorites and recent tools locally.
- Cache package and category metadata for faster startup.
- Provide CLI commands for scripting and diagnostics.

## Requirements

- Arch Linux or an Arch-based distribution.
- Rust toolchain with Cargo.
- `pacman` available in `PATH`.
- BlackArch repository configured and synchronized.
- `pkexec` from polkit for install and remove operations.
- A running polkit authentication agent for graphical privilege prompts.
- A supported terminal emulator for launching tools:
  - `kitty`
  - `foot`
  - `alacritty`
  - `wezterm`

The default terminal is `kitty`.

## Build

```bash
cargo build --release
```

The release binary will be available at:

```text
target/release/blackarch-hyprland-tui
```

## Run

Start the TUI:

```bash
cargo run
```

Or run the compiled binary:

```bash
./target/release/blackarch-hyprland-tui
```

Run diagnostics before using package operations:

```bash
cargo run -- doctor
```

## CLI Commands

```bash
cargo run -- doctor
cargo run -- categories
cargo run -- tools
cargo run -- tools --category blackarch-webapp
cargo run -- search sqlmap
cargo run -- info sqlmap
cargo run -- executables sqlmap
cargo run -- run sqlmap
cargo run -- favorites
cargo run -- favorite sqlmap
cargo run -- unfavorite sqlmap
cargo run -- sync-cache
```

Command overview:

| Command | Description |
| --- | --- |
| `doctor` | Check pacman, polkit, BlackArch package data, cache, and config status. |
| `categories` | Print available BlackArch groups/categories. |
| `tools` | Print all available BlackArch tools. |
| `tools --category <name>` | Print tools from a specific category. |
| `search <query>` | Search BlackArch packages and print JSON results. |
| `info <package>` | Print detailed package metadata as JSON. |
| `executables <package>` | Print executable names for an installed package. |
| `run <package>` | Launch an installed package executable in the configured terminal. |
| `favorites` | Print favorite package names. |
| `favorite <package>` | Add a package to favorites. |
| `unfavorite <package>` | Remove a package from favorites. |
| `sync-cache` | Refresh category and tool caches. |

## TUI Keybindings

| Key | Action |
| --- | --- |
| `Up` / `Down` | Move through tools, categories, menu items, or queue items. |
| `Tab` | Switch focus between panes or leave search mode. |
| `/` | Enter search mode. |
| `Enter` | Open the action menu or confirm the selected modal action. |
| `a` | Add or remove the selected package from the install queue. |
| `I` | Open the install queue modal. |
| `s` | Sync cache. |
| `d` | Refresh selected tool details. |
| `r` | Run the selected installed tool. |
| `i` | Open install confirmation for the selected package. |
| `x` | Open remove confirmation for the selected package. |
| `f` | Toggle favorite status for the selected package. |
| `c` | Copy the selected command. |
| `?` | Show a short help message in the status bar. |
| `Esc` | Clear errors, close modals, or leave search mode. |
| `q` | Quit the TUI or close active menus/modals. |

## Configuration

On first run, the application creates a default configuration file:

```text
$XDG_CONFIG_HOME/blackarch-hypr-tui/config.toml
```

If `XDG_CONFIG_HOME` is not set, this usually resolves to:

```text
~/.config/blackarch-hypr-tui/config.toml
```

Default configuration:

```toml
[ui]
theme = "catppuccin-mocha"
show_icons = true

[pacman]
prefer_cache = true
sync_on_start = false
max_package_info_jobs = 8

[terminal]
program = "kitty"
runner_class = "blackarch-tool-runner"
hold_after_run = false
```

Supported `terminal.program` values are `kitty`, `foot`, `alacritty`, and `wezterm`.

`terminal.hold_after_run` is currently not supported by the CLI run command without shell wrapping and should remain `false`.

## Data Locations

Cache files are stored under:

```text
$XDG_CACHE_HOME/blackarch-hypr-tui
```

This usually resolves to:

```text
~/.cache/blackarch-hypr-tui
```

User state is stored under:

```text
$XDG_DATA_HOME/blackarch-hypr-tui
```

This usually resolves to:

```text
~/.local/share/blackarch-hypr-tui
```

User state includes:

- `favorites.json`
- `recent.json`

## Package Operations

Install operations use:

```text
pkexec pacman -S --needed --noconfirm <packages>
```

Remove operations use:

```text
pkexec pacman -Rns --noconfirm <package>
```

The application validates package and executable names before building commands. Privileged package operations require `pkexec` and an active polkit authentication agent.

## Development

Run tests:

```bash
cargo test
```

Check formatting:

```bash
cargo fmt --check
```

Run Clippy:

```bash
cargo clippy --all-targets --all-features
```

## Troubleshooting

Run:

```bash
cargo run -- doctor
```

Common issues:

- `pkexec not found`: install polkit and ensure `pkexec` is available in `PATH`.
- `no polkit authentication agent detected`: start a polkit authentication agent in the current desktop session.
- No BlackArch groups or packages found: verify that the BlackArch repository is configured and run `sudo pacman -Sy`.
- Terminal launch fails: set `[terminal].program` to one of the supported terminal emulators installed on your system.
