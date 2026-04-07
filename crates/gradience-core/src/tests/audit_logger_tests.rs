use crate::audit::logger::{AuditLogger, compute_audit_hash};
use crate::error::GradienceError;

#[test]
fn test_audit_hmac_chain_integrity() {
    let mut logger = AuditLogger::new(b"secret-test-key");
    let entry1 = logger.log("wallet-1", "sign_tx", "allowed").unwrap();
    let entry2 = logger.log("wallet-1", "sign_tx", "allowed").unwrap();

    assert_eq!(entry2.prev_hash, entry1.current_hash);
    assert!(logger.verify_chain().is_ok());
}

#[test]
fn test_audit_tamper_detection() {
    let mut logger = AuditLogger::new(b"secret-test-key");
    let mut entry = logger.log("wallet-1", "sign_tx", "allowed").unwrap();

    // Tamper decision
    entry.decision = "denied".into();
    let recomputed = compute_audit_hash(
        logger.secret_key(),
        &entry.prev_hash,
        &format!("{}:{}:{}:{}:{}", entry.id, entry.wallet_id, entry.action, entry.decision, entry.created_at),
    );
    assert_ne!(recomputed, entry.current_hash);
}
