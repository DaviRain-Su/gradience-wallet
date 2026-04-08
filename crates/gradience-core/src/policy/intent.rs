use crate::error::Result;
use crate::policy::engine::Intent;
use crate::ows::adapter::Transaction;

pub struct IntentParser;

impl IntentParser {
    pub fn new() -> Self {
        Self
    }

    /// Parse transaction intent and compute a simple risk score (0.0 - 1.0).
    /// This is a heuristic-based demo implementation.
    pub fn parse(&self, tx: &Transaction, chain_id: &str) -> Result<Intent> {
        let data = tx.data.clone();
        let to = tx.to.as_deref().unwrap_or("");
        let value = tx.value.parse::<u128>().unwrap_or(0);

        let mut intent_type = "transfer".to_string();
        let mut target_protocol = None;
        let mut risk_score = Some(0.1);

        // Detect swap-like behavior by common DEX selectors
        if !data.is_empty() {
            let selector = hex::encode(&data[..data.len().min(4)]);
            match selector.as_str() {
                // swapExactTokensForTokens, swapExactETHForTokens, swapExactTokensForETH
                "38ed1739" | "7ff36ab5" | "18cbafe5" |
                // exactInput, exactInputSingle (Uniswap V3)
                "c04b8d59" | "04e45aaf" => {
                    intent_type = "swap".into();
                    target_protocol = Some("uniswap".into());
                    risk_score = Some(0.4);
                }
                // addLiquidity, addLiquidityETH
                "e8e33700" | "f305d719" => {
                    intent_type = "liquidity".into();
                    target_protocol = Some("uniswap".into());
                    risk_score = Some(0.35);
                }
                // approve (ERC-20)
                "095ea7b3" => {
                    intent_type = "approve".into();
                    risk_score = Some(0.25);
                }
                // stake, deposit (common staking protocols)
                "a694fc3a" | "d0e30db0" => {
                    intent_type = "stake".into();
                    risk_score = Some(0.3);
                }
                _ => {
                    // Unknown calldata = slightly higher base risk
                    risk_score = Some(0.25);
                }
            }
        }

        // High value transfer heuristic
        let eth_value = value as f64 / 1e18;
        if intent_type == "transfer" && eth_value > 1.0 {
            risk_score = Some(risk_score.unwrap_or(0.1) + 0.2);
        }
        if intent_type == "swap" && eth_value > 0.5 {
            risk_score = Some(risk_score.unwrap_or(0.4) + 0.2);
        }

        // Unknown recipient (not a known contract) with non-trivial value
        if to.len() < 20 && eth_value > 0.01 {
            risk_score = Some((risk_score.unwrap_or(0.1f64) + 0.15f64).min(1.0f64));
        }

        // Cross-chain or unusual chain raises risk slightly
        if !chain_id.contains("8453") && !chain_id.contains("1") {
            risk_score = Some((risk_score.unwrap_or(0.1f64) + 0.1f64).min(1.0f64));
        }

        Ok(Intent {
            intent_type,
            from_token: None,
            to_token: None,
            estimated_value_usd: Some(eth_value * 2000.0), // rough ETH price
            target_protocol,
            risk_score,
        })
    }

    /// Quick parse: returns intent type string only.
    pub fn quick_type(&self, tx: &Transaction, chain_id: &str) -> String {
        self.parse(tx, chain_id).map(|i| i.intent_type).unwrap_or_else(|_| "unknown".into())
    }
}
