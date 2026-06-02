# Atuin Bar

<p align="center">
  <img src="logo.png" alt="Atuin Bar" width="200" />
</p>


Spotlight-like overlay interface for [atuin](https://github.com/atuinsh/atuin) shell history search.

## Overview

Atuin Bar provides a macOS Spotlight-style keyboard shortcut and overlay window for quickly searching your shell history using atuin. Press a global hotkey, type to search, and select commands to copy to clipboard.

## Features

- Global keyboard shortcut to show/hide search overlay (configurable)
- Spotlight-like UI with center-screen overlay
- Real-time search through atuin history
- Keyboard navigation (arrow keys, Enter, Escape)
- Automatic clipboard copy on selection

## Configuration

The app can be configured in two ways:

### 1. Settings Menu (Recommended)

Click on the **Atuin-Bar** menu and select **Settings** to open a graphical configuration window. This allows you to:
- Change the global keyboard shortcut (requires restart)
- Toggle between dark and light themes
- Adjust the maximum number of search results displayed
- Customize the window width
- Check GitHub for updates and install a newer release

Changes to theme, max results, and window width take effect immediately. Shortcut changes require an app restart.

### Self-Update

To update atuin-bar from the app:

1. Open the **Atuin-Bar** menu and select **Settings**.
2. In the **Updates** section, click **Check for Updates**.
3. If a newer GitHub Release is available, atuin-bar downloads the macOS release archive, installs it over the current `.app` bundle, quits, and reopens automatically.

If you are already running the latest version, the settings window shows an “already up to date” message.

**Note:** Self-update currently works only when running the packaged macOS `.app` bundle from a GitHub Release. It is not available in development mode or when running the raw binary directly.

### 2. Configuration File

The app uses a configuration file at `~/.config/atuin-bar/config.toml`. On first run, a default config file is created automatically. You can also edit this file directly:

```toml
# Global shortcut to toggle the window
# Examples: "CommandOrControl+Shift+Space", "Alt+Space", "Super+H"
shortcut = "CommandOrControl+Shift+Space"

# Theme: "dark" or "light" (default: "dark")
theme = "dark"

# Maximum number of results to display (default: 20)
max_results = 20

# Window width in pixels (default: 700)
window_width = 700
```

**Note:** The Settings menu and config file are synchronized - changes made in either location will be reflected in both.

## Build Configuration

### Dependencies

**Rust:**
- Tauri 2.x
- serde & serde_json for serialization
- tauri-plugin-global-shortcut for hotkey support
- tauri-plugin-opener for system integration

**Frontend:**
- TypeScript
- Vite for bundling
- Tauri API libraries

### Build Profiles

The release profile is optimized for small binary size:
- `opt-level = "z"` - Maximum size optimization
- `lto = true` - Link-time optimization enabled
- `codegen-units = 1` - Single codegen unit for better optimization
- `panic = "abort"` - Smaller panic handler
- `strip = true` - Strip debug symbols

### Building

**Development:**
```bash
pnpm install
pnpm tauri dev
```

**Release:**
```bash
pnpm tauri build
```

The release build will create optimized binaries in `src-tauri/target/release/bundle/`.

### GitHub Releases

Use the version script to keep `package.json`, `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock`, and `src-tauri/tauri.conf.json` in sync:

```bash
pnpm set-version patch   # or minor, major, 1.2.3, v1.2.3
git add package.json src-tauri/Cargo.toml src-tauri/Cargo.lock src-tauri/tauri.conf.json
git commit -m "Bump version to 1.2.3"
git push origin main
```

When CI sees a new version on `main`, it creates the matching `v1.2.3` tag, builds a universal macOS `.app` bundle, packages it as `atuin-bar-v1.2.3-macos-universal.zip`, and creates a GitHub Release with auto-generated release notes. Pushing a `vX.Y.Z` tag manually also triggers the release workflow.

### Platform Support

- **Primary:** macOS (10.13+)
- **Secondary:** Linux (optional, requires GTK3 development libraries)

### macOS Private API

The app uses `macos-private-api` feature for better overlay window behavior on macOS. This enables proper window level management for the overlay effect.

## Development

### Prerequisites

- Rust (via rustup)
- Node.js & pnpm
- On Linux: GTK3 development libraries (libgtk-3-dev, libwebkit2gtk-4.0-dev)

### Recommended IDE Setup

- [VS Code](https://code.visualstudio.com/)
- [Tauri VS Code Extension](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode)
- [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)

## License

MIT OR Apache-2.0
