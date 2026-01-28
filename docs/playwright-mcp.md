# Playwright MCP Quickstart

This repo ships a helper script to run the official Playwright Model Context Protocol (MCP) server so agent clients can use browser automation as a tool.

## Prereqs
- Node.js 18+
- npm (bundled with Node)

## Start the MCP server

```bash
# From repo root
scripts/run-playwright-mcp.sh
```

What it does:
- Launches `@playwright/mcp` headless
- Restricts allowed hosts to `127.0.0.1,localhost` for safety
- Stores artifacts under `.mcp/playwright/`

You can pass flags through to the underlying server, for example to change the browser:

```bash
scripts/run-playwright-mcp.sh --browser firefox
```

## Using with agents
Point your MCP-capable client at the server endpoint the script prints (default: `ws://127.0.0.1:3333`). Consult your client’s docs for how to register an MCP server.

## Notes
- No repo code changes are required to run the server.
- The server is stateless; stop/start as needed.
