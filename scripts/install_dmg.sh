#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DMG_DIR="$ROOT_DIR/src-tauri/target/release/bundle/dmg"

if [[ $# -gt 0 ]]; then
  DMG_PATH="$1"
else
  DMG_PATH="$(ls -t "$DMG_DIR"/*.dmg 2>/dev/null | head -n 1 || true)"
fi

if [[ -z "${DMG_PATH:-}" || ! -f "$DMG_PATH" ]]; then
  echo "No DMG file found. Run: npm run tauri build"
  exit 1
fi

MOUNT_DIR="$(mktemp -d /tmp/pomodoro-pulse-mount.XXXXXX)"

cleanup() {
  hdiutil detach "$MOUNT_DIR" >/dev/null 2>&1 || true
  rmdir "$MOUNT_DIR" >/dev/null 2>&1 || true
}
trap cleanup EXIT

hdiutil attach "$DMG_PATH" -nobrowse -readonly -mountpoint "$MOUNT_DIR" >/dev/null

APP_SOURCE="$(find "$MOUNT_DIR" -maxdepth 1 -type d -name "*.app" | head -n 1 || true)"
if [[ -z "${APP_SOURCE:-}" ]]; then
  echo "No .app found in mounted DMG."
  exit 1
fi

TARGET_DIR="/Applications"
if [[ ! -w "$TARGET_DIR" ]]; then
  TARGET_DIR="$HOME/Applications"
  mkdir -p "$TARGET_DIR"
fi

TARGET_APP="$TARGET_DIR/$(basename "$APP_SOURCE")"
if [[ -d "$TARGET_APP" ]]; then
  rm -rf "$TARGET_APP"
fi

ditto "$APP_SOURCE" "$TARGET_APP"
echo "Installed: $TARGET_APP"
open "$TARGET_APP"
