# App Launcher (al)

A minimal, fast application launcher for Windows with [GlazeWM](https://github.com/glzr-tech/glazewm) integration. Inspired by rofi and PowerToys Run, but built for the Windows tiling window manager ecosystem.

## Features

- **Instant Toggle** - Resident daemon; running `al.exe` again toggles it over a named pipe
- **System Tray** - Lives in the tray between uses, with no taskbar button
- **Minimal UI** - Just a search bar and results
- **Prefix Search** - Shows only apps whose name starts with what you type
- **Pinned Apps** - Favorite apps always appear at the top
- **GlazeWM Colors** - Border, selection, and background follow your GlazeWM border colors
- **Keyboard First** - Arrows, Enter, and Escape
- **App Discovery** - Indexes Start Menu and desktop shortcuts plus Store/MSIX apps, cached for fast startup

## Requirements

- Windows 10/11
- Rust 1.90+
- GlazeWM (optional, for the hotkey and startup integration)

## Installation

```bash
git clone https://github.com/mdeforge/app-launcher.git
cd app-launcher
cargo build --release
```

The binary is at `target/release/al.exe`.

## GlazeWM Integration

Add this to your `~/.glzr/glazewm/config.yaml`, replacing the path with wherever `al.exe` lives:

```yaml
general:
  startup_commands:
    - 'shell-exec C:\path\to\app-launcher\target\release\al.exe --hidden'

keybindings:
  - commands: ['shell-exec C:\path\to\app-launcher\target\release\al.exe']
    bindings: ['alt+a']

window_rules:
  - commands: ['ignore']
    match:
      - window_process: { equals: 'al' }
```

- **Startup** - `--hidden` starts the daemon in the tray without showing the window. If `al` is already running, it does nothing.
- **Hotkey** - Toggles the launcher. If the daemon isn't running yet, the first press starts it.
- **Window rule** - Keeps GlazeWM from tiling the launcher window.

Bind `al.exe` directly rather than wrapping it in `powershell` or `cmd`. Console programs open a terminal window that steals focus, and the launcher hides itself when it loses focus.

See `glazewm-config.yaml` for a commented version of this snippet.

## Usage

| Input | Action |
|-------|--------|
| Hotkey (e.g. `Alt+A`) | Toggle launcher |
| Type | Show apps whose name starts with the typed text (case-insensitive) |
| `Arrow Up/Down` | Move selection |
| `Enter` / click | Launch app and hide |
| `Escape` / click elsewhere | Hide launcher |
| Tray icon left-click | Open launcher |
| Tray icon right-click | Open or Quit |

### Command Line

| Command | Effect |
|---------|--------|
| `al.exe` | Start the daemon and show the launcher, or toggle it if already running |
| `al.exe --hidden` | Start the daemon in the tray; no effect if already running |

## Configuration

**Colors** come from `~/.glzr/glazewm/config.yaml`, read once when the daemon starts:

- `window_effects.focused_window.border.color` - window outline, selected row, and tray icon
- `window_effects.other_windows.border.color` - panel background

If the file is missing, a border is disabled, or a color is unset, that color falls back to the built-in theme. Quit and restart `al` to pick up changes.

**Pinned apps** are set in `src/app.rs`:

```rust
const PINNED_APPS: &[&str] = &[
    "Visual Studio Code",
    "Visual Studio 2022",
    "Microsoft Teams",
    "Outlook",
    "Microsoft Excel",
    "Visio",
    "PowerPoint",
];
```

**Window size** and fallback colors are in `src/ui/theme.rs`.

**App cache** is stored at `%LOCALAPPDATA%\al\apps_cache.json` and refreshed in the background each time the daemon starts. Icons are cached with the app list, so they show as soon as the launcher opens.

## Architecture

```
src/
├── main.rs           # Entry point: single-instance mutex, startup flags, window setup
├── app.rs            # App state, UI rendering, message handling
├── ipc.rs            # Named pipe server/client for toggle commands
├── discovery/        # Start Menu, desktop, and Store app scanning; JSON cache
├── search/           # Prefix search
├── platform/
│   ├── glazewm.rs    # Reads border colors from the GlazeWM config
│   ├── icons.rs      # App icon extraction via the Windows Shell
│   └── tray.rs       # System tray icon and menu
└── ui/
    └── theme.rs      # Dimensions, colors, styling constants
```

### Key Design Decisions

- **Single Instance** - A named mutex ensures only one daemon runs
- **IPC** - A second `al.exe` sends commands to the daemon over a named pipe, then exits
- **Daemon in the Tray** - The launcher stays resident for instant response and is hidden, not minimized, between uses
- **Shared Command Channel** - The pipe server and the tray icon (on its own Win32 message-loop thread) feed the same command channel into the app

## Development

```bash
# Debug build
cargo build

# Release build
cargo build --release

# Tests
cargo test
```

Both builds use the Windows GUI subsystem, so neither opens a console. `cargo build` fails to replace `al.exe` while the daemon is running; quit it from the tray first.

## Tech Stack

- **Language**: Rust
- **GUI Framework**: [Iced](https://iced.rs/) 0.13
- **Windows API**: [windows](https://docs.rs/windows/) crate
- **System Tray**: [tray-icon](https://docs.rs/tray-icon/)
- **GlazeWM Config**: [yaml-rust2](https://docs.rs/yaml-rust2/)
- **Async Runtime**: Tokio
- **Serialization**: Serde/JSON

## License

MIT License - see LICENSE file for details.
