# App Launcher (al)

A modern, blazing-fast application launcher for Windows with seamless [GlazeWM](https://github.com/glzr-tech/glazewm) integration. Inspired by rofi and PowerToys Run, but built specifically for the Windows tiling window manager ecosystem.

## Features

- **Instant Launch** - Resident daemon; re-running `al.exe` toggles it over a named pipe
- **Fuzzy Search** - Find apps quickly with typo-tolerant matching
- **Pinned Apps** - Favorite apps always appear at the top
- **Modern UI** - Transparent window, wallpaper background, clock overlay
- **Keyboard First** - Full keyboard navigation (arrows, enter, escape)
- **App Discovery** - Automatically indexes Start Menu shortcuts
- **Caching** - Lightning-fast subsequent launches
- **GlazeWM Ready** - Pre-configured integration out of the box

## Requirements

- Windows 10/11
- Rust toolchain (1.70+)
- GlazeWM (optional, for keybinding integration)

## Installation

### From Source

```bash
git clone https://github.com/mdeforge/app-launcher.git
cd app-launcher
cargo build --release
```

The binary will be at `target/release/al.exe`.

### Manual Setup

1. Build the release binary
2. Add the binary directory to your PATH, or keep it in this directory
3. Configure GlazeWM (see below)
4. Run `al.exe` once to generate the app cache

## GlazeWM Integration

Add this to your `~/.glzr/glazewm/config.yaml`:

```yaml
keybindings:
  - commands: ["shell-exec C:\\path\\to\\app-launcher\\target\\release\\al.exe"]
    bindings: ["alt+a"]

window_rules:
  - commands: ["ignore"]
    match:
      - window_process: { equals: "al" }
```

Replace the path with wherever `al.exe` lives. Press `Alt+A` to toggle the launcher: the first press starts the daemon, and later presses run a short-lived `al.exe` that sends `toggle` to the daemon and exits.

Bind `al.exe` directly rather than wrapping it in `powershell` or `cmd`. Console programs open a terminal window that steals focus, and the launcher hides itself when it loses focus.

## Usage

### Keyboard Shortcuts

| Key | Action |
|-----|--------|
| `Alt+A` | Toggle launcher (from GlazeWM) |
| `Type to search` | Filter applications |
| `Arrow Up/Down` | Navigate results |
| `Enter` | Launch selected app |
| `Escape` | Close launcher |

### Configuration

Pinned apps are configured in `src/app.rs`:

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

Window dimensions are in `src/ui/theme.rs`.

## Architecture

```
src/
├── main.rs           # Entry point, IPC server, single-instance mutex
├── app.rs            # Main app state, UI rendering, message handling
├── discovery/        # Start Menu app scanning and caching
├── search/           # Fuzzy search implementation
├── ipc.rs            # Named pipe server/client for IPC
├── platform/         # Windows-specific (wallpaper, icons, vibrancy)
└── ui/
    ├── mod.rs        # UI component definitions
    └── theme.rs      # Colors, dimensions, styling constants
```

### Key Design Decisions

- **Single Instance** - Named mutex ensures only one daemon runs
- **IPC Communication** - A second `al.exe` instance sends commands to the daemon over a named pipe, then exits
- **Daemon Pattern** - Launcher stays resident in memory for instant response
- **App Caching** - Scanned apps are cached to JSON for fast subsequent loads
- **Transparent Window** - Uses `window-vibrancy` for modern glass effect

## Development

```bash
# Debug build (with console)
cargo build

# Release build (GUI, no console)
cargo build --release

# Watch mode (requires cargo-watch)
cargo watch -x build
```

## Tech Stack

- **Language**: Rust
- **GUI Framework**: [Iced](https://iced.rs/) 0.13
- **Windows API**: [windows](https://docs.rs/windows/) crate
- **Fuzzy Search**: [fuzzy-matcher](https://docs.rs/fuzzy-matcher/)
- **Async Runtime**: Tokio
- **Serialization**: Serde/JSON

## License

MIT License - see LICENSE file for details.
