const { PrivateKeyAccount } = require('js-conflux-sdk');

function main() {
  const seedHex = process.argv[2];
  const networkId = parseInt(process.argv[3] || '1', 10);
  if (!seedHex) {
    console.log(JSON.stringify({ success: false, error: 'missing seed hex' }));
    process.exit(1);
  }
  if (Number.isNaN(networkId)) {
    console.log(JSON.stringify({ success: false, error: 'invalid networkId' }));
    process.exit(1);
  }
  const seedBuf = Buffer.from(seedHex.replace(/^0x/, ''), 'hex');
  // Deterministic private key from first 32 bytes of seed (same pattern as TON/ETH in this project)
  const secret = seedBuf.slice(0, 32);
  const account = new PrivateKeyAccount('0x' + secret.toString('hex'), networkId);
  console.log(JSON.stringify({
    success: true,
    address: account.address,
    privateKey: account.privateKey,
    hexAddress: account.hexAddress,
  }));
}

main();
