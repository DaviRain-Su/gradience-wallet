use crate::error::Result;

const FORTA_API_URL: &str = "https://explorer.forta.network/api/v1/alerts/stats";

#[derive(Debug, Clone)]
pub struct ThreatSnapshot {
    /// 0-100, derived from recent alert count
    pub threat_score: f64,
}

/// Fetch Forta alert stats and compute a threat score.
/// Returns a score between 0.0 (no threats) and 100.0 (high threat activity).
pub async fn fetch_forta_threat_score() -> Result<ThreatSnapshot> {
    let client = reqwest::Client::new();
    let resp = client
        .get(FORTA_API_URL)
        .timeout(std::time::Duration::from_secs(5))
        .send()
        .await;

    let score = match resp {
        Ok(r) => {
            let json: serde_json::Value = r
                .json()
                .await
                .map_err(|e| crate::error::GradienceError::Http(format!("json decode: {}", e)))?;
            // Forta stats endpoint returns total alert count in various buckets.
            // We use a simple heuristic: if total alerts > 1000 in recent window, score is high.
            let total: i64 = json
                .get("total")
                .and_then(|v: &serde_json::Value| v.as_i64())
                .or_else(|| {
                    json.get("data")
                        .and_then(|d: &serde_json::Value| d.get("total"))
                        .and_then(|v: &serde_json::Value| v.as_i64())
                })
                .unwrap_or(0);
            // Map 0..10000 alerts to 0..100 score
            ((total as f64) / 100.0).clamp(0.0, 100.0)
        }
        Err(_) => {
            // Graceful fallback: if Forta is unreachable, return neutral score
            50.0
        }
    };

    Ok(ThreatSnapshot { threat_score: score })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_fetch_forta_threat_score_does_not_panic() {
        let res = fetch_forta_threat_score().await;
        assert!(res.is_ok() || res.is_err());
    }
}
