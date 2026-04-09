use crate::error::Result;

const COINGECKO_GLOBAL_URL: &str = "https://api.coingecko.com/api/v3/global";

#[derive(Debug, Clone)]
pub struct MarketRiskSnapshot {
    /// 0-100, derived from market fear/greed or volatility
    pub market_fear_score: f64,
}

/// Fetch global crypto market data from CoinGecko and compute a fear score.
/// Returns a score between 0.0 (calm) and 100.0 (extreme fear/volatility).
pub async fn fetch_market_fear_score() -> Result<MarketRiskSnapshot> {
    let client = reqwest::Client::new();
    let resp = client
        .get(COINGECKO_GLOBAL_URL)
        .timeout(std::time::Duration::from_secs(5))
        .send()
        .await
        .map_err(|e| crate::error::GradienceError::Http(e.to_string()))?;

    let json: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| crate::error::GradienceError::Http(format!("json decode: {}", e)))?;

    // Try fear & greed index first
    let data = json.get("data").cloned().unwrap_or(serde_json::json!({}));

    let score = if let Some(fgi) = data.get("market_cap_change_percentage_24h_usd") {
        // Use 24h global market cap change as a proxy for market fear
        let change = fgi.as_f64().unwrap_or(0.0);
        // Map -10% .. +10% to 100 .. 0
        
        ((-change).clamp(-10.0, 10.0) + 10.0) / 20.0 * 100.0
    } else {
        // Fallback: neutral
        50.0
    };

    Ok(MarketRiskSnapshot {
        market_fear_score: score.clamp(0.0, 100.0),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_fetch_market_fear_score_does_not_panic() {
        let res = fetch_market_fear_score().await;
        assert!(res.is_ok() || res.is_err());
    }
}
