#!/usr/bin/env bash
set -euo pipefail

# Run the official Playwright MCP server with repo-friendly defaults.
# - Headless to avoid opening a browser window
# - Restrict allowed hosts to localhost by default
# - Store artifacts under .mcp/playwright/

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
ARTIFACT_DIR="$ROOT_DIR/.mcp/playwright"
mkdir -p "$ARTIFACT_DIR"

exec npx @playwright/mcp@latest \
  --headless \
  --allowed-hosts 127.0.0.1,localhost \
  --output-dir "$ARTIFACT_DIR" \
  "$@"
