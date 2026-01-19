# Atuin Bar

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

The app uses a configuration file at `~/.config/atuin-bar/config.toml`. On first run, a default config file is created automatically.

```toml
# Global shortcut to toggle the window
# Examples: "CommandOrControl+Shift+Space", "Alt+Space", "Super+H"
shortcut = "CommandOrControl+Shift+Space"
```

**Note:** Changes require an app restart to take effect.

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
npm install
npm run tauri dev
```

**Release:**
```bash
npm run tauri build
```

The release build will create optimized binaries in `src-tauri/target/release/bundle/`.

### Platform Support

- **Primary:** macOS (10.13+)
- **Secondary:** Linux (optional, requires GTK3 development libraries)

### macOS Private API

The app uses `macos-private-api` feature for better overlay window behavior on macOS. This enables proper window level management for the overlay effect.

## Development

### Prerequisites

- Rust (via rustup)
- Node.js & npm
- On Linux: GTK3 development libraries (libgtk-3-dev, libwebkit2gtk-4.0-dev)

### Recommended IDE Setup

- [VS Code](https://code.visualstudio.com/)
- [Tauri VS Code Extension](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode)
- [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)

## License

MIT OR Apache-2.0
