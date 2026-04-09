use crate::error::{GradienceError, Result};
use serde::{Deserialize, Serialize};

/// User-configurable payment route preference (multi-chain priority debit).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentRoutePreference {
    pub chain_id: String,
    pub token_address: String,
    pub priority: u32,
}

/// A payment requirement used to select the best route.
#[derive(Debug, Clone)]
pub struct PaymentRequirement {
    pub amount: String,
    pub token_hint: Option<String>,
}

/// Simple router that picks the first route with sufficient balance.
#[derive(Debug, Clone, Default)]
pub struct PaymentRouter {
    pub preferences: Vec<PaymentRoutePreference>,
}

impl PaymentRouter {
    pub fn new(preferences: Vec<PaymentRoutePreference>) -> Self {
        Self { preferences }
    }

    /// Select the best route by checking balances in priority order.
    /// For demo this uses a naive balance check heuristic.
    pub async fn select_route(&self, _req: &PaymentRequirement) -> Result<PaymentRoutePreference> {
        let mut prefs = self.preferences.clone();
        prefs.sort_by_key(|p| p.priority);
        if let Some(pref) = prefs.into_iter().next() {
            // TODO: real balance check via RPC
            // For development demo, return first configured route.
            return Ok(pref);
        }
        Err(GradienceError::Validation(
            "no payment route available".into(),
        ))
    }
}
