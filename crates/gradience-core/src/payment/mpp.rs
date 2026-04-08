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

    /// Build batch transfer calldata for multi-recipient payments.
    /// For EVM chains: encodes individual ERC20 `transfer(address,uint256)` calls.
    /// Returns a Vec of (to_address, calldata) tuples serialized as JSON.
    /// For native token: returns transfer tuples with empty calldata.
    pub fn build_batch(
        &self,
        req: &MppPaymentRequest,
    ) -> Result<Vec<u8>, crate::error::GradienceError> {
        if req.recipients.is_empty() {
            return Err(crate::error::GradienceError::InvalidCredential(
                "no recipients".into(),
            ));
        }

        let is_native = req.token_address.is_empty()
            || req.token_address == "0x0000000000000000000000000000000000000000";

        let mut transfers: Vec<BatchTransfer> = Vec::with_capacity(req.recipients.len());

        for r in &req.recipients {
            let amount = r.amount.parse::<u128>().map_err(|e| {
                crate::error::GradienceError::Validation(format!("bad amount: {}", e))
            })?;

            if is_native {
                transfers.push(BatchTransfer {
                    to: r.address.clone(),
                    value: r.amount.clone(),
                    data: String::new(),
                });
            } else {
                // ERC20 transfer(address,uint256) selector: 0xa9059cbb
                let mut calldata = vec![0xa9u8, 0x05, 0x9c, 0xbb];
                let addr_bytes = hex::decode(r.address.trim_start_matches("0x"))
                    .map_err(|e| {
                        crate::error::GradienceError::Validation(format!(
                            "bad address: {}",
                            e
                        ))
                    })?;
                // Left-pad address to 32 bytes
                calldata.extend_from_slice(&[0u8; 12]);
                calldata.extend_from_slice(&addr_bytes);
                // Left-pad amount to 32 bytes
                let mut amount_bytes = [0u8; 32];
                amount_bytes[16..].copy_from_slice(&amount.to_be_bytes());
                calldata.extend_from_slice(&amount_bytes);

                transfers.push(BatchTransfer {
                    to: req.token_address.clone(),
                    value: "0".into(),
                    data: format!("0x{}", hex::encode(&calldata)),
                });
            }
        }

        serde_json::to_vec(&transfers).map_err(|e| {
            crate::error::GradienceError::Validation(format!("serialize batch: {}", e))
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BatchTransfer {
    to: String,
    value: String,
    data: String,
}
