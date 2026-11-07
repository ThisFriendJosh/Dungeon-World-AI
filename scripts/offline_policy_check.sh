#!/usr/bin/env bash
set -euo pipefail
# Compatibility entry point; the guard itself runs on Windows and Linux with Node.
node "$(dirname "$0")/offline_policy_check.mjs"
