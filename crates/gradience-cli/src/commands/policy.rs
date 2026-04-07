use crate::context::AppContext;
use anyhow::Result;
use std::fs;

pub async fn set(ctx: &AppContext, wallet_id: String, file: String) -> Result<()> {
    let content = fs::read_to_string(&file)?;
    // Fast-fail on malformed JSON before touching the wallet or vault.
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
    ).await?;

    println!("Policy set for wallet {} (policy id: {})", wallet_id, policy_id);
    Ok(())
}
