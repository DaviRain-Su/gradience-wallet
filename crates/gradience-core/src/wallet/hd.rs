/// Standard HD derivation path helpers for multi-chain wallet management.
/// This module provides the local abstraction layer for BIP-44 style paths.

pub fn path_for(chain_id: &str, account_index: u32) -> String {
    if chain_id.starts_with("eip155:") {
        path_for_evm(account_index)
    } else if chain_id.starts_with("solana:") {
        path_for_solana(account_index)
    } else if chain_id.starts_with("stellar:") {
        path_for_stellar(account_index)
    } else {
        // Default to EVM path for unknown EVM-like chains
        path_for_evm(account_index)
    }
}

/// BIP-44 Ethereum path: m/44'/60'/0'/0/{account_index}
pub fn path_for_evm(account_index: u32) -> String {
    format!("m/44'/60'/0'/0/{}", account_index)
}

/// BIP-44 Solana path: m/44'/501'/{account_index}'
pub fn path_for_solana(account_index: u32) -> String {
    format!("m/44'/501'/{}'", account_index)
}

/// Stellar path (BIP-44 coin type 148): m/44'/148'/{account_index}'
pub fn path_for_stellar(account_index: u32) -> String {
    format!("m/44'/148'/{}'", account_index)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_evm_path() {
        assert_eq!(path_for_evm(0), "m/44'/60'/0'/0/0");
        assert_eq!(path_for_evm(7), "m/44'/60'/0'/0/7");
    }

    #[test]
    fn test_solana_path() {
        assert_eq!(path_for_solana(0), "m/44'/501'/0'");
        assert_eq!(path_for_solana(3), "m/44'/501'/3'");
    }

    #[test]
    fn test_dispatch() {
        assert_eq!(path_for("eip155:8453", 1), path_for_evm(1));
        assert_eq!(path_for("solana:mainnet", 2), path_for_solana(2));
        assert_eq!(path_for("stellar:pubnet", 4), path_for_stellar(4));
    }
}
