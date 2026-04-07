import { privateKeyToAccount } from "viem/accounts";
import { createWalletClient, http, defineChain, type Chain } from "viem";
import { readFileSync } from "fs";

const RPC_URL = process.env.ANCHOR_RPC_URL || "https://hashkeychain-testnet.alt.technology";
const PRIVATE_KEY = process.env.ANCHOR_PRIVATE_KEY;

if (!PRIVATE_KEY) {
  console.error("Missing ANCHOR_PRIVATE_KEY");
  process.exit(1);
}

const hashKeyChainTestnet: Chain = defineChain({
  id: 133,
  name: "HashKey Chain Testnet",
  nativeCurrency: { name: "HashKey Token", symbol: "HSK", decimals: 18 },
  rpcUrls: {
    default: { http: [RPC_URL] },
    public: { http: [RPC_URL] },
  },
  blockExplorers: {
    default: {
      name: "HashKey Explorer",
      url: "https://hashkeychain-testnet-explorer.alt.technology",
    },
  },
});

const account = privateKeyToAccount(PRIVATE_KEY as `0x${string}`);

const client = createWalletClient({
  account,
  chain: hashKeyChainTestnet,
  transport: http(RPC_URL),
});

async function deploy() {
  const artifactPath = process.env.ANCHOR_ARTIFACT || "out/AuditAnchor.json";
  const { abi, bytecode } = loadArtifact(artifactPath);

  const hash = await client.deployContract({
    abi,
    bytecode: bytecode as `0x${string}`,
  });
  console.log("Deploy tx hash:", hash);
  console.log("Track it on: https://hashkeychain-testnet-explorer.alt.technology/tx/" + hash);
}

function loadArtifact(path: string) {
  try {
    const json = JSON.parse(readFileSync(path, "utf8"));
    const abi = json.abi || json;
    let bytecode = json.bytecode;
    if (typeof bytecode === "object") bytecode = bytecode.object;
    if (!bytecode) throw new Error("bytecode not found in artifact");
    if (!bytecode.startsWith("0x")) bytecode = "0x" + bytecode;
    return { abi, bytecode };
  } catch (e: any) {
    console.error("Failed to load artifact:", e.message);
    console.error("\nPlease compile AuditAnchor.sol first. You can:");
    console.error("  1. Use Remix IDE (https://remix.ethereum.org) to compile and download JSON artifact");
    console.error("  2. Or install Foundry and run: forge build");
    console.error("  3. Then set ANCHOR_ARTIFACT=path/to/AuditAnchor.json");
    process.exit(1);
  }
}

deploy().catch((e) => {
  console.error(e);
  process.exit(1);
});
