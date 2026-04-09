use crate::context::AppContext;
use anyhow::Result;
use std::fs;

pub async fn set(ctx: &AppContext, wallet_id: String, file: String) -> Result<()> {
    let content = fs::read_to_string(&file)?;
    let _: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| anyhow::anyhow!("Invalid policy JSON: {}", e))?;

    let wallet = gradience_db::queries::get_wallet_by_id(&ctx.db, &wallet_id).await?;
    if wallet.is_none() {
        anyhow::bail!("Wallet not found: {}", wallet_id);
    }

    let policy_id = gradience_core::policy::service::create_policy_sync(
        &ctx.db,
        Some(&wallet_id),
        None,
        &content,
        Some(&ctx.vault_dir),
    )
    .await?;

    println!(
        "Policy set for wallet {} (policy id: {})",
        wallet_id, policy_id
    );
    Ok(())
}

pub async fn approve(ctx: &AppContext, approval_id: String) -> Result<()> {
    let username = ctx.read_passphrase().unwrap_or_else(|| "user-1".into());
    gradience_db::queries::update_policy_approval_status(
        &ctx.db,
        &approval_id,
        "approved",
        Some(&username),
    )
    .await?;
    println!("Approved policy approval {}", approval_id);
    Ok(())
}

pub async fn reject(ctx: &AppContext, approval_id: String) -> Result<()> {
    let username = ctx.read_passphrase().unwrap_or_else(|| "user-1".into());
    gradience_db::queries::update_policy_approval_status(
        &ctx.db,
        &approval_id,
        "rejected",
        Some(&username),
    )
    .await?;
    println!("Rejected policy approval {}", approval_id);
    Ok(())
}

pub async fn list_approvals(ctx: &AppContext, wallet_id: Option<String>) -> Result<()> {
    let rows = match wallet_id {
        Some(wid) => gradience_db::queries::list_pending_policy_approvals(&ctx.db, &wid).await?,
        None => gradience_db::queries::list_all_pending_policy_approvals(&ctx.db).await?,
    };
    if rows.is_empty() {
        println!("No pending policy approvals.");
    } else {
        println!("Pending policy approvals:");
        for a in rows {
            println!(
                "  {} | wallet: {} | status: {} | {}",
                a.id, a.wallet_id, a.status, a.request_json
            );
        }
    }
    Ok(())
}
