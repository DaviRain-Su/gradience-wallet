use crate::context::AppContext;
use anyhow::Result;
use gradience_core::ows::adapter::OwsAdapter;

pub async fn create(ctx: &AppContext, wallet_id: String, name: String) -> Result<()> {
    if name.trim().is_empty() {
        anyhow::bail!("API key name cannot be empty");
    }

    let wallet = gradience_db::queries::get_wallet_by_id(&ctx.db, &wallet_id).await?;
    if wallet.is_none() {
        anyhow::bail!("Wallet not found: {}", wallet_id);
    }

    let passphrase = ctx.read_passphrase()
        .ok_or_else(|| anyhow::anyhow!("No session found. Run 'gradience auth login' first."))?;
    let vault = ctx.ows.init_vault(&passphrase).await?;

    let key = ctx.ows.attach_api_key_and_policies(&vault, &wallet_id, &name, vec![]).await?;

    // Store in Gradience DB
    let key_hash = hex::decode(&key.token_hash).unwrap_or_default();
    gradience_db::queries::create_api_key(
        &ctx.db,
        &key.id,
        &wallet_id,
        &name,
        &key_hash,
        "sign,read",
        None,
    ).await?;

    println!("Created API key '{}' (id: {})", key.name, key.id);
    if let Some(token) = key.raw_token {
        println!("Raw token (show once): {}", token);
    }
    Ok(())
}

pub async fn revoke(ctx: &AppContext, key_id: String) -> Result<()> {
    gradience_db::queries::revoke_api_key(&ctx.db, &key_id).await?;

    // Also revoke in OWS vault if session exists
    if let Some(passphrase) = ctx.read_passphrase() {
        if let Ok(vault) = ctx.ows.init_vault(&passphrase).await {
            let _ = ctx.ows.revoke_api_key(&vault, &key_id).await;
        }
    }

    println!("Revoked API key {}", key_id);
    Ok(())
}

pub async fn list(ctx: &AppContext, wallet_id: String) -> Result<()> {
    let keys = gradience_db::queries::list_api_keys_by_wallet(&ctx.db, &wallet_id).await?;
    if keys.is_empty() {
        println!("No API keys for wallet {}", wallet_id);
        return Ok(());
    }
    println!("API keys for wallet {}:", wallet_id);
    for k in keys {
        let status = if k.expires_at.is_some() { "revoked" } else { "active" };
        println!("  {} - {} [{}]", k.id, k.name, status);
    }
    Ok(())
}
