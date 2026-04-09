/// Resolve a public RPC endpoint for a given chain identifier.
/// Supports both CAIP-2 chain IDs and short chain names.
pub fn resolve_rpc(chain_id: &str) -> &str {
    match chain_id {
        "eip155:8453" | "base" | "8453" => "https://mainnet.base.org",
        "eip155:84532" | "base-sepolia" | "84532" => "https://sepolia.base.org",
        "eip155:1" | "eth" | "ethereum" | "1" => "https://eth.llamarpc.com",
        "eip155:196" | "xlayer" | "okx" | "196" => "https://rpc.xlayer.tech",
        "eip155:56" | "bsc" | "bnb" | "56" => "https://bsc-dataseed.binance.org",
        "eip155:97" | "bsc-testnet" | "97" => "https://data-seed-prebsc-1-s1.bnbchain.org:8545",
        "eip155:1030" | "conflux:mainnet" | "1030" => "https://evm.confluxrpc.com",
        "eip155:71" | "conflux" | "cfx" | "conflux:testnet" | "71" => {
            "https://evmtest.confluxrpc.com"
        }
        "eip155:42161" | "arbitrum" | "arb" | "42161" => "https://arb1.arbitrum.io/rpc",
        "eip155:421614" | "arbitrum-sepolia" | "421614" => "https://sepolia-rollup.arbitrum.io/rpc",
        "eip155:137" | "polygon" | "matic" | "137" => "https://polygon-rpc.com",
        "eip155:80002" | "polygon-amoy" | "80002" => "https://rpc-amoy.polygon.technology",
        "eip155:10" | "optimism" | "op" | "10" => "https://mainnet.optimism.io",
        "eip155:11155420" | "optimism-sepolia" | "11155420" => "https://sepolia.optimism.io",
        "solana:101" | "solana" | "sol" => "https://api.mainnet-beta.solana.com",
        "solana:103" => "https://api.devnet.solana.com",
        "cfx:1029" | "conflux-core:mainnet" => "https://main.confluxrpc.com",
        "cfx:1" | "conflux-core" | "cfx-core" | "conflux-core:testnet" => {
            "https://test.confluxrpc.com"
        }
        "ton" | "toncoin" | "ton:0" | "ton:-1" => "https://testnet.toncenter.com/api/v2",
        "ton:testnet" => "https://testnet.toncenter.com/api/v2",
        "ton:mainnet" => "https://toncenter.com/api/v2",
        "stellar:pubnet" | "stellar" | "xlm" => "https://horizon.stellar.org",
        _ => "https://eth.llamarpc.com",
    }
}

/// Parse a user-friendly chain name into a CAIP-2 chain ID.
pub fn chain_id_from_name(chain: &str) -> String {
    let lower = chain.to_lowercase();
    match lower.as_str() {
        "base" | "8453" => "eip155:8453".into(),
        "base-sepolia" | "84532" => "eip155:84532".into(),
        "eth" | "ethereum" | "1" => "eip155:1".into(),
        "xlayer" | "okx" | "196" => "eip155:196".into(),
        "bsc" | "bnb" | "56" => "eip155:56".into(),
        "bsc-testnet" | "97" => "eip155:97".into(),
        "conflux" | "cfx" | "71" => "eip155:71".into(),
        "conflux:mainnet" | "1030" => "eip155:1030".into(),
        "conflux-core" | "cfx-core" | "cfx-core:testnet" => "cfx:1".into(),
        "conflux-core:mainnet" | "cfx-core:mainnet" => "cfx:1029".into(),
        "arbitrum" | "arb" | "42161" => "eip155:42161".into(),
        "arbitrum-sepolia" | "421614" => "eip155:421614".into(),
        "polygon" | "matic" | "137" => "eip155:137".into(),
        "polygon-amoy" | "80002" => "eip155:80002".into(),
        "optimism" | "op" | "10" => "eip155:10".into(),
        "optimism-sepolia" | "11155420" => "eip155:11155420".into(),
        "solana" | "sol" => "solana:101".into(),
        "ton" | "toncoin" => "ton:0".into(),
        "stellar" | "xlm" => "stellar:pubnet".into(),
        _ => format!("eip155:{}", lower),
    }
}

/// Returns true for EVM-compatible chains.
pub fn is_evm_chain(chain_id: &str) -> bool {
    chain_id.starts_with("eip155:")
        || matches!(
            chain_id,
            "base"
                | "base-sepolia"
                | "eth"
                | "ethereum"
                | "xlayer"
                | "okx"
                | "bsc"
                | "bnb"
                | "bsc-testnet"
                | "conflux"
                | "cfx"
                | "arbitrum"
                | "arb"
                | "arbitrum-sepolia"
                | "polygon"
                | "matic"
                | "polygon-amoy"
                | "optimism"
                | "op"
                | "optimism-sepolia"
                | "1"
                | "56"
                | "97"
                | "71"
                | "8453"
                | "84532"
                | "1030"
                | "196"
                | "42161"
                | "421614"
                | "137"
                | "80002"
                | "10"
                | "11155420"
        )
}

/// Returns true for Solana.
pub fn is_solana(chain_id: &str) -> bool {
    chain_id.starts_with("solana:") || matches!(chain_id, "solana" | "sol")
}

/// Returns true for TON.
pub fn is_ton(chain_id: &str) -> bool {
    chain_id.starts_with("ton:") || matches!(chain_id, "ton" | "toncoin")
}

/// Returns true for Conflux Core Space.
pub fn is_conflux_core(chain_id: &str) -> bool {
    chain_id.starts_with("cfx:") || matches!(chain_id, "conflux-core" | "cfx-core")
}

/// Returns true for Stellar.
pub fn is_stellar(chain_id: &str) -> bool {
    chain_id.starts_with("stellar:") || matches!(chain_id, "stellar" | "xlm")
}

/// Resolve Conflux Core Space networkId from chain id.
pub fn conflux_core_network_id(chain_id: &str) -> u32 {
    match chain_id {
        "cfx:1029" | "conflux-core:mainnet" => 1029,
        "cfx:1" | "conflux-core" | "cfx-core" | "conflux-core:testnet" => 1,
        _ => 1,
    }
}

/// Resolve the chain numeric ID for EVM chains.
pub fn evm_chain_num(chain_id: &str) -> u64 {
    match chain_id {
        "eip155:8453" | "base" | "8453" => 8453,
        "eip155:84532" | "base-sepolia" | "84532" => 84532,
        "eip155:1" | "eth" | "ethereum" | "1" => 1,
        "eip155:196" | "xlayer" | "okx" | "196" => 196,
        "eip155:56" | "bsc" | "bnb" | "56" => 56,
        "eip155:97" | "bsc-testnet" | "97" => 97,
        "eip155:1030" | "conflux" | "cfx" | "1030" => 1030,
        "eip155:71" | "conflux:testnet" | "71" => 71,
        "eip155:42161" | "arbitrum" | "arb" | "42161" => 42161,
        "eip155:421614" | "arbitrum-sepolia" | "421614" => 421614,
        "eip155:137" | "polygon" | "matic" | "137" => 137,
        "eip155:80002" | "polygon-amoy" | "80002" => 80002,
        "eip155:10" | "optimism" | "op" | "10" => 10,
        "eip155:11155420" | "optimism-sepolia" | "11155420" => 11155420,
        _ => 1,
    }
}
