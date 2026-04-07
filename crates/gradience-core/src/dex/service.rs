use crate::error::{GradienceError, Result};
use crate::ows::adapter::Transaction;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwapQuote {
    pub from_token: String,
    pub to_token: String,
    pub from_amount: String,
    pub to_amount: String,
    pub price_impact: String,
    pub provider: String,
}

pub struct DexService;

impl DexService {
    pub fn new() -> Self {
        Self
    }

    /// Mock quote for demo. In production, wire 1inch / Jupiter Quote API.
    pub async fn get_quote(
        &self,
        _wallet_id: &str,
        from: &str,
        to: &str,
        amount: &str,
    ) -> Result<SwapQuote> {
        if from.eq_ignore_ascii_case(to) {
            return Err(GradienceError::InvalidCredential("same token swap".into()));
        }
        let amount_f64: f64 = amount.parse().unwrap_or(0.0);
        let out = amount_f64 * 0.997;
        Ok(SwapQuote {
            from_token: from.into(),
            to_token: to.into(),
            from_amount: amount.into(),
            to_amount: format!("{:.6}", out),
            price_impact: "0.30%".into(),
            provider: "mock-aggregator".into(),
        })
    }

    /// Build a real unsigned swap transaction.
    /// Tries 1inch Swap API first (requires ONEINCH_API_KEY env).
    /// Falls back to raw Uniswap V3 exactInputSingle calldata.
    pub async fn build_swap_tx(
        &self,
        from_addr: &str,
        from_token: &str,
        to_token: &str,
        amount: &str,
        chain_num: u64,
    ) -> Result<Transaction> {
        // Try 1inch if API key is present
        if let Ok(key) = std::env::var("ONEINCH_API_KEY") {
            let client = super::oneinch::OneInchClient::new(key);
            let inch_tx = client
                .swap(
                    chain_num,
                    from_token,
                    to_token,
                    amount,
                    from_addr,
                    1.0, // 1% slippage
                )
                .await?;
            return Ok(Transaction {
                to: Some(inch_tx.to),
                value: inch_tx.value,
                data: hex::decode(inch_tx.data.trim_start_matches("0x"))
                    .unwrap_or_default(),
                raw_hex: inch_tx.data,
            });
        }

        // Fallback: Uniswap V3 exactInputSingle on Base
        if chain_num != 8453 {
            return Err(GradienceError::Validation(
                "fallback Uniswap V3 only available on Base (8453)".into(),
            ));
        }

        let amount_hex = format!("0x{:x}", amount.parse::<u128>().unwrap_or(0));
        let min_out = "0x0"; // demo: accept any output
        let sqrt_price_limit = "0x0";
        let uni = super::uniswap::encode_exact_input_single(
            from_token,
            to_token,
            3000, // 0.3% pool
            from_addr,
            &amount_hex,
            min_out,
            sqrt_price_limit,
        )?;

        Ok(Transaction {
            to: Some(uni.to),
            value: uni.value,
            data: uni.data.clone(),
            raw_hex: hex::encode(&uni.data),
        })
    }
}
