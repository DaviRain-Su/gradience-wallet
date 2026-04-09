use crate::error::{GradienceError, Result};
use sqlx::{Pool, Sqlite};

fn get_audit_secret() -> Vec<u8> {
    std::env::var("AUDIT_SECRET")
        .map(|s| s.into_bytes())
        .unwrap_or_else(|_| b"gradience-audit-secret-v1".to_vec())
}

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
    let secret_key = get_audit_secret();

    let prev_hash = match gradience_db::queries::list_audit_logs_by_wallet(db, wallet_id, 1).await {
        Ok(mut logs) if !logs.is_empty() => logs.remove(0).current_hash,
        _ => {
            use sha3::Digest;
            format!(
                "{:x}",
                sha3::Sha3_256::new().chain_update(b"GENESIS").finalize()
            )
        }
    };

    let created_at = chrono::Utc::now().to_rfc3339();
    let entry_data = format!(
        "{}:{}:{}:{}:{}",
        wallet_id, action, decision, context_json, created_at
    );
    let current_hash = compute_audit_hash(&secret_key, &prev_hash, &entry_data);

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
    let secret_key = get_audit_secret();
    let mut logs = gradience_db::queries::list_audit_logs_by_wallet(db, wallet_id, i64::MAX)
        .await
        .map_err(|e| GradienceError::Database(e.to_string()))?;
    logs.reverse(); // oldest first

    let mut expected_prev = {
        use sha3::Digest;
        format!(
            "{:x}",
            sha3::Sha3_256::new().chain_update(b"GENESIS").finalize()
        )
    };
    for log in &logs {
        if log.prev_hash != expected_prev {
            return Err(GradienceError::Signature("audit chain broken".into()));
        }
        let entry_data = format!(
            "{}:{}:{}:{}:{}",
            log.wallet_id,
            log.action,
            log.decision,
            log.context_json,
            log.created_at.to_rfc3339()
        );
        let recomputed = compute_audit_hash(&secret_key, &log.prev_hash, &entry_data);
        if recomputed != log.current_hash {
            return Err(GradienceError::Signature("audit hash mismatch".into()));
        }
        expected_prev = log.current_hash.clone();
    }
    Ok(())
}

/// Generate a Merkle proof for a specific audit log within a wallet's log set.
pub async fn generate_merkle_proof_for_log(
    db: &Pool<Sqlite>,
    wallet_id: &str,
    log_id: i64,
) -> Result<(Vec<String>, String, String)> {
    let logs = gradience_db::queries::list_audit_logs_by_wallet(db, wallet_id, i64::MAX)
        .await
        .map_err(|e| GradienceError::Database(e.to_string()))?;

    if logs.is_empty() {
        return Err(GradienceError::NotFound("no audit logs for wallet".into()));
    }

    let index = logs
        .iter()
        .position(|l| l.id == log_id)
        .ok_or_else(|| GradienceError::NotFound(format!("log_id {} not found", log_id)))?;

    let leaves: Vec<[u8; 32]> = logs
        .iter()
        .map(|l| {
            let mut buf = [0u8; 32];
            let bytes = hex::decode(l.current_hash.trim_start_matches("0x")).unwrap_or_else(|_| {
                crate::audit::merkle::keccak256(l.current_hash.as_bytes()).to_vec()
            });
            let n = buf.len().min(bytes.len());
            buf[..n].copy_from_slice(&bytes[..n]);
            buf
        })
        .collect();

    let tree = crate::audit::merkle::MerkleTree::new(leaves);
    let (proof, leaf) = tree
        .generate_proof(index)
        .ok_or_else(|| GradienceError::Signature("merkle proof generation failed".into()))?;

    let proof_hex: Vec<String> = proof
        .iter()
        .map(|p| format!("0x{}", hex::encode(p)))
        .collect();
    let leaf_hex = format!("0x{}", hex::encode(leaf));
    let root_hex = format!("0x{}", hex::encode(tree.root));

    Ok((proof_hex, leaf_hex, root_hex))
}
