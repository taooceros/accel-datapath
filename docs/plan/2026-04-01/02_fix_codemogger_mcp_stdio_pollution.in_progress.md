# Fix codemogger MCP stdio pollution

## Goal

Prevent `codemogger mcp` from emitting shell banner text on stdout before the MCP protocol starts.

## Scope

- update the local `codemogger` devenv shell hook to avoid printing banners during non-interactive command runs
- make the configured MCP launcher use quiet `devenv` mode for extra protection
- verify that MCP startup no longer emits shell bootstrap text before any codemogger output

## Result

- pending
