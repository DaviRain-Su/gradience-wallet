use serde::{Deserialize, Serialize};
use crate::error::{GradienceError, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct X402Requirement {
    pub scheme: String,
    pub network: String,
    pub token_address: String,
    pub recipient: String,
    pub amount: String,
    pub deadline: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct X402Payment {
    pub requirement: X402Requirement,
    pub signature: String,
    pub tx_hash: Option<String>,
}

pub struct X402Service;

impl X402Service {
    pub fn new() -> Self {
        Self
    }

    pub fn create_requirement(
        &self,
        recipient: &str,
        amount: &str,
        token_address: &str,
        deadline: u64,
    ) -> Result<X402Requirement> {
        if recipient.is_empty() || !recipient.starts_with("0x") {
            return Err(GradienceError::InvalidCredential("invalid recipient address".into()));
        }
        Ok(X402Requirement {
            scheme: "exact".into(),
            network: "base".into(),
            token_address: token_address.into(),
            recipient: recipient.into(),
            amount: amount.into(),
            deadline,
        })
    }

    pub fn sign_payment(
        &self,
        requirement: X402Requirement,
        signature: &str,
    ) -> Result<X402Payment> {
        if signature.len() < 10 {
            return Err(GradienceError::Signature("invalid signature length".into()));
        }
        Ok(X402Payment {
            requirement,
            signature: signature.into(),
            tx_hash: None,
        })
    }

    pub fn verify_receipt(&self, payment: &X402Payment, current_time: u64) -> Result<bool> {
        if current_time > payment.requirement.deadline {
            return Ok(false);
        }
        if payment.signature.is_empty() {
            return Ok(false);
        }
        Ok(true)
    }
}
