# Fix codemogger MCP stdio pollution

**Status**: cancelled on 2026-04-15

Historical note:
Relationship: Preserved as a historical planning record; future work should start from a fresh plan instead of reviving this stale in-progress file.

## Goal

Prevent `codemogger mcp` from emitting shell banner text on stdout before the MCP protocol starts.

## Scope

- update the local `codemogger` devenv shell hook to avoid printing banners during non-interactive command runs
- make the configured MCP launcher use quiet `devenv` mode for extra protection
- verify that MCP startup no longer emits shell bootstrap text before any codemogger output

## Result

- pending
