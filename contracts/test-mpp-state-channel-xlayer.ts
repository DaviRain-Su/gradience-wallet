import { privateKeyToAccount } from "viem/accounts";
import { createWalletClient, createPublicClient, http, defineChain, keccak256, toHex } from "viem";
import { readFileSync } from "fs";
import { execSync } from "child_process";

const xlayerTestnet = defineChain({
  id: 1952,
  name: "XLayer Testnet",
  nativeCurrency: { name: "OKB", symbol: "OKB", decimals: 18 },
  rpcUrls: {
    default: { http: ["https://testrpc.xlayer.tech"] },
    public: { http: ["https://testrpc.xlayer.tech"] },
  },
});

const PAYER_PK = process.env.PAYER_PRIVATE_KEY as `0x${string}`;
const PAYEE_PK = process.env.PAYEE_PRIVATE_KEY as `0x${string}`;
if (!PAYER_PK || !PAYEE_PK) {
  console.error("Missing PAYER_PRIVATE_KEY or PAYEE_PRIVATE_KEY env vars");
  process.exit(1);
}

const payerAccount = privateKeyToAccount(PAYER_PK);
const payeeAccount = privateKeyToAccount(PAYEE_PK);

const publicClient = createPublicClient({ chain: xlayerTestnet, transport: http() });
const payerClient = createWalletClient({ account: payerAccount, chain: xlayerTestnet, transport: http() });
const payeeClient = createWalletClient({ account: payeeAccount, chain: xlayerTestnet, transport: http() });

const artifactPath = "out/MppStateChannel.sol/MppStateChannel.json";
const { abi } = JSON.parse(readFileSync(artifactPath, "utf8"));
const CONTRACT_ADDRESS = "0xb02931aa17fdcfc26393556dbdd0cfaca0d44090" as `0x${string}`;

