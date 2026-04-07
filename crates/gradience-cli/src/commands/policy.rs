use crate::context::AppContext;
use anyhow::Result;
use std::fs;

pub async fn set(ctx: &AppContext, wallet_id: String, file: String) -> Result<()> {
    let content = fs::read_to_string(&file)?;
    let _policy: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| anyhow::anyhow!("Invalid policy JSON: {}", e))?;

    let wallet = gradience_db::queries::get_wallet_by_id(&ctx.db, &wallet_id).await?;
    if wallet.is_none() {
        anyhow::bail!("Wallet not found: {}", wallet_id);
    }

    gradience_db::queries::create_policy(
        &ctx.db,
        &uuid::Uuid::new_v4().to_string(),
        "cli-policy",
        Some(&wallet_id),
        None,
        &content,
        1,
    ).await?;

    println!("Policy set for wallet {}", wallet_id);
    Ok(())
}
