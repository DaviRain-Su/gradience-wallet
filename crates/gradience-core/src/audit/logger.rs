// DEPRECATED: This in-memory audit logger is kept only for backward compatibility.
// All new audit logging should go through `crate::audit::service::log_wallet_action`,
// which persists logs to the database and maintains an HMAC chain.
#![allow(deprecated)]

use crate::error::{GradienceError, Result};
use serde::{Deserialize, Serialize};
use sha3::{Digest, Sha3_256};

#[deprecated(
    since = "0.1.0",
    note = "Use `crate::audit::service` for DB-backed audit logging"
)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLogEntry {
    pub id: i64,
    pub wallet_id: String,
    pub action: String,
    pub decision: String,
    pub prev_hash: String,
    pub current_hash: String,
    pub created_at: String,
}

pub struct AuditLogger {
    secret_key: Vec<u8>,
    last_hash: String,
    entries: Vec<AuditLogEntry>,
}

impl AuditLogger {
    pub fn new(secret_key: &[u8]) -> Self {
        let genesis = format!("{:x}", Sha3_256::digest(b"GENESIS"));
        Self {
            secret_key: secret_key.to_vec(),
            last_hash: genesis.clone(),
            entries: Vec::new(),
        }
    }

    pub fn log(&mut self, wallet_id: &str, action: &str, decision: &str) -> Result<AuditLogEntry> {
        let id = self.entries.len() as i64 + 1;
        let created_at = chrono::Utc::now().to_rfc3339();

        let entry_data = format!(
            "{}:{}:{}:{}:{}",
            id, wallet_id, action, decision, created_at
        );
        let current_hash = compute_audit_hash(&self.secret_key, &self.last_hash, &entry_data);

        let entry = AuditLogEntry {
            id,
            wallet_id: wallet_id.into(),
            action: action.into(),
            decision: decision.into(),
            prev_hash: self.last_hash.clone(),
            current_hash: current_hash.clone(),
            created_at,
        };

        self.last_hash = current_hash;
        self.entries.push(entry.clone());
        Ok(entry)
    }

    pub fn verify_chain(&self) -> Result<()> {
        let mut expected_prev = format!("{:x}", Sha3_256::digest(b"GENESIS"));
        for entry in &self.entries {
            if entry.prev_hash != expected_prev {
                return Err(GradienceError::Signature("audit chain broken".into()));
            }
            let entry_data = format!(
                "{}:{}:{}:{}:{}",
                entry.id, entry.wallet_id, entry.action, entry.decision, entry.created_at
            );
            let recomputed = compute_audit_hash(&self.secret_key, &entry.prev_hash, &entry_data);
            if recomputed != entry.current_hash {
                return Err(GradienceError::Signature("audit hash mismatch".into()));
            }
            expected_prev = entry.current_hash.clone();
        }
        Ok(())
    }

    pub fn secret_key(&self) -> &[u8] {
        &self.secret_key
    }

    pub fn entries(&self) -> &[AuditLogEntry] {
        &self.entries
    }
}

pub fn compute_audit_hash(secret: &[u8], prev_hash: &str, data: &str) -> String {
    let mut hasher = Sha3_256::new();
    hasher.update(secret);
    hasher.update(prev_hash.as_bytes());
    hasher.update(data.as_bytes());
    format!("{:x}", hasher.finalize())
}
