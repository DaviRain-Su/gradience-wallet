use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MppPaymentRequest {
    pub sender_wallet_id: String,
    pub recipients: Vec<MppRecipient>,
    pub token_address: String,
    pub chain: String,
    pub memo: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MppRecipient {
    pub address: String,
    pub amount: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MppPaymentResult {
    pub batch_tx_hash: String,
    pub recipient_count: usize,
}

pub struct MppService;

impl MppService {
    pub fn new() -> Self {
        Self
    }

    pub fn build_batch(
        &self,
        req: &MppPaymentRequest,
    ) -> Result<Vec<u8>, crate::error::GradienceError> {
        if req.recipients.is_empty() {
            return Err(crate::error::GradienceError::InvalidCredential(
                "no recipients".into(),
            ));
        }
        // Placeholder: in production this would encode a multi-transfer call.
        let json = serde_json::to_vec(req).unwrap_or_default();
        Ok(json)
    }
}
