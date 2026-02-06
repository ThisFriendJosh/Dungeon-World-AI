#!/usr/bin/env bash
set -euo pipefail

# Usage:
#   scripts/import_gamemasterai.sh /path/to/local/gamemasterai-clone
# or
#   scripts/import_gamemasterai.sh https://github.com/ThisFriendJosh/gamemasterai.git

SRC="${1:-}"
if [[ -z "$SRC" ]]; then
  echo "Usage: $0 <local-path-or-git-url-to-gamemasterai>"
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
  GM_SRC="$SRC"
else
  TMP_DIR="$(mktemp -d)"
  git clone --depth=1 "$SRC" "$TMP_DIR/gamemasterai"
  GM_SRC="$TMP_DIR/gamemasterai"
fi

mkdir -p services/ai/vendor/gamemasterai
rsync -a --delete \
  --exclude '.git' \
  --exclude 'node_modules' \
  "$GM_SRC/" services/ai/vendor/gamemasterai/

cat > services/ai/ATTRIBUTION.md <<'ATTR'
# Attribution

This directory may include code and patterns imported from:
- https://github.com/ThisFriendJosh/gamemasterai (MIT)

Imported code in `services/ai/vendor/gamemasterai` should remain clearly separated from adapted runtime code in `services/ai/src`.
ATTR

echo "Imported GameMasterAI snapshot into services/ai/vendor/gamemasterai"
echo "Next: adapt only through local interfaces in services/ai/src and run npm run policy:offline"
