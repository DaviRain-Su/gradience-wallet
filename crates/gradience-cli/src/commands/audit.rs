use crate::context::AppContext;
use anyhow::Result;

pub async fn list(ctx: &AppContext, wallet_id: String) -> Result<()> {
    let logs = gradience_db::queries::list_audit_logs_by_wallet(&ctx.db, &wallet_id, 50).await?;
    println!("Audit logs for wallet {}:", wallet_id);
    for log in logs {
        println!(
            "  [{}] {} -> {} (hash: {})",
            log.created_at, log.action, log.decision, log.current_hash
        );
    }
    Ok(())
}

pub async fn verify(ctx: &AppContext, wallet_id: String) -> Result<()> {
    gradience_core::audit::service::verify_wallet_audit_chain(&ctx.db, &wallet_id).await?;
    println!("Audit chain for wallet {} is valid.", wallet_id);
    Ok(())
}

pub async fn export(
    ctx: &AppContext,
    wallet_id: String,
    format: &str,
    output: String,
) -> Result<()> {
    let logs =
        gradience_db::queries::list_audit_logs_by_wallet(&ctx.db, &wallet_id, i64::MAX).await?;
    let count = logs.len();

    if format == "csv" {
        let mut csv = String::from("id,wallet_id,action,decision,tx_hash,created_at\n");
        for l in &logs {
            csv.push_str(&format!(
                "{},{},{},{},{},{}\n",
                l.id,
                l.wallet_id,
                l.action,
                l.decision,
                l.tx_hash.as_deref().unwrap_or(""),
                l.created_at.to_rfc3339()
            ));
        }
        std::fs::write(&output, csv)?;
    } else {
        let json: Vec<_> = logs
            .into_iter()
            .map(|l| {
                serde_json::json!({
                    "id": l.id,
                    "wallet_id": l.wallet_id,
                    "action": l.action,
                    "decision": l.decision,
                    "tx_hash": l.tx_hash,
                    "created_at": l.created_at.to_rfc3339(),
                })
            })
            .collect();
        std::fs::write(&output, serde_json::to_string_pretty(&json)?)?;
    }

    println!("Exported {} audit log(s) to {}", count, output);
    Ok(())
}
