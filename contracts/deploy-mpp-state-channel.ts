import { privateKeyToAccount } from "viem/accounts";
import { createWalletClient, http, defineChain, type Chain } from "viem";
import { readFileSync } from "fs";

const PRIVATE_KEY = process.env.ANCHOR_PRIVATE_KEY;
if (!PRIVATE_KEY) {
  console.error("Missing ANCHOR_PRIVATE_KEY");
  process.exit(1);
}

const chains: Record<string, Chain> = {
  "bsc-testnet": defineChain({
    id: 97,
    name: "BSC Testnet",
    nativeCurrency: { name: "BNB", symbol: "tBNB", decimals: 18 },
    rpcUrls: {
      default: { http: ["https://data-seed-prebsc-1-s1.bnbchain.org:8545"] },
      public: { http: ["https://data-seed-prebsc-1-s1.bnbchain.org:8545"] },
    },
  }),
  "conflux-espace-testnet": defineChain({
    id: 71,
    name: "Conflux eSpace Testnet",
    nativeCurrency: { name: "CFX", symbol: "CFX", decimals: 18 },
    rpcUrls: {
      default: { http: ["https://evmtestnet.confluxrpc.com"] },
      public: { http: ["https://evmtestnet.confluxrpc.com"] },
    },
  }),
  "xlayer-testnet": defineChain({
    id: 1952,
    name: "XLayer Testnet",
    nativeCurrency: { name: "OKB", symbol: "OKB", decimals: 18 },
    rpcUrls: {
      default: { http: ["https://testrpc.xlayer.tech"] },
      public: { http: ["https://testrpc.xlayer.tech"] },
    },
  }),
  "base-sepolia": defineChain({
    id: 84532,
    name: "Base Sepolia",
    nativeCurrency: { name: "ETH", symbol: "ETH", decimals: 18 },
    rpcUrls: {
      default: { http: ["https://sepolia.base.org"] },
      public: { http: ["https://sepolia.base.org"] },
    },
  }),
  "arbitrum-sepolia": defineChain({
    id: 421614,
    name: "Arbitrum Sepolia",
    nativeCurrency: { name: "ETH", symbol: "ETH", decimals: 18 },
    rpcUrls: {
      default: { http: ["https://sepolia-rollup.arbitrum.io/rpc"] },
      public: { http: ["https://sepolia-rollup.arbitrum.io/rpc"] },
    },
  }),
  "polygon-amoy": defineChain({
    id: 80002,
    name: "Polygon Amoy",
    nativeCurrency: { name: "MATIC", symbol: "MATIC", decimals: 18 },
    rpcUrls: {
      default: { http: ["https://rpc-amoy.polygon.technology"] },
      public: { http: ["https://rpc-amoy.polygon.technology"] },
    },
  }),
};

function detectArtifact(): string {
  const candidates = [
    "out/MppStateChannel.sol/MppStateChannel.json",
    "out/MppStateChannel.json",
  ];
  for (const p of candidates) {
    try {
      readFileSync(p, "utf8");
      return p;
    } catch {}
  }
  throw new Error("MppStateChannel artifact not found. Searched: " + candidates.join(", "));
}

function loadArtifact(path: string) {
  const json = JSON.parse(readFileSync(path, "utf8"));
  const abi = json.abi || json;
  let bytecode = json.bytecode;
  if (typeof bytecode === "object") bytecode = bytecode.object;
  if (!bytecode) throw new Error("bytecode not found");
  if (!bytecode.startsWith("0x")) bytecode = "0x" + bytecode;
  return { abi, bytecode };
}

async function deployToChain(chainName: string, chain: Chain) {
  const account = privateKeyToAccount(PRIVATE_KEY as `0x${string}`);
  const client = createWalletClient({
    account,
    chain,
    transport: http(),
  });

  const artifactPath = process.env.STATE_CHANNEL_ARTIFACT || detectArtifact();
  const { abi, bytecode } = loadArtifact(artifactPath);

  console.log(`\nDeploying MppStateChannel to ${chainName} (chainId=${chain.id})...`);
  try {
    const hash = await client.deployContract({
      abi,
      bytecode: bytecode as `0x${string}`,
    });
    console.log(`  tx: ${hash}`);
    return { chain: chainName, chainId: chain.id, tx: hash };
  } catch (e: any) {
    console.error(`  FAILED: ${e.message}`);
    return { chain: chainName, chainId: chain.id, error: e.message };
  }
}

async function main() {
  const target = process.argv[2];
  const results: any[] = [];

  if (target && chains[target]) {
    results.push(await deployToChain(target, chains[target]));
  } else if (target === "all") {
    for (const [name, chain] of Object.entries(chains)) {
      results.push(await deployToChain(name, chain));
    }
  } else {
    console.log("Usage: bun run deploy-mpp-state-channel.ts <chain|all>");
    console.log("Available chains:", Object.keys(chains).join(", "));
    process.exit(1);
  }

  console.log("\n=== Deployment Summary ===");
  for (const r of results) {
    if (r.error) {
      console.log(`  ${r.chain} (${r.chainId}): FAILED - ${r.error}`);
    } else {
      console.log(`  ${r.chain} (${r.chainId}): ${r.tx}`);
    }
  }
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
