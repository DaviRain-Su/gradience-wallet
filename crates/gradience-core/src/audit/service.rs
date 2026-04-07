use crate::error::{GradienceError, Result};
use sqlx::{Pool, Sqlite};

fn compute_audit_hash(secret: &[u8], prev_hash: &str, data: &str) -> String {
    use sha3::Digest;
    let mut hasher = sha3::Sha3_256::new();
    hasher.update(secret);
    hasher.update(prev_hash.as_bytes());
    hasher.update(data.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// Write an audit log entry to the database, maintaining HMAC chain integrity.
pub async fn log_wallet_action(
    db: &Pool<Sqlite>,
    wallet_id: &str,
    api_key_id: Option<&str>,
    action: &str,
    context_json: &str,
    decision: &str,
) -> Result<i64> {
    let secret_key = b"gradience-audit-secret-v1";

    let prev_hash = match gradience_db::queries::list_audit_logs_by_wallet(db, wallet_id, 1).await {
        Ok(mut logs) if !logs.is_empty() => logs.remove(0).current_hash,
        _ => {
            use sha3::Digest;
            format!("{:x}", sha3::Sha3_256::new().chain_update(b"GENESIS").finalize())
        }
    };

    let created_at = chrono::Utc::now().to_rfc3339();
    let entry_data = format!(
        "{}:{}:{}:{}:{}",
        wallet_id, action, decision, context_json, created_at
    );
    let current_hash = compute_audit_hash(secret_key, &prev_hash, &entry_data);

    let id = gradience_db::queries::insert_audit_log(
        db,
        wallet_id,
        api_key_id,
        action,
        context_json,
        decision,
        &prev_hash,
        &current_hash,
    )
    .await
    .map_err(|e| GradienceError::Database(e.to_string()))?;

    Ok(id)
}

/// Verify the HMAC chain for a wallet's audit logs.
pub async fn verify_wallet_audit_chain(db: &Pool<Sqlite>, wallet_id: &str) -> Result<()> {
    let secret_key = b"gradience-audit-secret-v1";
    let mut logs = gradience_db::queries::list_audit_logs_by_wallet(db, wallet_id, i64::MAX)
        .await
        .map_err(|e| GradienceError::Database(e.to_string()))?;
    logs.reverse(); // oldest first

    let mut expected_prev = {
        use sha3::Digest;
        format!("{:x}", sha3::Sha3_256::new().chain_update(b"GENESIS").finalize())
    };
    for log in &logs {
        if log.prev_hash != expected_prev {
            return Err(GradienceError::Signature("audit chain broken".into()));
        }
        let entry_data = format!(
            "{}:{}:{}:{}:{}",
            log.wallet_id, log.action, log.decision, log.context_json, log.created_at.to_rfc3339()
        );
        let recomputed = compute_audit_hash(secret_key, &log.prev_hash, &entry_data);
        if recomputed != log.current_hash {
            return Err(GradienceError::Signature("audit hash mismatch".into()));
        }
        expected_prev = log.current_hash.clone();
    }
    Ok(())
}
