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

    /// Multi-chain quote aggregator.
    /// Priority: Jupiter (Solana) > 1inch (EVM, key required) > Uniswap V3 eth_call > estimate.
    pub async fn get_quote(
        &self,
        _wallet_id: &str,
        from: &str,
        to: &str,
        amount: &str,
        chain_num: u64,
    ) -> Result<SwapQuote> {
        if from.eq_ignore_ascii_case(to) {
            return Err(GradienceError::InvalidCredential("same token swap".into()));
        }

        // 1) Jupiter for Solana
        if chain_num == 101 {
            let jup = super::jupiter::JupiterClient::new();
            let input_mint = super::jupiter::resolve_solana_mint(from);
            let output_mint = super::jupiter::resolve_solana_mint(to);
            let jup_amount = super::jupiter::normalize_solana_amount(from, amount);
            match jup.quote(&input_mint, &output_mint, &jup_amount, 50).await {
                Ok((q, _)) => {
                    let out_f = q.out_amount.parse::<f64>().unwrap_or(0.0);
                    let impact = format!("{}%", q.price_impact_pct);
                    return Ok(SwapQuote {
                        from_token: from.into(),
                        to_token: to.into(),
                        from_amount: amount.into(),
                        to_amount: format!("{:.0}", out_f),
                        price_impact: impact,
                        provider: "jupiter".into(),
                    });
                }
                Err(e) => {
                    tracing::warn!("Jupiter quote failed: {}, falling back", e);
                }
            }
        }

        // 2) 1inch for EVM if API key present
        if let Ok(key) = std::env::var("ONEINCH_API_KEY") {
            let client = super::oneinch::OneInchClient::new(key);
            match client.quote(chain_num, from, to, amount).await {
                Ok(q) => {
                    return Ok(SwapQuote {
                        from_token: from.into(),
                        to_token: to.into(),
                        from_amount: amount.into(),
                        to_amount: q.to_amount,
                        price_impact: q.price_impact,
                        provider: "1inch".into(),
                    });
                }
                Err(e) => {
                    tracing::warn!("1inch quote failed: {}, falling back", e);
                }
            }
        }

        // 3) Uniswap V3 QuoterV2 via eth_call
        let rpc_url = rpc_url_for_chain(chain_num);
        let client = crate::rpc::evm::EvmRpcClient::new("evm", rpc_url)?;
        let cfg = super::uniswap::router_for_chain(chain_num);
        if let Some(quoter) = cfg.quoter {
            let amount_u = amount.parse::<u128>().unwrap_or(0);
            // Try common fee tiers and pick the best output.
            let mut best_out: Option<u128> = None;
            for fee in [500u32, 3000, 10000] {
                match super::uniswap::encode_quote_exact_input_single(from, to, fee, amount_u, 0) {
                    Ok(data) => {
                        match client.eth_call(&quoter, &format!("0x{}", hex::encode(&data))).await {
                            Ok(resp) => {
                                let hex = resp.trim_start_matches("0x");
                                if hex.len() >= 64 {
                                    let out = u128::from_str_radix(&hex[0..64], 16).unwrap_or(0);
                                    if out > 0 {
                                        if best_out.map(|b| out > b).unwrap_or(true) {
                                            best_out = Some(out);
                                        }
                                    }
                                } else if hex.is_empty() || hex.starts_with("08c379a0") {
                                    tracing::warn!("Uniswap quoter reverted for fee={}", fee);
                                }
                            }
                            Err(e) => {
                                tracing::warn!("Uniswap eth_call failed for fee={}: {}", fee, e);
                            }
                        }
                    }
                    Err(e) => {
                        tracing::warn!("Uniswap encode failed for fee={}: {}", fee, e);
                    }
                }
            }
            if let Some(out) = best_out {
                return Ok(SwapQuote {
                    from_token: from.into(),
                    to_token: to.into(),
                    from_amount: amount.into(),
                    to_amount: format!("{}", out),
                    price_impact: "0.30%".into(),
                    provider: "uniswap-v3-quoter".into(),
                });
            }
            tracing::warn!("Uniswap quoter returned no valid quote for any fee tier, using estimate");
        }

        // 4) Heuristic estimate
        let amount_f64: f64 = amount.parse().unwrap_or(0.0);
        let out = amount_f64 * 0.997;
        Ok(SwapQuote {
            from_token: from.into(),
            to_token: to.into(),
            from_amount: amount.into(),
            to_amount: format!("{:.6}", out),
            price_impact: "0.30%".into(),
            provider: "estimate".into(),
        })
    }

    /// Build an unsigned swap transaction.
    /// Priority: LI.FI (cross-chain / EVM aggregator) > 1inch > Uniswap V3 fallback.
    /// Solana (Jupiter) returns a placeholder because Solana tx signing is not yet wired.
    pub async fn build_swap_tx(
        &self,
        from_addr: &str,
        from_token: &str,
        to_token: &str,
        amount: &str,
        chain_num: u64,
        slippage_bps: u16,
    ) -> Result<Transaction> {
        let slippage_pct = slippage_bps as f64 / 100.0;

        // 1) Solana swap via Jupiter
        if chain_num == 101 {
            let jup = super::jupiter::JupiterClient::new();
            let input_mint = super::jupiter::resolve_solana_mint(from_token);
            let output_mint = super::jupiter::resolve_solana_mint(to_token);
            let jup_amount = super::jupiter::normalize_solana_amount(from_token, amount);
            let (_quote, quote_json) = jup.quote(&input_mint, &output_mint, &jup_amount, slippage_bps).await?;
            let swap = jup.swap(&quote_json, from_addr).await?;
            use base64::Engine;
            let tx_bytes = base64::engine::general_purpose::STANDARD
                .decode(swap.swap_transaction)
                .map_err(|e| GradienceError::Http(format!("base64 decode failed: {}", e)))?;
            return Ok(Transaction {
                to: None,
                value: "0".into(),
                data: tx_bytes.clone(),
                raw_hex: format!("0x{}", hex::encode(&tx_bytes)),
            });
        }

        // 2) LI.FI for EVM / cross-chain
        let lifi_from = super::lifi::resolve_token_address(chain_num, from_token);
        let lifi_to = super::lifi::resolve_token_address(chain_num, to_token);
        let lifi_client = super::lifi::LiFiClient::new();
        match lifi_client.quote(
            chain_num, chain_num, &lifi_from, &lifi_to, amount, from_addr, slippage_pct
        ).await {
            Ok(lq) => {
                if let Some(tx_req) = lq.transaction_request {
                    return Ok(Transaction {
                        to: Some(tx_req.to),
                        value: tx_req.value,
                        data: hex::decode(tx_req.data.trim_start_matches("0x")).unwrap_or_default(),
                        raw_hex: tx_req.data,
                    });
                }
                // No transactionRequest returned: build Uniswap fallback using LI.FI min out
                let amount_hex = format!("0x{:x}", amount.parse::<u128>().unwrap_or(0));
                let to_amount = lq.estimate.to_amount_min.parse::<u128>().unwrap_or(0);
                let min_out = to_amount.saturating_mul((10000u128 - slippage_bps as u128) / 10000);
                let min_out_hex = format!("0x{:x}", min_out);
                let uni = super::uniswap::encode_exact_input_single(
                    from_token, to_token, 3000, from_addr, &amount_hex, &min_out_hex, "0x0", chain_num,
                )?;
                return Ok(Transaction {
                    to: Some(uni.to),
                    value: uni.value,
                    data: uni.data.clone(),
                    raw_hex: hex::encode(&uni.data),
                });
            }
            Err(e) => {
                tracing::warn!("LI.FI swap routing failed: {}, trying next fallback", e);
            }
        }

        // 3) 1inch if API key is present
        if let Ok(key) = std::env::var("ONEINCH_API_KEY") {
            let client = super::oneinch::OneInchClient::new(key);
            if let Ok(inch_tx) = client.swap(chain_num, from_token, to_token, amount, from_addr, slippage_pct).await {
                return Ok(Transaction {
                    to: Some(inch_tx.to),
                    value: inch_tx.value,
                    data: hex::decode(inch_tx.data.trim_start_matches("0x")).unwrap_or_default(),
                    raw_hex: inch_tx.data,
                });
            }
        }

        // 4) Uniswap V3 exactInputSingle fallback
        let amount_hex = format!("0x{:x}", amount.parse::<u128>().unwrap_or(0));
        let quote = self.get_quote("", from_token, to_token, amount, chain_num).await?;
        let to_amount = quote.to_amount.parse::<u128>().unwrap_or(0);
        let min_out = to_amount.saturating_mul((10000u128 - slippage_bps as u128) / 10000);
        let min_out_hex = format!("0x{:x}", min_out);
        let uni = super::uniswap::encode_exact_input_single(
            from_token, to_token, 3000, from_addr, &amount_hex, &min_out_hex, "0x0", chain_num,
        )?;

        Ok(Transaction {
            to: Some(uni.to),
            value: uni.value,
            data: uni.data.clone(),
            raw_hex: hex::encode(&uni.data),
        })
    }
}

fn rpc_url_for_chain(chain_num: u64) -> &'static str {
    match chain_num {
        8453 => "https://mainnet.base.org",
        1 => "https://eth.llamarpc.com",
        42161 => "https://arb1.arbitrum.io/rpc",
        10 => "https://mainnet.optimism.io",
        56 => "https://bsc-dataseed.bnbchain.org",
        _ => "https://eth.llamarpc.com",
    }
}
