use crate::context::AppContext;
use anyhow::Result;

pub async fn list(ctx: &AppContext, wallet_id: String) -> Result<()> {
    let logs = gradience_db::queries::list_audit_logs_by_wallet(&ctx.db, &wallet_id, 50)
        .await?;
    println!("Audit logs for wallet {}:", wallet_id);
    for log in logs {
        println!("  [{}] {} -> {} (hash: {})", log.created_at, log.action, log.decision, log.current_hash);
    }
    Ok(())
}

pub async fn verify(ctx: &AppContext, wallet_id: String) -> Result<()> {
    gradience_core::audit::service::verify_wallet_audit_chain(&ctx.db, &wallet_id).await?;
    println!("Audit chain for wallet {} is valid.", wallet_id);
    Ok(())
}
