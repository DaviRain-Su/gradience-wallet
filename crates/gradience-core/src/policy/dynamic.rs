use crate::error::{GradienceError, Result};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone)]
pub struct RiskSignal {
    pub score: f64,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Default)]
pub struct RiskSignalCache {
    inner: Arc<Mutex<HashMap<(String, String), RiskSignal>>>,
}

impl RiskSignalCache {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set(&self, wallet_id: &str, signal_type: &str, score: f64) {
        let mut map = self.inner.lock().unwrap();
        map.insert(
            (wallet_id.into(), signal_type.into()),
            RiskSignal {
                score,
                updated_at: chrono::Utc::now(),
            },
        );
    }

    pub fn get(&self, wallet_id: &str, signal_type: &str) -> Option<RiskSignal> {
        let map = self.inner.lock().unwrap();
        map.get(&(wallet_id.into(), signal_type.into())).cloned()
    }

    pub fn evaluate(&self, wallet_id: &str, max_forta: f64, max_chainalysis: f64) -> Result<(bool, Vec<String>)> {
        let mut reasons = Vec::new();
        let mut denied = false;

        if let Some(sig) = self.get(wallet_id, "forta") {
            if sig.score > max_forta {
                reasons.push(format!("Forta risk score {} exceeds threshold {}", sig.score, max_forta));
                denied = true;
            }
        }
        if let Some(sig) = self.get(wallet_id, "chainalysis") {
            if sig.score > max_chainalysis {
                reasons.push(format!("Chainalysis risk score {} exceeds threshold {}", sig.score, max_chainalysis));
                denied = true;
            }
        }

        Ok((denied, reasons))
    }
}

/// Mock fetcher that generates random risk signals for demo purposes.
pub async fn mock_fetch_signals(cache: RiskSignalCache, interval_sec: u64) {
    use rand::Rng;
    let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(interval_sec));
    loop {
        interval.tick().await;
        let mut rng = rand::thread_rng();
        let forta = rng.gen::<f64>();
        let chainalysis = rng.gen::<f64>();
        // Use a wildcard wallet key so any wallet can be checked against the latest global signal.
        cache.set("*", "forta", forta);
        cache.set("*", "chainalysis", chainalysis);
    }
}
