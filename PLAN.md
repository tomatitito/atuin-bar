# Atuin Bar - Implementation Plan

## Overview
Build a Spotlight-like search interface for atuin that responds to a global keyboard shortcut, providing quick access to shell history search.

## Goals
- **Primary**: macOS support with Spotlight-style overlay UI
- **Secondary**: Linux support (if not overly complex)
- **Lightweight**: Fast startup, minimal resource usage
- **Configurable**: Customizable keyboard shortcuts
- **Intuitive**: Familiar Spotlight-like UX

## Technology Stack
- **Framework**: Tauri (Rust backend + web frontend)
- **UI**: HTML/CSS/JavaScript (or React/Vue if needed for complexity)
- **Integration**: atuin CLI via command execution
- **Hotkeys**: Tauri global-shortcut plugin
- **CI/CD**: GitHub Actions for testing and releases

## CI/CD & Release Strategy

### Testing
- GitHub Actions workflow to run tests on every push/PR
- Run on multiple platforms (macOS, optionally Linux)
- Execute Rust tests (`cargo test`)
- Execute frontend tests (if applicable)
- Lint checks (clippy, formatting)

### Releases
- **Trigger**: Push git tag with format `vx.y.z` (e.g., `v1.0.0`)
- **Artifacts**: Generate downloadable executables for:
  - macOS: `.zip` containing `.app` bundle (e.g., `atuin-bar-v1.0.0-macos.zip`)
  - Linux (optional): `.AppImage`, `.deb`, or `.tar.gz`
- **Process**:
  - GitHub Actions builds binaries for each platform
  - Creates GitHub Release with tag
  - Attaches compiled artifacts to release
  - Auto-generates release notes from commits
- **Versioning**: Use semantic versioning (major.minor.patch)
- **Installation**:
  - macOS: Download zip, extract, drag `.app` to Applications folder
  - Note: First run requires right-click → Open (unsigned app)
- **Future**: Homebrew Cask integration for easier installation (`brew install --cask atuin-bar`)

## Architecture

### Components

#### 1. Tauri Backend (Rust)
- Global hotkey registration and handling
- Execute atuin search commands
- Parse atuin output
- Clipboard operations
- Window management (show/hide overlay)
- Configuration management

#### 2. Frontend (Web UI)
- Search input field
- Results list display
- Keyboard navigation (up/down arrows, enter, escape)
- Result formatting (command, exit code, directory, timestamp)
- Spotlight-like styling and animations

#### 3. Atuin Integration
- Execute `atuin search` with custom format
- Real-time search as user types
- Parse formatted output into structured data

## Features

### Phase 1: Core Functionality
1. **Global Hotkey**
   - Register configurable global shortcut (default: Ctrl+R)
   - Show/hide overlay window
   - Focus search input when shown

2. **Search Interface**
   - Center-screen overlay window (Spotlight-like)
   - Search input field
   - Real-time search using `atuin search <query> --format "{command}|{exit}|{directory}|{time}"`
   - Display results in a list

3. **Result Selection**
   - Keyboard navigation (arrow keys)
   - Enter to select
   - Escape to close
   - Copy selected command to clipboard

4. **Basic Configuration**
   - Config file for keyboard shortcut
   - Maybe: theme/appearance settings

### Phase 2: Enhanced Features (Optional)
1. **Terminal Integration**
   - Attempt to insert command into active terminal (if feasible)
   - Fallback to clipboard if terminal insertion fails

2. **Advanced Search**
   - Support atuin filters (directory, exit code, time range)
   - Search syntax hints

3. **Result Details**
   - Show full command details on hover/selection
   - Display exit code with visual indicator (green/red)
   - Show directory context

4. **Performance**
   - Debounce search input
   - Limit results for performance
   - Cache recent searches

## Implementation Steps

### 1. Project Setup
- [ ] Initialize Tauri project
- [ ] Set up basic window configuration (overlay mode)
- [ ] Configure build system
- [ ] Set up GitHub Actions for CI (tests, linting)
- [ ] Set up GitHub Actions for releases (triggered by vx.y.z tags)

### 2. UI Development
- [ ] Create Spotlight-like overlay window
- [ ] Build search input component
- [ ] Create results list component
- [ ] Implement keyboard navigation
- [ ] Add styling (Spotlight-inspired)

### 3. Backend Integration
- [ ] Add global-shortcut Tauri plugin
- [ ] Implement hotkey registration
- [ ] Implement window show/hide logic
- [ ] Create atuin CLI execution function
- [ ] Parse atuin output format
- [ ] Implement clipboard copy functionality

### 4. Atuin Integration
- [ ] Test atuin search command with custom format
- [ ] Implement real-time search (debounced)
- [ ] Parse and structure results
- [ ] Handle edge cases (no results, errors)

### 5. Configuration
- [ ] Create config file structure
- [ ] Implement config loading
- [ ] Add hotkey customization
- [ ] (Optional) Add appearance settings

### 6. Testing & Refinement
- [ ] Test on macOS
- [ ] Test keyboard navigation
- [ ] Test global hotkey registration
- [ ] Handle edge cases
- [ ] (Optional) Test on Linux

### 7. Documentation
- [ ] README with installation instructions
- [ ] Usage guide
- [ ] Configuration documentation

### 8. Future Enhancements
- [ ] Homebrew Cask formula for easier installation
- [ ] Code signing and notarization (requires Apple Developer account)

## Technical Considerations

### Atuin CLI Integration
- Use `atuin search <query> --format "{command}|{exit}|{directory}|{time}"`
- Parse pipe-delimited output
- Handle special characters in commands
- Consider using `--limit` for performance

### Window Behavior
- Always-on-top overlay window
- Center screen positioning
- Hide on focus loss (click outside)
- Transparent/blurred background

### Keyboard Shortcut
- Ctrl+R is commonly used in terminals (reverse search)
- On macOS, Cmd+Space is Spotlight (avoid conflict)
- Make it configurable via config file
- Handle shortcut conflicts gracefully

### Cross-Platform Challenges
- Global hotkeys work differently on macOS vs Linux
- Terminal integration is platform-specific (may need to punt on this)
- Window overlays have different APIs
- Clipboard access should be straightforward with Tauri

### Performance
- Tauri apps start quickly (~100-200ms)
- Debounce search input (200-300ms)
- Limit results (maybe 20-50 items)
- Consider caching atuin results briefly

### CI/CD Setup
- Use Tauri's official GitHub Actions for building releases
- Configure matrix builds for multiple platforms
- Set up caching for Rust dependencies and Node modules
- Use `tauri-action` for automated building and releasing
- Tag-based releases: push `vx.y.z` tag to trigger release workflow
- Test workflow runs on every push/PR for quality assurance

## Open Questions
1. Should we support multiple selection (copy multiple commands)?
2. How to handle very long commands in the UI?
3. Should we show command previews/syntax highlighting?
4. How to handle atuin not being installed/configured?
5. Should we bundle configuration UI or use config file only?

## Success Criteria
- Global hotkey shows overlay instantly
- Search results appear within 300ms of typing
- Keyboard navigation is smooth and intuitive
- Selected command copies to clipboard reliably
- Works on macOS (primary target)
- Configurable keyboard shortcut
