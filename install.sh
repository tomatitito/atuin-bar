#!/bin/bash
set -e

APP_NAME="atuin-bar.app"
BUNDLE_PATH="src-tauri/target/release/bundle/macos/$APP_NAME"
INSTALL_PATH="/Applications/$APP_NAME"

if [ ! -d "$BUNDLE_PATH" ]; then
  echo "Build not found at $BUNDLE_PATH"
  echo "Run 'npm run tauri build' first."
  exit 1
fi

rm -rf "$INSTALL_PATH"
cp -R "$BUNDLE_PATH" "$INSTALL_PATH"
echo "Installed $APP_NAME to /Applications"
