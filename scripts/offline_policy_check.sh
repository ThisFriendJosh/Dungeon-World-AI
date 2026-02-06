#!/usr/bin/env bash
set -euo pipefail

# Offline policy guard: fail on obvious cloud runtime dependencies.

TARGETS=(
  "apps/vtt-ui"
  "services/ai"
  "apps/desktop/src-tauri/src"
)

PATTERNS=(
  "from ['\"]openai['\"]"
  "from ['\"]anthropic['\"]"
  "@google/generative-ai"
  "cohere"
  "api.openai.com"
  "anthropic.com"
  "generativelanguage.googleapis.com"
  "https://"
  "http://"
)

EXIT_CODE=0
for t in "${TARGETS[@]}"; do
  if [[ ! -e "$t" ]]; then
    continue
  fi
  for p in "${PATTERNS[@]}"; do
    if rg -n "$p" "$t" >/tmp/offline_hits.txt 2>/dev/null; then
      # allow local dev url references in tauri config docs/scripts by restricting targets above
      echo "[offline-policy] forbidden pattern '$p' in $t"
      cat /tmp/offline_hits.txt
      EXIT_CODE=1
    fi
  done
done

if [[ "$EXIT_CODE" -ne 0 ]]; then
  echo "Offline policy check FAILED"
  exit 1
fi

echo "Offline policy check PASSED"
