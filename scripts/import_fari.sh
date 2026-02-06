#!/usr/bin/env bash
set -euo pipefail

# Usage:
#   scripts/import_fari.sh /path/to/local/fari-clone
# or
#   scripts/import_fari.sh https://github.com/ThisFriendJosh/fari-app.git

SRC="${1:-}"
if [[ -z "$SRC" ]]; then
  echo "Usage: $0 <local-path-or-git-url-to-fari>"
  exit 1
fi

TMP_DIR=""
cleanup() {
  if [[ -n "$TMP_DIR" && -d "$TMP_DIR" ]]; then
    rm -rf "$TMP_DIR"
  fi
}
trap cleanup EXIT

if [[ -d "$SRC" ]]; then
  FARI_SRC="$SRC"
else
  TMP_DIR="$(mktemp -d)"
  git clone --depth=1 "$SRC" "$TMP_DIR/fari-app"
  FARI_SRC="$TMP_DIR/fari-app"
fi

if [[ ! -f "$FARI_SRC/package.json" ]]; then
  echo "error: source does not look like a JS app (missing package.json): $FARI_SRC"
  exit 1
fi

mkdir -p apps
if [[ -d apps/vtt-ui ]]; then
  rm -rf apps/vtt-ui.pre-fari
  mv apps/vtt-ui apps/vtt-ui.pre-fari
fi
mkdir -p apps/vtt-ui

rsync -a --delete \
  --exclude '.git' \
  --exclude 'node_modules' \
  "$FARI_SRC/" apps/vtt-ui/

# Force workspace-compatible package identity + scripts to preserve Tauri wiring.
node <<'NODE'
const fs = require('fs');
const path = 'apps/vtt-ui/package.json';
const pkg = JSON.parse(fs.readFileSync(path, 'utf8'));
pkg.name = '@dwa/vtt-ui';
pkg.private = true;
pkg.scripts = pkg.scripts || {};
if (!pkg.scripts.dev) pkg.scripts.dev = 'vite --port 5173';
if (!pkg.scripts.build) pkg.scripts.build = 'vite build';
if (!pkg.scripts.typecheck) pkg.scripts.typecheck = 'tsc --noEmit';
fs.writeFileSync(path, JSON.stringify(pkg, null, 2) + '\n');
NODE

echo "Imported Fari into apps/vtt-ui."
echo "Previous UI backup (if existed): apps/vtt-ui.pre-fari"
echo "Next: run npm run policy:offline and npm run check"
