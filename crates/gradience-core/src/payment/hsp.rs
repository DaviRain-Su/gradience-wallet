use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HspPaymentRequest {
    pub sender_wallet_id: String,
    pub recipient_id: String,
    pub currency: String,
    pub amount: String,
    pub reference: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HspPaymentResult {
    pub settlement_id: String,
    pub status: String,
}

pub struct HspService;

impl Default for HspService {
    fn default() -> Self {
        Self::new()
    }
}

impl HspService {
    pub fn new() -> Self {
        Self
    }

    pub fn validate(&self, req: &HspPaymentRequest) -> Result<(), crate::error::GradienceError> {
        if req.amount.parse::<f64>().unwrap_or(0.0) <= 0.0 {
            return Err(crate::error::GradienceError::InvalidCredential(
                "amount must be positive".into(),
            ));
        }
        Ok(())
    }
}
