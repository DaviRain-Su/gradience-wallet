/// Resolve a public RPC endpoint for a given chain identifier.
/// Supports both CAIP-2 chain IDs and short chain names.
pub fn resolve_rpc(chain_id: &str) -> &str {
    match chain_id {
        "eip155:8453" | "base" | "8453" => "https://mainnet.base.org",
        "eip155:1" | "eth" | "ethereum" | "1" => "https://eth.llamarpc.com",
        "eip155:196" | "xlayer" | "okx" | "196" => "https://rpc.xlayer.tech",
        "eip155:56" | "bsc" | "bnb" | "56" => "https://bsc-dataseed.binance.org",
        "solana:101" | "solana" | "sol" => "https://api.mainnet-beta.solana.com",
        "solana:103" => "https://api.devnet.solana.com",
        "stellar:pubnet" | "stellar" | "xlm" => "https://horizon.stellar.org",
        _ => "https://eth.llamarpc.com",
    }
}

/// Parse a user-friendly chain name into a CAIP-2 chain ID.
pub fn chain_id_from_name(chain: &str) -> String {
    let lower = chain.to_lowercase();
    match lower.as_str() {
        "base" | "8453" => "eip155:8453".into(),
        "eth" | "ethereum" | "1" => "eip155:1".into(),
        "xlayer" | "okx" | "196" => "eip155:196".into(),
        "bsc" | "bnb" | "56" => "eip155:56".into(),
        "solana" | "sol" => "solana:101".into(),
        "stellar" | "xlm" => "stellar:pubnet".into(),
        _ => format!("eip155:{}", lower),
    }
}

/// Returns true for EVM-compatible chains.
pub fn is_evm_chain(chain_id: &str) -> bool {
    chain_id.starts_with("eip155:") || matches!(chain_id, "base" | "eth" | "ethereum" | "xlayer" | "okx" | "bsc" | "bnb" | "1" | "56" | "8453" | "196")
}

/// Returns true for Solana.
pub fn is_solana(chain_id: &str) -> bool {
    chain_id.starts_with("solana:") || matches!(chain_id, "solana" | "sol")
}

/// Returns true for Stellar.
pub fn is_stellar(chain_id: &str) -> bool {
    chain_id.starts_with("stellar:") || matches!(chain_id, "stellar" | "xlm")
}

/// Resolve the chain numeric ID for EVM chains.
pub fn evm_chain_num(chain_id: &str) -> u64 {
    match chain_id {
        "eip155:8453" | "base" | "8453" => 8453,
        "eip155:1" | "eth" | "ethereum" | "1" => 1,
        "eip155:196" | "xlayer" | "okx" | "196" => 196,
        "eip155:56" | "bsc" | "bnb" | "56" => 56,
        _ => 1,
    }
}
