pub mod ai;
pub mod chain;
pub mod config;
pub mod error;
pub mod identity;
pub mod wallet;
pub mod policy;
pub mod dex;
pub mod payment;
pub mod audit;
pub mod ows;
pub mod rpc;
pub mod team;
pub mod portfolio;

pub use error::{GradienceError, Result};

/// Parse an ETH amount string (e.g. "0.001" or "1.5") into wei (u128).
/// Returns an error if the string is malformed or has more than 18 decimals.
pub fn eth_to_wei(amount: &str) -> Result<u128> {
    let amount = amount.trim();
    if amount.is_empty() {
        return Err(GradienceError::Validation("empty amount".into()));
    }

    let (int_part, frac_part) = match amount.find('.') {
        Some(idx) => (&amount[..idx], &amount[idx + 1..]),
        None => (amount, ""),
    };

    if frac_part.len() > 18 {
        return Err(GradienceError::Validation("too many decimals".into()));
    }

    let int = int_part.parse::<u128>()
        .map_err(|_| GradienceError::Validation("invalid integer part".into()))?;

    let frac_padded = format!("{:0<18}", frac_part);
    let frac = frac_padded.parse::<u128>()
        .map_err(|_| GradienceError::Validation("invalid fractional part".into()))?;

    let wei = int.saturating_mul(1_000_000_000_000_000_000u128)
        .saturating_add(frac);

    Ok(wei)
}

#[cfg(test)]
mod tests {
    use super::*;
    pub mod wallet_manager_tests;
    pub mod ows_adapter_tests;
    pub mod policy_engine_tests;
    pub mod evm_rpc_tests;
    pub mod audit_logger_tests;
    pub mod merkle_tests;
    pub mod team_tests;
    pub mod payment_tests;
    pub mod mpp_integration_tests;
    pub mod dex_service_tests;

    #[test]
    fn test_eth_to_wei_basic() {
        assert_eq!(eth_to_wei("1").unwrap(), 1_000_000_000_000_000_000u128);
        assert_eq!(eth_to_wei("0.1").unwrap(), 100_000_000_000_000_000u128);
        assert_eq!(eth_to_wei("0.001").unwrap(), 1_000_000_000_000_000u128);
        assert_eq!(eth_to_wei("0.000000000000000001").unwrap(), 1u128);
    }

    #[test]
    fn test_eth_to_wei_zero() {
        assert_eq!(eth_to_wei("0").unwrap(), 0u128);
        assert_eq!(eth_to_wei("0.0").unwrap(), 0u128);
    }

    #[tokio::test]
    async fn test_build_swap_tx_base_fallback() {
        let svc = crate::dex::service::DexService::new();
        let tx = svc
            .build_swap_tx(
                "0x742d35Cc6634C0532925a3b844Bc9e7595f2bD0C",
                "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913",
                "0x4200000000000000000000000000000000000006",
                "1000000",
                8453,
                50,
            )
            .await
            .unwrap();
        assert!(tx.to.is_some());
        assert!(!tx.data.is_empty());
    }

    #[tokio::test]
    async fn test_get_quote_fallback() {
        let svc = crate::dex::service::DexService::new();
        let q = svc
            .get_quote(
                "w1",
                "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913",
                "0x4200000000000000000000000000000000000006",
                "1000000",
                8453,
            )
            .await
            .unwrap();
        assert_eq!(q.from_token, "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913");
        assert!(!q.to_amount.is_empty());
    }
}

