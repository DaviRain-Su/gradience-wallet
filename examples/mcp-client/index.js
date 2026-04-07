const { spawn } = require('child_process');

const MCP_BIN = process.env.MCP_BIN || 'cargo';
const MCP_ARGS = process.env.MCP_ARGS ? process.env.MCP_ARGS.split(' ') : ['run', '--bin', 'gradience-mcp'];

function send(stdin, msg) {
  const json = JSON.stringify(msg);
  console.log('\n>> SEND:', json);
  stdin.write(json + '\n');
}

function main() {
  const proc = spawn(MCP_BIN, MCP_ARGS, {
    cwd: process.env.GRADIENCE_ROOT || '../..',
    stdio: ['pipe', 'pipe', 'pipe']
  });

  proc.stdout.on('data', (data) => {
    const lines = data.toString().trim().split('\n');
    lines.forEach((line) => {
      if (!line.trim()) return;
      console.log('\n<< RECV:', line);
      try {
        const msg = JSON.parse(line);
        handleMessage(proc.stdin, msg);
      } catch (e) {
        console.log('   (non-json line)');
      }
    });
  });

  proc.stderr.on('data', (data) => {
    console.error('[mcp stderr]', data.toString().trim());
  });

  proc.on('close', (code) => {
    console.log(`\nMCP process exited with code ${code}`);
    process.exit(code);
  });

  // Step 1: initialize
  send(proc.stdin, {
    jsonrpc: '2.0',
    id: 1,
    method: 'initialize',
    params: { protocolVersion: '2024-11-05', capabilities: {}, clientInfo: { name: 'example-mcp-client', version: '1.0.0' } }
  });
}

let state = 'init';
let tools = [];

function handleMessage(stdin, msg) {
  if (msg.id === 1 && state === 'init') {
    console.log('\n✅ Initialized');
    state = 'tools_list';
    send(stdin, { jsonrpc: '2.0', id: 2, method: 'tools/list' });
    return;
  }

  if (msg.id === 2 && state === 'tools_list') {
    tools = (msg.result && msg.result.tools) || [];
    console.log(`\n✅ Discovered ${tools.length} tools:`);
    tools.forEach((t) => console.log(`   • ${t.name}: ${t.description}`));

    // Step 3: call get_balance (requires wallet_id)
    const walletId = process.env.WALLET_ID;
    if (walletId) {
      state = 'tools_call';
      send(stdin, {
        jsonrpc: '2.0',
        id: 3,
        method: 'tools/call',
        params: {
          name: 'get_balance',
          arguments: { walletId, chainId: 'eip155:8453' }
        }
      });
    } else {
      console.log('\nℹ️ Set WALLET_ID env var to demo a tool call.');
      stdin.end();
    }
    return;
  }

  if (msg.id === 3 && state === 'tools_call') {
    console.log('\n✅ Tool call result:');
    console.log(JSON.stringify(msg.result, null, 2));
    stdin.end();
    return;
  }

  if (msg.error) {
    console.error('\n❌ Error:', msg.error);
    stdin.end();
  }
}

main();
