const { Conflux, Drip } = require('js-conflux-sdk');

async function main() {
  const args = {};
  for (let i = 2; i < process.argv.length; i += 2) {
    const key = process.argv[i].replace(/^--/, '');
    args[key] = process.argv[i + 1];
  }

  const required = ['rpc', 'privateKey', 'to', 'networkId'];
  for (const r of required) {
    if (args[r] == null) {
      console.log(JSON.stringify({ success: false, error: `missing ${r}` }));
      process.exit(1);
    }
  }
  if (args.amount == null && args.value == null) {
    console.log(JSON.stringify({ success: false, error: 'missing amount or value' }));
    process.exit(1);
  }

  try {
    const conflux = new Conflux({
      url: args.rpc,
      networkId: parseInt(args.networkId, 10),
    });
    const account = conflux.wallet.addPrivateKey(args.privateKey);
    let value;
    if (args.value) {
      value = args.value.startsWith('0x') ? BigInt(args.value) : Drip.fromCFX(parseFloat(args.value));
    } else {
      value = Drip.fromCFX(parseFloat(args.amount));
    }
    const txHash = await conflux.cfx.sendTransaction({
      from: account.address,
      to: args.to,
      value,
    });
    // sendTransaction returns the transaction hash string directly
    console.log(JSON.stringify({ success: true, txHash }));
  } catch (err) {
    console.log(JSON.stringify({ success: false, error: err.message || String(err) }));
    process.exit(1);
  }
}

main();