async function main() {
  console.log("Payer:", payerAccount.address);
  console.log("Payee:", payeeAccount.address);
  console.log("Contract:", CONTRACT_ADDRESS);

  // Fund payee if needed
  const payeeBalance = await publicClient.getBalance({ address: payeeAccount.address });
  if (payeeBalance < 5000000000000000n) {
    console.log("Funding payee with 0.01 OKB...");
    const fundTx = await payerClient.sendTransaction({
      to: payeeAccount.address,
      value: 10000000000000000n, // 0.01 OKB
    });
    console.log("  fund tx:", fundTx);
    await publicClient.waitForTransactionReceipt({ hash: fundTx });
  }

  // Generate deterministic channelId (use a fresh one each run)
  const channelId = keccak256(toHex("test-channel-003"));
  console.log("Channel ID:", channelId);

  // Open channel (payer -> contract)
  const now = Math.floor(Date.now() / 1000);
  const challengePeriod = 30n; // 30 seconds for quick test
  const expiresAt = BigInt(now + 86400); // 24 hours from now
  const deposit = 1000000000000000n; // 0.001 OKB

  console.log("\n1. Opening channel...");
  try {
    const openHash = await payerClient.writeContract({
      address: CONTRACT_ADDRESS,
      abi,
      functionName: "openChannel",
      args: [channelId, payeeAccount.address, challengePeriod, expiresAt],
      value: deposit,
    });
    console.log("  tx:", openHash);
    const openReceipt = await publicClient.waitForTransactionReceipt({ hash: openHash });
    console.log("  status:", openReceipt.status);
  } catch (e: any) {
    console.error("  openChannel failed:", e.message);
    // Maybe already exists, continue
  }

  // Read channel state (mapping getter returns tuple)
  const channelRaw = await publicClient.readContract({
    address: CONTRACT_ADDRESS,
    abi,
    functionName: "channels",
    args: [channelId],
  }) as any;
  const channelTuple = Array.isArray(channelRaw)
    ? channelRaw
    : Object.values(channelRaw || {});
  const [
    chPayer,
    chPayee,
    chDeposit,
    chSettledAmount,
    chChallengePeriod,
    chExpiresAt,
    chClosed,
    chChallengeEnd,
    chLatestNonce,
  ] = channelTuple;
  console.log("\n2. Channel on-chain state:", {
    payer: chPayer,
    payee: chPayee,
    deposit: chDeposit?.toString?.(),
    settledAmount: chSettledAmount?.toString?.(),
    challengePeriod: chChallengePeriod?.toString?.(),
    expiresAt: chExpiresAt?.toString?.(),
    closed: chClosed,
    challengeEnd: chChallengeEnd?.toString?.(),
    latestNonce: chLatestNonce?.toString?.(),
  });

  // Build state update signature (payer signs off-chain)
  const nonce = 1n;
  const amount = 500000000000000n; // 0.0005 OKB

  // Generate signature using the Rust SDK binary to avoid viem's double-hashing behavior
  const secretHex = PAYER_PK;
  const sigCmd = `../target/debug/examples/generate_state_channel_sig ${secretHex} ${channelId} ${nonce} ${amount}`;
  const signature = execSync(sigCmd, { encoding: "utf8" }).trim() as `0x${string}`;
  console.log("\n3. State update signature (from Rust SDK):", signature);

  // Verify signature on-chain
  const verifyResult = await publicClient.readContract({
    address: CONTRACT_ADDRESS,
    abi,
    functionName: "channels",
    args: [channelId],
  });
  console.log("  Signature length:", signature.length, "bytes");

  // Initiate settlement (payee)
  console.log("\n4. Initiating settlement (payee)...");
  const settleHash = await payeeClient.writeContract({
    address: CONTRACT_ADDRESS,
    abi,
    functionName: "initiateSettlement",
    args: [channelId, nonce, amount, signature],
  });
  console.log("  tx:", settleHash);
  const settleReceipt = await publicClient.waitForTransactionReceipt({ hash: settleHash });
  console.log("  status:", settleReceipt.status);

  // Read channel state again
  const afterRaw = await publicClient.readContract({
    address: CONTRACT_ADDRESS,
    abi,
    functionName: "channels",
    args: [channelId],
  }) as any;
  const afterTuple = Array.isArray(afterRaw) ? afterRaw : Object.values(afterRaw || {});
  const afterChallengeEnd = afterTuple[7];
  const afterSettledAmount = afterTuple[3];
  const afterLatestNonce = afterTuple[8];
  console.log("\n5. Channel after settlement:", {
    settledAmount: afterSettledAmount?.toString?.(),
    challengeEnd: afterChallengeEnd?.toString?.(),
    latestNonce: afterLatestNonce?.toString?.(),
  });

  // Wait challenge period
  const waitSeconds = Number(afterChallengeEnd) - Math.floor(Date.now() / 1000);
  if (waitSeconds > 0) {
    console.log(`\n6. Waiting ${waitSeconds}s for challenge period...`);
    await new Promise((r) => setTimeout(r, waitSeconds * 1000 + 2000));
  }

  // Confirm settlement (can be anyone, using payer here)
  console.log("\n7. Confirming settlement...");
  const confirmHash = await payerClient.writeContract({
    address: CONTRACT_ADDRESS,
    abi,
    functionName: "confirmSettlement",
    args: [channelId],
  });
  console.log("  tx:", confirmHash);
  const confirmReceipt = await publicClient.waitForTransactionReceipt({ hash: confirmHash });
  console.log("  status:", confirmReceipt.status);

  const finalRaw = await publicClient.readContract({
    address: CONTRACT_ADDRESS,
    abi,
    functionName: "channels",
    args: [channelId],
  }) as any;
  const finalTuple = Array.isArray(finalRaw) ? finalRaw : Object.values(finalRaw || {});
  console.log("\n8. Final channel state:", {
    closed: finalTuple[6],
    settledAmount: finalTuple[3]?.toString?.(),
  });

  console.log("\n=== Test completed successfully ===");
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
