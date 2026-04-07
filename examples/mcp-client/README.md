# MCP Client Example

A minimal Node.js client that speaks the Model Context Protocol (MCP) to the `gradience-mcp` server over stdio.

## Features
- Spawns `gradience-mcp` automatically
- Sends `initialize` handshake
- Discovers tools via `tools/list`
- Calls a tool (`get_balance`) via `tools/call`

## Run

```bash
cd examples/mcp-client
WALLET_ID=<your-wallet-id> node index.js
```

Or from repo root:

```bash
cd examples/mcp-client
WALLET_ID=your-wallet-id GRADIENCE_ROOT=../.. node index.js
```

You can also customize the MCP binary path:

```bash
MCP_BIN=/path/to/gradience-mcp WALLET_ID=your-wallet-id node index.js
```
