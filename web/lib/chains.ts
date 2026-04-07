/**
 * Chain name registry for CAIP-2 chain IDs.
 * Covers major EVM chains and Solana networks.
 * Unknown chains fall back to a human-readable parsed name.
 */

export const CHAIN_NAMES: Record<string, string> = {
  // Ethereum L1
  "eip155:1": "Ethereum",
  "eip155:5": "Goerli",
  "eip155:11155111": "Sepolia",
  "eip155:17000": "Holesky",

  // L2s
  "eip155:8453": "Base",
  "eip155:84531": "Base Goerli",
  "eip155:84532": "Base Sepolia",
  "eip155:137": "Polygon",
  "eip155:80001": "Polygon Mumbai",
  "eip155:80002": "Polygon Amoy",
  "eip155:42161": "Arbitrum One",
  "eip155:421613": "Arbitrum Goerli",
  "eip155:421614": "Arbitrum Sepolia",
  "eip155:10": "Optimism",
  "eip155:420": "Optimism Goerli",
  "eip155:11155420": "Optimism Sepolia",
  "eip155:324": "zkSync Era",
  "eip155:300": "zkSync Sepolia",
  "eip155:1101": "Polygon zkEVM",
  "eip155:1442": "Polygon zkEVM Testnet",
  "eip155:59144": "Linea",
  "eip155:59140": "Linea Goerli",
  "eip155:5000": "Mantle",
  "eip155:5001": "Mantle Testnet",
  "eip155:534352": "Scroll",
  "eip155:534351": "Scroll Sepolia",
  "eip155:7777777": "Zora",
  "eip155:999999999": "Zora Sepolia",
  "eip155:169": "Manta Pacific",
  "eip155:3441005": "Manta Pacific Testnet",
  "eip155:81457": "Blast",
  "eip155:168587773": "Blast Sepolia",
  "eip155:252": "Fraxtal",
  "eip155:2522": "Fraxtal Testnet",
  "eip155:957": "Lyra Chain",
  "eip155:1750": "Metall2",

  // BNB / Avalanche / Fantom / etc
  "eip155:56": "BNB Chain",
  "eip155:97": "BNB Testnet",
  "eip155:43114": "Avalanche C-Chain",
  "eip155:43113": "Avalanche Fuji",
  "eip155:250": "Fantom",
  "eip155:4002": "Fantom Testnet",
  "eip155:1284": "Moonbeam",
  "eip155:1285": "Moonriver",
  "eip155:1287": "Moonbase Alpha",
  "eip155:2020": "Ronin",
  "eip155:2021": "Ronin Testnet",
  "eip155:25": "Cronos",
  "eip155:338": "Cronos Testnet",
  "eip155:42220": "Celo",
  "eip155:44787": "Celo Alfajores",
  "eip155:100": "Gnosis",
  "eip155:10200": "Chiado",
  "eip155:1313161554": "Aurora",
  "eip155:1313161555": "Aurora Testnet",
  "eip155:288": "Boba Network",
  "eip155:280": "Boba Goerli",

  // HashKey Chain
  "eip155:177": "HashKey Chain",
  "eip155:133": "HashKey Chain Testnet",

  // Solana
  "solana:4sGjMW1sUnHzSxGspuhpqLDx6miyAjh93kWrQX5L9rD": "Solana Mainnet",
  "solana:5eykt4UsFv8P8NJdTREpY1vzqKqZKvdp": "Solana Devnet",
  "solana:EtWTRABZaYq6iMfeYKouRu166VU2xqa1": "Solana Testnet",
  "solana:11111111111111111111111111111111": "Solana Localnet",
};

export function formatChainName(caip2ChainId: string): string {
  if (CHAIN_NAMES[caip2ChainId]) {
    return CHAIN_NAMES[caip2ChainId];
  }

  // Fallback for unknown EVM chains
  if (caip2ChainId.startsWith("eip155:")) {
    const chainNum = caip2ChainId.replace("eip155:", "");
    return `EVM Chain ${chainNum}`;
  }

  // Fallback for unknown Solana networks
  if (caip2ChainId.startsWith("solana:")) {
    const network = caip2ChainId.replace("solana:", "");
    return `Solana ${network.slice(0, 8)}...`;
  }

  // Generic fallback
  return caip2ChainId;
}
